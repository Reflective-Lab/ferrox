use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use ferrox_ortools_sys::OrtoolsStatus;
use ferrox_ortools_sys::safe::LinearSolver;
use std::collections::HashMap;
use tracing::warn;

use super::problem::{LpPlan, LpRequest};

const REQUEST_PREFIX: &str = "glop-request:";
const PLAN_PREFIX: &str = "glop-plan:";

pub struct GlopLpSuggestor;

#[async_trait]
impl Suggestor for GlopLpSuggestor {
    fn name(&self) -> &'static str {
        "GlopLpSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("LP simplex; polynomial in practice; GLOP v9.11")
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

            match serde_json::from_str::<LpRequest>(&fact.content) {
                Ok(req) => {
                    let plan = solve_lp(&req);
                    let confidence = match plan.status.as_str() {
                        "optimal" => 1.0,
                        "feasible" => 0.7,
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
                    warn!(id = %fact.id, error = %e, "malformed glop-request");
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

pub fn solve_lp(req: &LpRequest) -> LpPlan {
    let mut solver = LinearSolver::new_glop(&req.id);
    let mut name_to_idx: HashMap<String, i32> = HashMap::new();

    for var in &req.variables {
        let idx = solver.num_var(var.lb, var.ub, &var.name);
        name_to_idx.insert(var.name.clone(), idx);
    }

    for con in &req.constraints {
        let ci = solver.add_constraint(con.lb, con.ub, &con.name);
        for term in &con.terms {
            if let Some(&vi) = name_to_idx.get(&term.var) {
                solver.set_constraint_coeff(ci, vi, term.coeff);
            }
        }
    }

    for term in &req.objective.terms {
        if let Some(&vi) = name_to_idx.get(&term.var) {
            solver.set_objective_coeff(vi, term.coeff);
        }
    }

    if req.objective.maximize {
        solver.maximize();
    } else {
        solver.minimize();
    }

    let status = match solver.solve() {
        OrtoolsStatus::Optimal => "optimal",
        OrtoolsStatus::Feasible => "feasible",
        OrtoolsStatus::Infeasible => "infeasible",
        OrtoolsStatus::Unbounded => "unbounded",
        _ => "error",
    };

    let values: Vec<(String, f64)> = req
        .variables
        .iter()
        .filter_map(|v| {
            name_to_idx
                .get(&v.name)
                .map(|&vi| (v.name.clone(), solver.var_value(vi)))
        })
        .collect();

    let objective_value = if matches!(status, "optimal" | "feasible") {
        solver.objective_value()
    } else {
        0.0
    };

    LpPlan {
        request_id: req.id.clone(),
        status: status.to_string(),
        values,
        objective_value,
        solver: "glop-v9.15".to_string(),
    }
}
