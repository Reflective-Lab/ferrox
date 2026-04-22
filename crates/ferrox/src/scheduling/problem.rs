use serde::{Deserialize, Serialize};

/// An agent that can execute tasks requiring one of its declared capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingAgent {
    pub id: usize,
    pub name: String,
    /// Capability tags this agent possesses (e.g. "python", "ml", "rust").
    pub capabilities: Vec<String>,
}

/// A unit of work to be scheduled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingTask {
    pub id: usize,
    pub name: String,
    /// The single capability an agent must have to execute this task.
    pub required_capability: String,
    /// Duration in minutes.
    pub duration_min: i64,
    /// Earliest start (minutes from horizon start).
    pub release_min: i64,
    /// Latest finish (minutes from horizon start).  Must be ≥ release + duration.
    pub deadline_min: i64,
}

/// Seeded into `ContextKey::Seeds` with id prefix `"scheduling-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingRequest {
    pub id: String,
    pub agents: Vec<SchedulingAgent>,
    pub tasks: Vec<SchedulingTask>,
    /// Planning horizon in minutes.
    pub horizon_min: i64,
    /// Per-solver time budget in seconds.  Suggestors may honour or ignore this.
    #[serde(default = "default_time_limit")]
    pub time_limit_seconds: f64,
}

fn default_time_limit() -> f64 {
    30.0
}

/// A single task-to-agent assignment with resolved timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub task_id: usize,
    pub task_name: String,
    pub agent_id: usize,
    pub agent_name: String,
    pub start_min: i64,
    pub end_min: i64,
}

/// Written to `ContextKey::Strategies` with id prefix `"scheduling-plan-<solver>:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingPlan {
    pub request_id: String,
    pub assignments: Vec<TaskAssignment>,
    pub tasks_total: usize,
    pub tasks_scheduled: usize,
    /// Completion time of the last scheduled task (0 if nothing scheduled).
    pub makespan_min: i64,
    /// Short identifier for the algorithm that produced this plan.
    pub solver: String,
    /// `"optimal"`, `"feasible"`, `"infeasible"`, or `"error"`.
    pub status: String,
    pub wall_time_seconds: f64,
}

impl SchedulingPlan {
    /// Throughput ratio: scheduled / total tasks.  Used to derive confidence.
    pub fn throughput_ratio(&self) -> f64 {
        if self.tasks_total == 0 {
            return 0.0;
        }
        self.tasks_scheduled as f64 / self.tasks_total as f64
    }
}
