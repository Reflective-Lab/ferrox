use serde::{Deserialize, Serialize};

/// A single integer variable in a CP-SAT model.
/// Set `is_bool = true` for binary (0/1) variables that may serve as
/// optional-interval literals; the solver treats them as `BoolVar` internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpVariable {
    pub name: String,
    pub lb: i64,
    pub ub: i64,
    #[serde(default)]
    pub is_bool: bool,
}

/// A fixed-duration interval variable.  The solver enforces `end == start + duration`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalVarDef {
    pub name: String,
    pub start_var: String,
    pub duration: i64,
    pub end_var: String,
}

/// An optional interval that is active only when `lit_var` (a bool variable) equals 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalIntervalVarDef {
    pub name: String,
    pub start_var: String,
    pub duration: i64,
    pub end_var: String,
    pub lit_var: String,
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
    LinearLe {
        terms: Vec<CpTerm>,
        rhs: i64,
    },
    LinearGe {
        terms: Vec<CpTerm>,
        rhs: i64,
    },
    LinearEq {
        terms: Vec<CpTerm>,
        rhs: i64,
    },
    AllDifferent {
        vars: Vec<String>,
    },
    /// None of the listed interval variables may overlap in time.
    NoOverlap {
        intervals: Vec<String>,
    },
}

/// Seeded into `ContextKey::Seeds` with id prefix `"cpsat-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpSatRequest {
    pub id: String,
    pub variables: Vec<CpVariable>,
    #[serde(default)]
    pub interval_vars: Vec<IntervalVarDef>,
    #[serde(default)]
    pub optional_interval_vars: Vec<OptionalIntervalVarDef>,
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
    pub solver: String,
}
