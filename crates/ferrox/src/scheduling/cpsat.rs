use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use ferrox_ortools_sys::OrtoolsStatus;
use ferrox_ortools_sys::safe::CpModel;
use std::collections::HashMap;
use std::time::Instant;
use tracing::warn;

use super::problem::{SchedulingPlan, SchedulingRequest, SchedulingTask, TaskAssignment};

const PLAN_PREFIX: &str = "scheduling-plan-cpsat:";

use super::greedy::REQUEST_PREFIX;

/// Schedules tasks to optimality using CP-SAT optional-interval variables and
/// per-agent `NoOverlap` constraints.
///
/// **Algorithm:** CP-SAT (DPLL + LNS + clause learning).  Explores the full
/// combinatorial space; guarantees optimality within the time budget.
///
/// **When to use:** batch planning, pre-flight checks, or any context where
/// schedule quality matters more than latency.  Pair with
/// [`GreedySchedulerSuggestor`] in a Formation: greedy provides an immediate
/// warm-start answer; CP-SAT proves (or improves) it before execution begins.
///
/// **Confidence:**
/// - `optimal` status + 100% throughput → 1.0
/// - `optimal` status + partial throughput → throughput ratio (resource-limited)
/// - `feasible` status → throughput_ratio × 0.85 (time budget exhausted)
/// - `infeasible` → 0.0
///
/// [`GreedySchedulerSuggestor`]: super::greedy::GreedySchedulerSuggestor
pub struct CpSatSchedulerSuggestor;

