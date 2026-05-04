//! Multi-agent task scheduling suggestors.
//!
//! Two competing implementations of the same scheduling contract:
//!
//! | Suggestor | Algorithm | Latency | Confidence |
//! |---|---|---|---|
//! | [`GreedySchedulerSuggestor`] | EDF + earliest-available | sub-ms | ≤ 0.65 |
//! | [`CpSatSchedulerSuggestor`] | CP-SAT optional-interval | seconds | ≤ 1.0 |
//!
//! # Formation pattern
//!
//! Register both in the same [`converge_core::Engine`].  Both accept
//! `"scheduling-request:*"` seeds and emit solver-prefixed plans to
//! `ContextKey::Strategies`:
//!
//! - Greedy → `"scheduling-plan-greedy:<id>"`
//! - CP-SAT → `"scheduling-plan-cpsat:<id>"`
//!
//! Downstream consumers compare confidence scores and select the plan that
//! maximises throughput.  In practice: greedy answers in < 1 ms; CP-SAT
//! improves on it and proves optimality within the time budget.
//!
//! # Benchmark result (60 tasks · 12 agents · 5 skills · horizon 360 min)
//!
//! ```text
//! Greedy:  56 / 60 tasks scheduled   (93.3 %)   0.03 ms
//! CP-SAT:  60 / 60 tasks scheduled  (100.0 %)   260 ms   ← optimal
//! ```
//!
//! CP-SAT scheduled 4 additional tasks that greedy could not fit, and proved
//! the schedule is globally optimal.

pub mod problem;

pub mod greedy;

#[cfg(feature = "ortools")]
pub mod cpsat;

pub use greedy::GreedySchedulerSuggestor;
pub use problem::{
    SchedulingAgent, SchedulingPlan, SchedulingRequest, SchedulingTask, TaskAssignment,
};

#[cfg(feature = "ortools")]
pub use cpsat::CpSatSchedulerSuggestor;
