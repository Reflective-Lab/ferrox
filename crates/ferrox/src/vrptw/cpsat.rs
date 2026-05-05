use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use ferrox_ortools_sys::OrtoolsStatus;
use ferrox_ortools_sys::safe::CpModel;
use std::time::Instant;
use tracing::warn;

use super::greedy::REQUEST_PREFIX;
use super::problem::{Customer, RouteStop, VrptwPlan, VrptwRequest};

const PLAN_PREFIX: &str = "vrptw-plan-cpsat:";
/// Distance scale factor: 1 unit = 0.01 distance units.
const SCALE: i64 = 100;

/// Solves TSPTW to optimality using CP-SAT `AddCircuit` + time-window variables.
///
/// **Model:**
/// Nodes 0 = depot, 1..=N = customers.
///
/// ```text
/// x_ij ∈ {0,1} — arc i→j is used in the route
/// x_ii ∈ {0,1} — self-loop: customer i is skipped (optional visits)
///
/// AddCircuit({all arcs including self-loops})
///
/// t_i ∈ [window_open_i, window_close_i]  — arrival time at i
///
/// For each arc (i,j), i≠j:
///   t_j ≥ t_i + service_i + travel_ij − M·(1 − x_ij)
///   → LinearGe: t_j − t_i − M·x_ij ≥ service_i + travel_ij − M
///
/// Objective: maximise customers visited = minimise Σ x_ii
/// ```
///
/// **Confidence:**
/// - `optimal` → visit_ratio (resource-limited if < 1.0, otherwise proven max throughput)
/// - `feasible` → visit_ratio × 0.85
pub struct CpSatVrptwSuggestor;

