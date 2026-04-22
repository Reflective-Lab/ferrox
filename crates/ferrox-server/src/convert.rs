#![allow(clippy::result_large_err)]

use ferrox::cp::problem::{ConstraintKind, CpSatPlan, CpSatRequest, CpTerm, CpVariable};
use ferrox::lp::problem::{LpConstraint, LpObjective, LpPlan, LpRequest, LpTerm, LpVariable};
use ferrox::mip::problem::{
    MipConstraint, MipObjective, MipPlan, MipRequest, MipTerm, MipVariable, VarKind,
};
use tonic::Status;

use crate::proto::ferrox::v1 as p;

// ─── CP-SAT ──────────────────────────────────────────────────────────────────

pub fn cp_req_from_proto(r: p::SolveCpRequest) -> Result<CpSatRequest, Status> {
    let variables = r
        .variables
        .into_iter()
        .map(|v| CpVariable {
            name: v.name,
            lb: v.lb,
            ub: v.ub,
            is_bool: false,
        })
        .collect();

    let constraints = r
        .constraints
        .into_iter()
        .map(cp_constraint_from_proto)
        .collect::<Result<Vec<_>, _>>()?;

    let objective_terms = if r.objective_terms.is_empty() {
        None
    } else {
        Some(
            r.objective_terms
                .into_iter()
                .map(cp_term_from_proto)
                .collect(),
        )
    };

    Ok(CpSatRequest {
        id: r.id,
        variables,
        interval_vars: vec![],
        optional_interval_vars: vec![],
        constraints,
        objective_terms,
        minimize: r.minimize,
        time_limit_seconds: r.time_limit_seconds,
    })
}

fn cp_constraint_from_proto(c: p::CpConstraint) -> Result<ConstraintKind, Status> {
    use p::cp_constraint::Kind;
    match c
        .kind
        .ok_or_else(|| Status::invalid_argument("missing CpConstraint.kind"))?
    {
        Kind::LinearLe(l) => Ok(ConstraintKind::LinearLe {
            terms: l.terms.into_iter().map(cp_term_from_proto).collect(),
            rhs: l.rhs,
        }),
        Kind::LinearGe(l) => Ok(ConstraintKind::LinearGe {
            terms: l.terms.into_iter().map(cp_term_from_proto).collect(),
            rhs: l.rhs,
        }),
        Kind::LinearEq(l) => Ok(ConstraintKind::LinearEq {
            terms: l.terms.into_iter().map(cp_term_from_proto).collect(),
            rhs: l.rhs,
        }),
        Kind::AllDifferent(a) => Ok(ConstraintKind::AllDifferent { vars: a.vars }),
    }
}

fn cp_term_from_proto(t: p::CpTerm) -> CpTerm {
    CpTerm {
        var: t.var,
        coeff: t.coeff,
    }
}

pub fn cp_resp_to_proto(p: CpSatPlan) -> p::SolveCpResponse {
    p::SolveCpResponse {
        request_id: p.request_id,
        status: p.status,
        assignments: p
            .assignments
            .into_iter()
            .map(|(name, value)| p::StringI64 { name, value })
            .collect(),
        objective_value: p.objective_value,
        wall_time_seconds: p.wall_time_seconds,
        solver: p.solver,
    }
}

// ─── LP ──────────────────────────────────────────────────────────────────────

pub fn lp_req_from_proto(r: p::SolveLpRequest) -> Result<LpRequest, Status> {
    let objective = r
        .objective
        .ok_or_else(|| Status::invalid_argument("missing LpObjective"))?;

    Ok(LpRequest {
        id: r.id,
        variables: r
            .variables
            .into_iter()
            .map(|v| LpVariable {
                name: v.name,
                lb: v.lb,
                ub: v.ub,
            })
            .collect(),
        constraints: r
            .constraints
            .into_iter()
            .map(|c| LpConstraint {
                name: c.name,
                lb: c.lb,
                ub: c.ub,
                terms: c
                    .terms
                    .into_iter()
                    .map(|t| LpTerm {
                        var: t.var,
                        coeff: t.coeff,
                    })
                    .collect(),
            })
            .collect(),
        objective: LpObjective {
            terms: objective
                .terms
                .into_iter()
                .map(|t| LpTerm {
                    var: t.var,
                    coeff: t.coeff,
                })
                .collect(),
            maximize: objective.maximize,
        },
        time_limit_seconds: r.time_limit_seconds,
    })
}

pub fn lp_resp_to_proto(p: LpPlan) -> p::SolveLpResponse {
    p::SolveLpResponse {
        request_id: p.request_id,
        status: p.status,
        values: p
            .values
            .into_iter()
            .map(|(name, value)| p::StringF64 { name, value })
            .collect(),
        objective_value: p.objective_value,
        solver: p.solver,
    }
}

// ─── MIP ─────────────────────────────────────────────────────────────────────

pub fn mip_req_from_proto(r: p::SolveMipRequest) -> Result<MipRequest, Status> {
    let objective = r
        .objective
        .ok_or_else(|| Status::invalid_argument("missing MipObjective"))?;

    Ok(MipRequest {
        id: r.id,
        variables: r
            .variables
            .into_iter()
            .map(|v| MipVariable {
                name: v.name,
                lb: v.lb,
                ub: v.ub,
                kind: match v.kind {
                    1 => VarKind::Integer,
                    2 => VarKind::Binary,
                    _ => VarKind::Continuous,
                },
            })
            .collect(),
        constraints: r
            .constraints
            .into_iter()
            .map(|c| MipConstraint {
                name: c.name,
                lb: c.lb,
                ub: c.ub,
                terms: c
                    .terms
                    .into_iter()
                    .map(|t| MipTerm {
                        var: t.var,
                        coeff: t.coeff,
                    })
                    .collect(),
            })
            .collect(),
        objective: MipObjective {
            terms: objective
                .terms
                .into_iter()
                .map(|t| MipTerm {
                    var: t.var,
                    coeff: t.coeff,
                })
                .collect(),
            maximize: objective.maximize,
        },
        time_limit_seconds: r.time_limit_seconds,
        mip_gap_tolerance: r.mip_gap_tolerance,
    })
}

pub fn mip_resp_to_proto(p: MipPlan) -> p::SolveMipResponse {
    p::SolveMipResponse {
        request_id: p.request_id,
        status: p.status,
        values: p
            .values
            .into_iter()
            .map(|(name, value)| p::StringF64 { name, value })
            .collect(),
        objective_value: p.objective_value,
        mip_gap: p.mip_gap,
        solver: p.solver,
    }
}
