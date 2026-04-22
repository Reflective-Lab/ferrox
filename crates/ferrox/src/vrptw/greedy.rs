use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use std::time::Instant;
use tracing::warn;

use super::problem::{RouteStop, VrptwPlan, VrptwRequest};

pub(super) const REQUEST_PREFIX: &str = "vrptw-request:";
const PLAN_PREFIX: &str = "vrptw-plan-greedy:";

/// Routes a vehicle via Nearest-Neighbour heuristic with time-window feasibility check.
///
/// **Algorithm:** O(n²) — at each stop, visit the nearest unvisited customer
/// whose time window is still reachable.  Backtracks to depot when no further
/// customer is reachable.
///
/// **Confidence:** capped at 0.60 — nearest-neighbour can be 20-25% worse
/// than optimal on real instances.
pub struct NearestNeighborSuggestor;

#[async_trait]
impl Suggestor for NearestNeighborSuggestor {
    fn name(&self) -> &str {
        "NearestNeighborSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(n²) — nearest-neighbour with TW feasibility; deterministic, sub-ms for n ≤ 10 000")
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id.starts_with(REQUEST_PREFIX) && !own_plan_exists(ctx, request_id(&f.id)))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for fact in ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.id.starts_with(REQUEST_PREFIX))
        {
            let rid = request_id(&fact.id);
            if own_plan_exists(ctx, rid) {
                continue;
            }

            match serde_json::from_str::<VrptwRequest>(&fact.content) {
                Ok(req) => {
                    let plan = solve_nn(&req);
                    let confidence = (plan.visit_ratio() * 0.60).min(0.60);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{PLAN_PREFIX}{rid}"),
                            serde_json::to_string(&plan).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(confidence),
                    );
                }
                Err(e) => {
                    warn!(id = %fact.id, error = %e, "malformed vrptw-request");
                }
            }
        }

        if proposals.is_empty() {
            AgentEffect::empty()
        } else {
            AgentEffect::with_proposals(proposals)
        }
    }
}

fn request_id(fact_id: &str) -> &str {
    fact_id.trim_start_matches(REQUEST_PREFIX)
}

fn own_plan_exists(ctx: &dyn Context, request_id: &str) -> bool {
    let plan_id = format!("{PLAN_PREFIX}{request_id}");
    ctx.get(ContextKey::Strategies)
        .iter()
        .any(|f| f.id == plan_id)
}

/// Nearest-neighbour TSP with time-window feasibility.
pub fn solve_nn(req: &VrptwRequest) -> VrptwPlan {
    let t0 = Instant::now();
    let n = req.customers.len();
    let mut visited = vec![false; n];
    let mut route: Vec<RouteStop> = Vec::new();

    let mut cur_x = req.depot.x;
    let mut cur_y = req.depot.y;
    let mut cur_t = req.depot.ready_time;
    let mut total_dist = 0.0_f64;

    loop {
        // Find nearest reachable unvisited customer.
        let best = (0..n)
            .filter(|&i| !visited[i])
            .filter_map(|i| {
                let c = &req.customers[i];
                let dx = cur_x - c.x;
                let dy = cur_y - c.y;
                let dist = (dx * dx + dy * dy).sqrt();
                #[allow(clippy::cast_possible_truncation)]
                let travel = dist.ceil() as i64;
                let arrival = cur_t + travel;
                // Must arrive before window closes, and vehicle returns in time.
                let depart = arrival.max(c.window_open) + c.service_time;
                let depot_dx = c.x - req.depot.x;
                let depot_dy = c.y - req.depot.y;
                #[allow(clippy::cast_possible_truncation)]
                let return_dist = ((depot_dx * depot_dx + depot_dy * depot_dy).sqrt().ceil()) as i64;
                if arrival <= c.window_close && depart + return_dist <= req.depot.due_time {
                    Some((i, dist, travel, arrival, depart))
                } else {
                    None
                }
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        match best {
            Some((i, dist, travel, arrival, depart)) => {
                let c = &req.customers[i];
                visited[i] = true;
                total_dist += dist;
                cur_x = c.x;
                cur_y = c.y;
                cur_t = depart;
                let _ = travel;
                let _ = arrival;
                route.push(RouteStop {
                    customer_id: c.id,
                    customer_name: c.name.clone(),
                    arrival: depart - c.service_time,
                    departure: depart,
                });
            }
            None => break,
        }
    }

    // Return to depot.
    let depot_dx = cur_x - req.depot.x;
    let depot_dy = cur_y - req.depot.y;
    let return_dist = (depot_dx * depot_dx + depot_dy * depot_dy).sqrt();
    total_dist += return_dist;
    #[allow(clippy::cast_possible_truncation)]
    let return_time = cur_t + return_dist.ceil() as i64;

    VrptwPlan {
        request_id: req.id.clone(),
        customers_visited: route.len(),
        customers_total: n,
        route,
        total_distance: total_dist,
        return_time,
        solver: "nearest-neighbour".to_string(),
        status: "feasible".to_string(),
        wall_time_seconds: t0.elapsed().as_secs_f64(),
    }
}
