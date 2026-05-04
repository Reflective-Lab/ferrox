use serde::{Deserialize, Serialize};

/// One operation within a job: must execute on `machine_id` for `duration` units.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub machine_id: usize,
    pub duration: i64,
}

/// A job is an ordered sequence of operations; each must complete before the next begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: usize,
    pub name: String,
    /// Operations in their required execution order.
    pub operations: Vec<Operation>,
}

/// Seeded into `ContextKey::Seeds` with id prefix `"jspbench-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobShopRequest {
    pub id: String,
    pub jobs: Vec<Job>,
    pub num_machines: usize,
    #[serde(default = "default_time_limit")]
    pub time_limit_seconds: f64,
}

fn default_time_limit() -> f64 {
    30.0
}

impl JobShopRequest {
    /// Trivial upper bound on makespan: sum of all operation durations.
    pub fn horizon(&self) -> i64 {
        self.jobs
            .iter()
            .flat_map(|j| j.operations.iter())
            .map(|o| o.duration)
            .sum()
    }
}

/// A scheduled operation with resolved timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledOp {
    pub job_id: usize,
    pub job_name: String,
    pub machine_id: usize,
    pub op_index: usize,
    pub start: i64,
    pub end: i64,
}

/// Written to `ContextKey::Strategies` with id prefix `"jspbench-plan-<solver>:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobShopPlan {
    pub request_id: String,
    pub schedule: Vec<ScheduledOp>,
    pub makespan: i64,
    /// Proven lower bound (available when status is `"optimal"`).
    pub lower_bound: Option<i64>,
    pub solver: String,
    /// `"optimal"`, `"feasible"`, or `"error"`.
    pub status: String,
    pub wall_time_seconds: f64,
}
