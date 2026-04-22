use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use ferrox_highs_sys::HighsModelStatus;
use ferrox_highs_sys::safe::HighsSolver;
use std::collections::HashMap;
use tracing::warn;

use super::problem::{MipPlan, MipRequest, VarKind};

const REQUEST_PREFIX: &str = "mip-request:";
const PLAN_PREFIX: &str = "mip-plan:";

pub struct HighsMipSuggestor;

#[async_trait]
impl Suggestor for HighsMipSuggestor {
    fn name(&self) -> &'static str {
        "HighsMipSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("MIP branch-and-cut via HiGHS v1.7; NP-hard in general")
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id.starts_with(REQUEST_PREFIX) && !plan_exists(ctx, request_id(&f.id)))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for fact in ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.id.starts_with(REQUEST_PREFIX))
        {
            let rid = request_id(&fact.id);
            if plan_exists(ctx, rid) {
                continue;
            }

            match serde_json::from_str::<MipRequest>(&fact.content) {
                Ok(req) => {
                    let plan = solve_mip(&req);
                    let confidence = match plan.status.as_str() {
                        "optimal" => 1.0,
                        "feasible" => 0.6 + (1.0 - plan.mip_gap.min(1.0)) * 0.3,
                        _ => 0.0,
                    };
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{PLAN_PREFIX}{}", plan.request_id),
                            serde_json::to_string(&plan).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(confidence),
                    );
                }
                Err(e) => {
                    warn!(id = %fact.id, error = %e, "malformed mip-request");
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

fn plan_exists(ctx: &dyn Context, request_id: &str) -> bool {
    let plan_id = format!("{PLAN_PREFIX}{request_id}");
    ctx.get(ContextKey::Strategies)
        .iter()
        .any(|f| f.id == plan_id)
}

pub fn solve_mip(req: &MipRequest) -> MipPlan {
    let mut solver = HighsSolver::new();

    // HiGHS cost sign: we pass costs directly; HiGHS minimizes by default.
    // For maximize, negate all cost coefficients.
    let sign = if req.objective.maximize { -1.0 } else { 1.0 };

    // Build a cost vector indexed by variable position
    let mut costs = vec![0.0f64; req.variables.len()];
    let name_to_pos: HashMap<&str, usize> = req
        .variables
        .iter()
        .enumerate()
        .map(|(i, v)| (v.name.as_str(), i))
        .collect();

    for term in &req.objective.terms {
        if let Some(&pos) = name_to_pos.get(term.var.as_str()) {
            costs[pos] = term.coeff * sign;
        }
    }

    // Add columns
    let col_indices: Vec<i32> = req
        .variables
        .iter()
        .enumerate()
        .map(|(i, var)| match var.kind {
            VarKind::Continuous => solver.add_col(costs[i], var.lb, var.ub),
            VarKind::Integer => solver.add_int_col(costs[i], var.lb, var.ub),
            VarKind::Binary => solver.add_bin_col(costs[i]),
        })
        .collect();

    if let Some(tl) = req.time_limit_seconds {
        solver.set_time_limit(tl);
    }
    if let Some(gap) = req.mip_gap_tolerance {
        solver.set_mip_rel_gap(gap);
    }

    // Add rows
    for con in &req.constraints {
        let mut indices = Vec::new();
        let mut vals = Vec::new();
        for term in &con.terms {
            if let Some(&pos) = name_to_pos.get(term.var.as_str()) {
                indices.push(col_indices[pos]);
                vals.push(term.coeff);
            }
        }
        solver.add_row(con.lb, con.ub, &indices, &vals);
    }

    let status = solver.run();

    let status_str = match status {
        HighsModelStatus::Optimal => "optimal",
        HighsModelStatus::SolutionLimit | HighsModelStatus::TimeLimit => "feasible",
        HighsModelStatus::Infeasible => "infeasible",
        HighsModelStatus::Unbounded => "unbounded",
        _ => "error",
    };

    let (values, objective_value, mip_gap) = if status.is_success() {
        let vals: Vec<(String, f64)> = req
            .variables
            .iter()
            .enumerate()
            .map(|(i, v)| (v.name.clone(), solver.col_value(col_indices[i])))
            .collect();
        let obj = solver.objective_value() * sign;
        let gap = solver.mip_gap();
        (vals, obj, gap)
    } else {
        (vec![], 0.0, f64::INFINITY)
    };

    MipPlan {
        request_id: req.id.clone(),
        status: status_str.to_string(),
        values,
        objective_value,
        mip_gap,
        solver: "highs-v1.14.0".to_string(),
    }
}
