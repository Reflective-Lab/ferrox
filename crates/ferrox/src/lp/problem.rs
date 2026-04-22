use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpVariable {
    pub name: String,
    pub lb: f64,
    pub ub: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpTerm {
    pub var: String,
    pub coeff: f64,
}

/// A row constraint: lb <= sum(terms) <= ub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpConstraint {
    pub name: String,
    pub lb: f64,
    pub ub: f64,
    pub terms: Vec<LpTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpObjective {
    pub terms: Vec<LpTerm>,
    pub maximize: bool,
}

/// Seeded into `ContextKey::Seeds` with id prefix `"glop-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpRequest {
    pub id: String,
    pub variables: Vec<LpVariable>,
    pub constraints: Vec<LpConstraint>,
    pub objective: LpObjective,
    pub time_limit_seconds: Option<f64>,
}

/// Written to `ContextKey::Strategies` with id prefix `"glop-plan:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpPlan {
    pub request_id: String,
    pub status: String,
    pub values: Vec<(String, f64)>,
    pub objective_value: f64,
    pub solver: &'static str,
}
