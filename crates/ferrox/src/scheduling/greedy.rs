use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use std::time::Instant;
use tracing::warn;

use super::problem::{SchedulingPlan, SchedulingRequest, TaskAssignment};

pub(super) const REQUEST_PREFIX: &str = "scheduling-request:";
const PLAN_PREFIX: &str = "scheduling-plan-greedy:";

/// Schedules tasks via Earliest-Deadline-First + earliest-available skilled agent.
///
/// **Algorithm:** O(n·m·log n) where n = tasks, m = agents.
///
/// **When to use:** latency-sensitive pipelines where a good schedule is needed
/// in microseconds. Use alongside [`CpSatSchedulerSuggestor`] in a Formation;
/// the Engine will emit both plans and the highest-confidence one (CP-SAT optimal)
/// will be accepted for execution.
///
/// **Confidence:** capped at 0.65 — greedy cannot prove optimality.
///
/// [`CpSatSchedulerSuggestor`]: super::cpsat::CpSatSchedulerSuggestor
pub struct GreedySchedulerSuggestor;

#[async_trait]
impl Suggestor for GreedySchedulerSuggestor {
    fn name(&self) -> &str {
        "GreedySchedulerSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(n·m·log n) — EDF + earliest-available; deterministic, sub-ms for n ≤ 10 000 tasks")
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

            match serde_json::from_str::<SchedulingRequest>(fact.content()) {
                Ok(req) => {
                    let plan = solve_greedy(&req);
                    // Greedy is fast but can't prove optimality — cap confidence at 0.65.
                    let confidence = (plan.throughput_ratio() * 0.65).min(0.65);
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
                    warn!(id = %fact.id(), error = %e, "malformed scheduling-request");
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

/// Pure EDF + earliest-available scheduling.  No OR-Tools dependency.
pub fn solve_greedy(req: &SchedulingRequest) -> SchedulingPlan {
    let t0 = Instant::now();

    // Sort tasks by earliest deadline first.
    let mut ordered: Vec<_> = req.tasks.iter().collect();
    ordered.sort_by_key(|t| t.deadline_min);

    let mut next_free = vec![0i64; req.agents.len()];
    let mut assignments: Vec<TaskAssignment> = Vec::new();

    for task in &ordered {
        // Find capable agent whose earliest available time after release is smallest.
        let best = req
            .agents
            .iter()
            .filter(|a| a.capabilities.contains(&task.required_capability))
            .min_by_key(|a| next_free[a.id].max(task.release_min));

        if let Some(agent) = best {
            let start = next_free[agent.id].max(task.release_min);
            let end = start + task.duration_min;
            if end <= task.deadline_min {
                next_free[agent.id] = end;
                assignments.push(TaskAssignment {
                    task_id: task.id,
                    task_name: task.name.clone(),
                    agent_id: agent.id,
                    agent_name: agent.name.clone(),
                    start_min: start,
                    end_min: end,
                });
            }
        }
    }

    assignments.sort_by_key(|a| a.start_min);
    let makespan = assignments.iter().map(|a| a.end_min).max().unwrap_or(0);
    let scheduled = assignments.len();

    SchedulingPlan {
        request_id: req.id.clone(),
        assignments,
        tasks_total: req.tasks.len(),
        tasks_scheduled: scheduled,
        makespan_min: makespan,
        solver: "greedy-edf".to_string(),
        status: "feasible".to_string(),
        wall_time_seconds: t0.elapsed().as_secs_f64(),
    }
}