#[async_trait]
impl Suggestor for CpSatSchedulerSuggestor {
    fn name(&self) -> &str {
        "CpSatSchedulerSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some(concat!(
            "NP-hard in general; CP-SAT DPLL+LNS with optional-interval NoOverlap; ",
            "proves optimality for n ≤ 100 tasks within 30 s on 10-core hardware"
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

            match serde_json::from_str::<SchedulingRequest>(&fact.content) {
                Ok(req) => {
                    let plan = solve_cpsat(&req);
                    let confidence = match plan.status.as_str() {
                        "optimal" => plan.throughput_ratio(),
                        "feasible" => plan.throughput_ratio() * 0.85,
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
                    warn!(id = %fact.id, error = %e, "malformed scheduling-request");
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

/// Maximise tasks scheduled subject to agent-capacity (NoOverlap) and
/// time-window constraints.  Returns a [`SchedulingPlan`] with full assignments.
pub fn solve_cpsat(req: &SchedulingRequest) -> SchedulingPlan {
    let t0 = Instant::now();

    let mut model = CpModel::new();

    // Index helpers
    let mut name_to_idx: HashMap<String, i32> = HashMap::new();
    let mut bool_name_to_idx: HashMap<String, i32> = HashMap::new();
    let mut interval_name_to_idx: HashMap<String, i32> = HashMap::new();

    // ── Per-task start / end variables ────────────────────────────────────────

    for task in &req.tasks {
        let s_ub = (task.deadline_min - task.duration_min).max(task.release_min);
        let e_lb = task.release_min + task.duration_min;

        let s = model.new_int_var(task.release_min, s_ub, &start_name(task));
        let e = model.new_int_var(e_lb, task.deadline_min, &end_name(task));

        name_to_idx.insert(start_name(task), s);
        name_to_idx.insert(end_name(task), e);
    }

    // ── Per-(task, agent) assignment booleans + optional intervals ────────────

    // agent_id → list of optional interval var indices for that agent
    let mut agent_interval_idxs: Vec<Vec<i32>> = vec![Vec::new(); req.agents.len()];
    // task_index → list of x variable names for that task
    let mut task_assign_names: Vec<Vec<String>> = vec![Vec::new(); req.tasks.len()];

    for (ti, task) in req.tasks.iter().enumerate() {
        for agent in req
            .agents
            .iter()
            .filter(|a| a.capabilities.contains(&task.required_capability))
        {
            let x_name = x_var_name(task.id, agent.id);
            let ov_name = ov_var_name(task.id, agent.id);

            let x_idx = model.new_bool_var(&x_name);
            bool_name_to_idx.insert(x_name.clone(), x_idx);
            name_to_idx.insert(x_name.clone(), x_idx);

            let s_idx = name_to_idx[&start_name(task)];
            let e_idx = name_to_idx[&end_name(task)];

            let ov_idx =
                model.new_optional_interval_var(s_idx, task.duration_min, e_idx, x_idx, &ov_name);
            interval_name_to_idx.insert(ov_name, ov_idx);

            agent_interval_idxs[agent.id].push(ov_idx);
            task_assign_names[ti].push(x_name);
        }
    }

    // ── Constraints ───────────────────────────────────────────────────────────

    // 1. Each task assigned to at most one capable agent (optional scheduling).
    for names in &task_assign_names {
        if names.len() > 1 {
            let vars: Vec<i32> = names.iter().map(|n| name_to_idx[n]).collect();
            let ones = vec![1i64; vars.len()];
            model.add_linear_le(&vars, &ones, 1);
        }
    }

    // 2. No two tasks overlap on the same agent.
    for agent_ivs in &agent_interval_idxs {
        if agent_ivs.len() > 1 {
            model.add_no_overlap(agent_ivs);
        }
    }

    // ── Objective: maximise total tasks scheduled ─────────────────────────────

    let obj_vars: Vec<i32> = bool_name_to_idx.values().copied().collect();
    let obj_coeffs = vec![1i64; obj_vars.len()];
    model.maximize(&obj_vars, &obj_coeffs);

    let solution = model.solve(req.time_limit_seconds);
    let elapsed = t0.elapsed().as_secs_f64();

    let status = match solution.status() {
        OrtoolsStatus::Optimal => "optimal",
        OrtoolsStatus::Feasible => "feasible",
        OrtoolsStatus::Infeasible => "infeasible",
        OrtoolsStatus::Unbounded => "unbounded",
        _ => "error",
    };

    if !solution.status().is_success() {
        return SchedulingPlan {
            request_id: req.id.clone(),
            assignments: Vec::new(),
            tasks_total: req.tasks.len(),
            tasks_scheduled: 0,
            makespan_min: 0,
            solver: "cp-sat-v9.15".to_string(),
            status: status.to_string(),
            wall_time_seconds: elapsed,
        };
    }

    // ── Extract assignments ───────────────────────────────────────────────────

    let mut assignments: Vec<TaskAssignment> = Vec::new();

    for task in &req.tasks {
        for agent in req
            .agents
            .iter()
            .filter(|a| a.capabilities.contains(&task.required_capability))
        {
            let x_name = x_var_name(task.id, agent.id);
            if let Some(&x_idx) = bool_name_to_idx.get(&x_name) {
                if solution.value(x_idx) == 1 {
                    let s_idx = name_to_idx[&start_name(task)];
                    let start = solution.value(s_idx);
                    assignments.push(TaskAssignment {
                        task_id: task.id,
                        task_name: task.name.clone(),
                        agent_id: agent.id,
                        agent_name: agent.name.clone(),
                        start_min: start,
                        end_min: start + task.duration_min,
                    });
                    break;
                }
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
        solver: "cp-sat-v9.15".to_string(),
        status: status.to_string(),
        wall_time_seconds: elapsed,
    }
}

// ── Variable name helpers ─────────────────────────────────────────────────────

fn start_name(task: &SchedulingTask) -> String {
    format!("s_{}", task.id)
}
fn end_name(task: &SchedulingTask) -> String {
    format!("e_{}", task.id)
}
fn x_var_name(task_id: usize, agent_id: usize) -> String {
    format!("x_{task_id}_{agent_id}")
}
fn ov_var_name(task_id: usize, agent_id: usize) -> String {
    format!("ov_{task_id}_{agent_id}")
}
