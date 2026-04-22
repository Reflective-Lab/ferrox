use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use ferrox_ortools_sys::safe::CpModel;
use ferrox_ortools_sys::OrtoolsStatus;
use std::collections::HashMap;
use tracing::warn;

use super::problem::{CpSatPlan, CpSatRequest, ConstraintKind};

const REQUEST_PREFIX: &str = "cpsat-request:";
const PLAN_PREFIX: &str = "cpsat-plan:";

pub struct CpSatSuggestor;

#[async_trait]
impl Suggestor for CpSatSuggestor {
    fn name(&self) -> &str {
        "CpSatSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("NP-hard in general; CP-SAT DPLL+propagation+LNS; practical for n≤500 vars")
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id.starts_with(REQUEST_PREFIX) && !plan_exists(ctx, request_id(&f.id))
        })
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for fact in ctx.get(ContextKey::Seeds).iter().filter(|f| f.id.starts_with(REQUEST_PREFIX)) {
            let rid = request_id(&fact.id);
            if plan_exists(ctx, rid) {
                continue;
            }

            match serde_json::from_str::<CpSatRequest>(&fact.content) {
                Ok(req) => {
                    let plan = solve_cp(&req);
                    let confidence = match plan.status.as_str() {
                        "optimal"  => 1.0,
                        "feasible" => 0.7,
                        _          => 0.0,
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
                    warn!(id = %fact.id, error = %e, "malformed cpsat-request");
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
    ctx.get(ContextKey::Strategies).iter().any(|f| f.id == plan_id)
}

fn solve_cp(req: &CpSatRequest) -> CpSatPlan {
    let mut model = CpModel::new();
    let mut name_to_idx: HashMap<String, i32> = HashMap::new();

    for var in &req.variables {
        let idx = model.new_int_var(var.lb, var.ub, &var.name);
        name_to_idx.insert(var.name.clone(), idx);
    }

    for constraint in &req.constraints {
        match constraint {
            ConstraintKind::LinearLe { terms, rhs } => {
                let (vars, coeffs) = terms_to_vecs(terms, &name_to_idx);
                model.add_linear_le(&vars, &coeffs, *rhs);
            }
            ConstraintKind::LinearGe { terms, rhs } => {
                let (vars, coeffs) = terms_to_vecs(terms, &name_to_idx);
                model.add_linear_ge(&vars, &coeffs, *rhs);
            }
            ConstraintKind::LinearEq { terms, rhs } => {
                let (vars, coeffs) = terms_to_vecs(terms, &name_to_idx);
                model.add_linear_eq(&vars, &coeffs, *rhs);
            }
            ConstraintKind::AllDifferent { vars } => {
                let idxs: Vec<i32> = vars.iter()
                    .filter_map(|v| name_to_idx.get(v).copied())
                    .collect();
                model.add_all_different(&idxs);
            }
        }
    }

    if let Some(obj_terms) = &req.objective_terms {
        let (vars, coeffs) = terms_to_vecs(obj_terms, &name_to_idx);
        if req.minimize {
            model.minimize(&vars, &coeffs);
        } else {
            model.maximize(&vars, &coeffs);
        }
    }

    let time_limit = req.time_limit_seconds.unwrap_or(60.0);
    let solution = model.solve(time_limit);

    let status = match solution.status() {
        OrtoolsStatus::Optimal   => "optimal",
        OrtoolsStatus::Feasible  => "feasible",
        OrtoolsStatus::Infeasible => "infeasible",
        OrtoolsStatus::Unbounded  => "unbounded",
        _                         => "error",
    };

    let assignments = if solution.status().is_success() {
        req.variables.iter()
            .filter_map(|v| name_to_idx.get(&v.name).map(|&idx| (v.name.clone(), solution.value(idx))))
            .collect()
    } else {
        vec![]
    };

    let objective_value = if solution.status().is_success() && req.objective_terms.is_some() {
        Some(solution.objective_value())
    } else {
        None
    };

    CpSatPlan {
        request_id: req.id.clone(),
        status: status.to_string(),
        assignments,
        objective_value,
        wall_time_seconds: solution.wall_time(),
        solver: "cp-sat-v9.15",
    }
}

fn terms_to_vecs(
    terms: &[crate::cp::problem::CpTerm],
    name_to_idx: &HashMap<String, i32>,
) -> (Vec<i32>, Vec<i64>) {
    terms.iter()
        .filter_map(|t| name_to_idx.get(&t.var).map(|&idx| (idx, t.coeff)))
        .unzip()
}