#[async_trait]
impl Suggestor for CpSatVrptwSuggestor {
    fn name(&self) -> &str {
        "CpSatVrptwSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some(concat!(
            "NP-hard; CP-SAT AddCircuit + time-window propagation; ",
            "proves optimality for n ≤ 25 customers within 30 s on 10-core hardware"
        ))
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id().starts_with(REQUEST_PREFIX) && !own_plan_exists(ctx, request_id(f.id()))
        })
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for fact in ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.id().starts_with(REQUEST_PREFIX))
        {
            let rid = request_id(fact.id());
            if own_plan_exists(ctx, rid) {
                continue;
            }

            match serde_json::from_str::<VrptwRequest>(fact.content()) {
                Ok(req) => {
                    let plan = solve_cpsat_vrptw(&req);
                    let confidence = match plan.status.as_str() {
                        "optimal" => plan.visit_ratio(),
                        "feasible" => plan.visit_ratio() * 0.85,
                        _ => 0.0,
                    };
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
                    warn!(id = %fact.id(), error = %e, "malformed vrptw-request");
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
        .any(|f| f.id() == plan_id.as_str())
}

// ── Solver ────────────────────────────────────────────────────────────────────

pub fn solve_cpsat_vrptw(req: &VrptwRequest) -> VrptwPlan {
    let t0 = Instant::now();
    let n = req.customers.len();
    // Node 0 = depot, 1..=n = customers.
    let num_nodes = n + 1;

    // Scaled integer travel times.
    #[allow(clippy::cast_possible_truncation)]
    let travel = |from_x: f64, from_y: f64, to_x: f64, to_y: f64| -> i64 {
        let dx = from_x - to_x;
        let dy = from_y - to_y;
        ((dx * dx + dy * dy).sqrt() * SCALE as f64).ceil() as i64
    };

    let depot_travel_to = |c: &Customer| -> i64 { travel(req.depot.x, req.depot.y, c.x, c.y) };
    let customer_travel = |a: &Customer, b: &Customer| -> i64 { travel(a.x, a.y, b.x, b.y) };

    let horizon = req.depot.due_time * SCALE;
    let big_m = horizon + 1;

    let mut model = CpModel::new();

    // ── Arc variables ─────────────────────────────────────────────────────────

    // arc_lit[i][j] = bool literal for arc from node i to node j.
    let mut arc_lit: Vec<Vec<i32>> = vec![vec![-1; num_nodes]; num_nodes];

    for i in 0..num_nodes {
        for j in 0..num_nodes {
            if i == j && i == 0 {
                continue; // no depot self-loop
            }
            arc_lit[i][j] = model.new_bool_var(&format!("x_{i}_{j}"));
        }
    }

    // ── Time variables ────────────────────────────────────────────────────────
    // Scaled by SCALE; depot time vars for start and return.

    let depot_start_t = model.new_int_var(
        req.depot.ready_time * SCALE,
        req.depot.ready_time * SCALE,
        "t_depot_start",
    );
    let depot_end_t = model.new_int_var(0, req.depot.due_time * SCALE, "t_depot_end");

    // t[i] = scaled arrival time at customer i (1-indexed).
    let cust_t: Vec<i32> = req
        .customers
        .iter()
        .map(|c| {
            model.new_int_var(
                c.window_open * SCALE,
                c.window_close * SCALE,
                &format!("t_{}", c.id),
            )
        })
        .collect();

    // Helper: time var for node index.
    let t_node = |node: usize| -> i32 {
        if node == 0 {
            depot_start_t
        } else {
            cust_t[node - 1]
        }
    };

    // ── AddCircuit ────────────────────────────────────────────────────────────

    let mut tails: Vec<i32> = Vec::new();
    let mut heads: Vec<i32> = Vec::new();
    let mut lits: Vec<i32> = Vec::new();

    for i in 0..num_nodes {
        for j in 0..num_nodes {
            let lit = arc_lit[i][j];
            if lit == -1 {
                continue;
            }
            tails.push(i as i32);
            heads.push(j as i32);
            lits.push(lit);
        }
    }

    model.add_circuit(&tails, &heads, &lits);

    // ── Time-consistency constraints ──────────────────────────────────────────
    // For each non-self-loop arc (i→j): if x_ij = 1, t_j ≥ t_i + svc_i + travel_ij
    // Big-M: t_j - t_i - M*x_ij ≥ svc_i + travel_ij - M
    //        → LinearGe: [{t_j, 1}, {t_i, -1}, {x_ij, -M}] ≥ svc_i + travel_ij - M

    for i in 0..num_nodes {
        for j in 0..num_nodes {
            if i == j {
                continue;
            }
            let lit = arc_lit[i][j];
            if lit == -1 {
                continue;
            }

            let (svc_i, travel_ij) = if i == 0 {
                // depot → customer j
                let c = &req.customers[j - 1];
                (0i64, depot_travel_to(c))
            } else if j == 0 {
                // customer i → depot
                let c = &req.customers[i - 1];
                (c.service_time * SCALE, depot_travel_to(c))
            } else {
                // customer i → customer j
                let ci = &req.customers[i - 1];
                let cj = &req.customers[j - 1];
                (ci.service_time * SCALE, customer_travel(ci, cj))
            };

            let rhs = svc_i + travel_ij - big_m;
            let t_j = if j == 0 { depot_end_t } else { t_node(j) };
            let t_i = t_node(i);

            model.add_linear_ge(&[t_j, t_i, lit], &[1, -1, -big_m], rhs);
        }
    }

    // ── Objective: maximise customers visited = minimise Σ self-loop literals ─

    let self_loop_lits: Vec<i32> = (1..=n)
        .map(|i| arc_lit[i][i])
        .filter(|&l| l != -1)
        .collect();

    if !self_loop_lits.is_empty() {
        let coeffs = vec![1i64; self_loop_lits.len()];
        model.minimize(&self_loop_lits, &coeffs);
    }

    let solution = model.solve(req.time_limit_seconds);
    let elapsed = t0.elapsed().as_secs_f64();

    let status = match solution.status() {
        OrtoolsStatus::Optimal => "optimal",
        OrtoolsStatus::Feasible => "feasible",
        OrtoolsStatus::Infeasible => "infeasible",
        _ => "error",
    };

    if !solution.status().is_success() {
        return VrptwPlan {
            request_id: req.id.clone(),
            route: Vec::new(),
            customers_total: n,
            customers_visited: 0,
            total_distance: 0.0,
            return_time: 0,
            solver: "cp-sat-v9.15".to_string(),
            status: status.to_string(),
            wall_time_seconds: elapsed,
        };
    }

    // ── Extract route from arc literals ───────────────────────────────────────

    let mut route: Vec<RouteStop> = Vec::new();
    let mut total_distance = 0.0_f64;
    let mut cur = 0usize; // start at depot

    for _ in 0..=n {
        // Find the arc leaving cur that is active.
        let next = (0..num_nodes).find(|&j| {
            if j == cur {
                return false;
            }
            let lit = arc_lit[cur][j];
            lit != -1 && solution.value(lit) == 1
        });

        match next {
            None | Some(0) => break, // returned to depot
            Some(j) => {
                let c = &req.customers[j - 1];
                let arrival_scaled = solution.value(cust_t[j - 1]);
                #[allow(clippy::cast_precision_loss)]
                let arrival = arrival_scaled / SCALE;
                let departure = arrival + c.service_time;

                // Accumulate distance.
                let (fx, fy) = if cur == 0 {
                    (req.depot.x, req.depot.y)
                } else {
                    let pc = &req.customers[cur - 1];
                    (pc.x, pc.y)
                };
                let dx = fx - c.x;
                let dy = fy - c.y;
                total_distance += (dx * dx + dy * dy).sqrt();

                route.push(RouteStop {
                    customer_id: c.id,
                    customer_name: c.name.clone(),
                    arrival,
                    departure,
                });
                cur = j;
            }
        }
    }

    // Distance back to depot.
    if let Some(last_stop) = route.last() {
        if let Some(c) = req.customers.iter().find(|c| c.id == last_stop.customer_id) {
            let dx = c.x - req.depot.x;
            let dy = c.y - req.depot.y;
            total_distance += (dx * dx + dy * dy).sqrt();
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let return_time = solution.value(depot_end_t) / SCALE;

    VrptwPlan {
        request_id: req.id.clone(),
        customers_visited: route.len(),
        customers_total: n,
        route,
        total_distance,
        return_time,
        solver: "cp-sat-v9.15".to_string(),
        status: status.to_string(),
        wall_time_seconds: elapsed,
    }
}
