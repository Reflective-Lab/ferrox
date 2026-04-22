use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarKind { Continuous, Integer, Binary }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MipVariable {
    pub name: String,
    pub lb: f64,
    pub ub: f64,
    pub kind: VarKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MipTerm {
    pub var: String,
    pub coeff: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MipConstraint {
    pub name: String,
    pub lb: f64,
    pub ub: f64,
    pub terms: Vec<MipTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MipObjective {
    pub terms: Vec<MipTerm>,
    pub maximize: bool,
}

/// Seeded into `ContextKey::Seeds` with id prefix `"mip-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MipRequest {
    pub id: String,
    pub variables: Vec<MipVariable>,
    pub constraints: Vec<MipConstraint>,
    pub objective: MipObjective,
    pub time_limit_seconds: Option<f64>,
    pub mip_gap_tolerance: Option<f64>,
}

/// Written to `ContextKey::Strategies` with id prefix `"mip-plan:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MipPlan {
    pub request_id: String,
    pub status: String,
    pub values: Vec<(String, f64)>,
    pub objective_value: f64,
    pub mip_gap: f64,
    pub solver: &'static str,
}
