use thiserror::Error;

#[derive(Debug, Error)]
pub enum FerroxError {
    #[error("solver returned infeasible")]
    Infeasible,
    #[error("solver returned unbounded")]
    Unbounded,
    #[error("model invalid: {0}")]
    ModelInvalid(String),
    #[error("solver error")]
    SolverError,
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("no pending request")]
    NoPendingRequest,
}

pub type Result<T> = std::result::Result<T, FerroxError>;
