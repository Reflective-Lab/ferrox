use serde::{Deserialize, Serialize};

/// A single integer/boolean variable in a CP-SAT model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpVariable {
    pub name: String,
    pub lb: i64,
    pub ub: i64,
}

/// One term in a linear expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpTerm {
    pub var: String,
    pub coeff: i64,
}

/// A constraint over variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConstraintKind {
    LinearLe  { terms: Vec<CpTerm>, rhs: i64 },
    LinearGe  { terms: Vec<CpTerm>, rhs: i64 },
    LinearEq  { terms: Vec<CpTerm>, rhs: i64 },
    AllDifferent { vars: Vec<String> },
}

/// Seeded into `ContextKey::Seeds` with id prefix `"cpsat-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpSatRequest {
    pub id: String,
    pub variables: Vec<CpVariable>,
    pub constraints: Vec<ConstraintKind>,
    pub objective_terms: Option<Vec<CpTerm>>,
    pub minimize: bool,
    pub time_limit_seconds: Option<f64>,
}

/// Written to `ContextKey::Strategies` with id prefix `"cpsat-plan:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpSatPlan {
    pub request_id: String,
    pub status: String,
    pub assignments: Vec<(String, i64)>,
    pub objective_value: Option<i64>,
    pub wall_time_seconds: f64,
    pub solver: &'static str,
}
