use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use ferrox_ortools_sys::OrtoolsStatus;
use ferrox_ortools_sys::safe::CpModel;
use std::time::Instant;
use tracing::warn;

use super::greedy::REQUEST_PREFIX;
use super::problem::{JobShopPlan, JobShopRequest, ScheduledOp};

const PLAN_PREFIX: &str = "jspbench-plan-cpsat:";

/// Solves Job Shop Scheduling to optimality using CP-SAT interval variables.
///
/// **Model:**
/// - One fixed-duration interval per operation (start + duration = end enforced).
/// - `AddNoOverlap` per machine: no two operations on the same machine overlap.
/// - `LinearGe` precedence within each job: op k+1 starts after op k ends.
/// - Makespan variable `C_max`: minimised; bounded by the latest operation end.
///
/// **Confidence:**
/// - `optimal` → 1.0 (proven minimum makespan)
/// - `feasible` → 0.85 (time budget exhausted, solution may improve)
///
/// **Formation role:** CP-SAT is the quality anchor; [`GreedyJobShopSuggestor`]
/// provides an instant warm-start that CP-SAT verifies or beats.
///
/// [`GreedyJobShopSuggestor`]: super::greedy::GreedyJobShopSuggestor
pub struct CpSatJobShopSuggestor;

#[async_trait]
impl Suggestor for CpSatJobShopSuggestor {
    fn name(&self) -> &str {
        "CpSatJobShopSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some(concat!(
            "NP-hard; CP-SAT interval-NoOverlap formulation; ",
            "proves optimality for 15×10 instances in < 5 s on 10-core hardware"
        ))
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
                    let plan = solve_cpsat_jsp(&req);
                    let confidence = match plan.status.as_str() {
                        "optimal" => 1.0,
                        "feasible" => 0.85,
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

// ── Solver ────────────────────────────────────────────────────────────────────

pub fn solve_cpsat_jsp(req: &JobShopRequest) -> JobShopPlan {
    let t0 = Instant::now();
    let horizon = req.horizon();
    let mut model = CpModel::new();

    // start[j][k], end[j][k], interval[j][k]
    let mut starts: Vec<Vec<i32>> = Vec::new();
    let mut ends: Vec<Vec<i32>> = Vec::new();
    let mut intervals: Vec<Vec<i32>> = Vec::new();

    for job in &req.jobs {
        let mut js = Vec::new();
        let mut je = Vec::new();
        let mut ji = Vec::new();
        for (k, op) in job.operations.iter().enumerate() {
            let s = model.new_int_var(0, horizon, &format!("s_{}_{k}", job.id));
            let e = model.new_int_var(op.duration, horizon, &format!("e_{}_{k}", job.id));
            let iv = model.new_fixed_interval_var(s, op.duration, e, &format!("iv_{}_{k}", job.id));
            js.push(s);
            je.push(e);
            ji.push(iv);
        }
        starts.push(js);
        ends.push(je);
        intervals.push(ji);
    }

    // Makespan variable.
    let cmax = model.new_int_var(0, horizon, "cmax");

    // ── Per-job precedence constraints ────────────────────────────────────────
    // end[j][k] <= start[j][k+1]  →  start[j][k+1] - end[j][k] >= 0
    for (j, job) in req.jobs.iter().enumerate() {
        for k in 0..job.operations.len().saturating_sub(1) {
            model.add_linear_ge(
                &[starts[j][k + 1], ends[j][k]],
                &[1, -1],
                0,
            );
        }
    }

    // ── Per-machine NoOverlap ─────────────────────────────────────────────────
    let mut machine_intervals: Vec<Vec<i32>> = vec![Vec::new(); req.num_machines];
    for (j, job) in req.jobs.iter().enumerate() {
        for (k, op) in job.operations.iter().enumerate() {
            machine_intervals[op.machine_id].push(intervals[j][k]);
        }
    }
    for ivs in &machine_intervals {
        if ivs.len() > 1 {
            model.add_no_overlap(ivs);
        }
    }

    // ── Makespan: cmax >= end[j][last] for each job ───────────────────────────
    for (j, job) in req.jobs.iter().enumerate() {
        let last = job.operations.len() - 1;
        model.add_linear_ge(&[cmax, ends[j][last]], &[1, -1], 0);
    }

    // ── Minimise makespan ─────────────────────────────────────────────────────
    model.minimize(&[cmax], &[1]);

    let solution = model.solve(req.time_limit_seconds);
    let elapsed = t0.elapsed().as_secs_f64();

    let status = match solution.status() {
        OrtoolsStatus::Optimal => "optimal",
        OrtoolsStatus::Feasible => "feasible",
        OrtoolsStatus::Infeasible => "infeasible",
        _ => "error",
    };

    if !solution.status().is_success() {
        return JobShopPlan {
            request_id: req.id.clone(),
            schedule: Vec::new(),
            makespan: 0,
            lower_bound: None,
            solver: "cp-sat-v9.15".to_string(),
            status: status.to_string(),
            wall_time_seconds: elapsed,
        };
    }

    let makespan = solution.value(cmax);

    let mut schedule: Vec<ScheduledOp> = Vec::new();
    for (j, job) in req.jobs.iter().enumerate() {
        for (k, op) in job.operations.iter().enumerate() {
            let start = solution.value(starts[j][k]);
            schedule.push(ScheduledOp {
                job_id: job.id,
                job_name: job.name.clone(),
                machine_id: op.machine_id,
                op_index: k,
                start,
                end: start + op.duration,
            });
        }
    }
    schedule.sort_by_key(|s| (s.machine_id, s.start));

    let lower_bound = if status == "optimal" { Some(makespan) } else { None };

    JobShopPlan {
        request_id: req.id.clone(),
        schedule,
        makespan,
        lower_bound,
        solver: "cp-sat-v9.15".to_string(),
        status: status.to_string(),
        wall_time_seconds: elapsed,
    }
}
