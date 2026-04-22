use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use std::time::Instant;
use tracing::warn;

use super::problem::{JobShopPlan, JobShopRequest, ScheduledOp};

pub(super) const REQUEST_PREFIX: &str = "jspbench-request:";
const PLAN_PREFIX: &str = "jspbench-plan-greedy:";

/// Schedules a Job Shop instance via List Scheduling with Shortest Processing Time
/// (SPT) dispatching.
///
/// **Algorithm:** O(n·m²) where n = jobs, m = machines.  At each scheduling
/// event (a machine becomes free), all operations waiting for that machine are
/// ranked by duration; the shortest is dispatched first.
///
/// **Confidence:** capped at 0.60 — SPT can be arbitrarily bad vs. optimal in
/// the worst case.  Use alongside [`CpSatJobShopSuggestor`] in a Formation.
///
/// [`CpSatJobShopSuggestor`]: super::cpsat::CpSatJobShopSuggestor
pub struct GreedyJobShopSuggestor;

#[async_trait]
impl Suggestor for GreedyJobShopSuggestor {
    fn name(&self) -> &str {
        "GreedyJobShopSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(n·m²) — SPT list scheduling; sub-ms for n ≤ 1 000 jobs × m ≤ 100 machines")
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

            match serde_json::from_str::<JobShopRequest>(&fact.content) {
                Ok(req) => {
                    let plan = solve_greedy(&req);
                    // SPT dispatching: bounded confidence.
                    let confidence = 0.55_f64;
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
                    warn!(id = %fact.id, error = %e, "malformed jspbench-request");
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

/// SPT list scheduling: repeatedly dispatch the shortest ready operation per machine.
pub fn solve_greedy(req: &JobShopRequest) -> JobShopPlan {
    let t0 = Instant::now();
    let n = req.jobs.len();
    let m = req.num_machines;

    // next_op[j] = index of the next operation to schedule for job j
    let mut next_op = vec![0usize; n];
    // job_free[j]  = earliest time job j's next op can start (after predecessor)
    let mut job_free = vec![0i64; n];
    // machine_free[m] = earliest time machine m is free
    let mut machine_free = vec![0i64; m];

    let mut schedule: Vec<ScheduledOp> = Vec::new();
    let total_ops: usize = req.jobs.iter().map(|j| j.operations.len()).sum();

    while schedule.len() < total_ops {
        // For each machine, find all operations currently ready for it.
        // "Ready" = job's predecessor op is done AND this op's machine matches.
        let mut dispatched_any = false;

        for mach in 0..m {
            // Collect candidates: ready operations for this machine.
            let mut candidates: Vec<(usize, usize, i64, i64)> = Vec::new(); // (job, op_idx, earliest_start, duration)
            for (j, job) in req.jobs.iter().enumerate() {
                let k = next_op[j];
                if k >= job.operations.len() {
                    continue;
                }
                let op = &job.operations[k];
                if op.machine_id != mach {
                    continue;
                }
                let earliest = job_free[j].max(machine_free[mach]);
                candidates.push((j, k, earliest, op.duration));
            }

            if candidates.is_empty() {
                continue;
            }

            // SPT: choose the candidate with shortest duration; break ties by job id.
            candidates.sort_by_key(|&(j, _, _, dur)| (dur, j));
            let (j, k, earliest, dur) = candidates[0];

            let start = earliest;
            let end = start + dur;
            machine_free[mach] = end;
            job_free[j] = end;
            next_op[j] = k + 1;

            schedule.push(ScheduledOp {
                job_id: j,
                job_name: req.jobs[j].name.clone(),
                machine_id: mach,
                op_index: k,
                start,
                end,
            });
            dispatched_any = true;
        }

        if !dispatched_any {
            // Advance time to the next machine-free event.
            let next_t = machine_free.iter().copied().filter(|&t| {
                // Only machines that still have pending ops.
                (0..n).any(|j| {
                    let k = next_op[j];
                    k < req.jobs[j].operations.len()
                        && req.jobs[j].operations[k].machine_id
                            == machine_free
                                .iter()
                                .position(|&f| f == t)
                                .unwrap_or(usize::MAX)
                })
            }).min();

            if next_t.is_none() {
                // Deadlock guard: just advance the global minimum.
                let min_t = machine_free.iter().copied().min().unwrap_or(0);
                for mf in &mut machine_free {
                    if *mf == min_t {
                        *mf = 0; // reset to allow re-dispatch
                    }
                }
            }
        }
    }

    schedule.sort_by_key(|s| (s.machine_id, s.start));
    let makespan = schedule.iter().map(|s| s.end).max().unwrap_or(0);

    JobShopPlan {
        request_id: req.id.clone(),
        schedule,
        makespan,
        lower_bound: None,
        solver: "greedy-spt".to_string(),
        status: "feasible".to_string(),
        wall_time_seconds: t0.elapsed().as_secs_f64(),
    }
}
