//! Job Shop Scheduling — competing Formation suggestors.
//!
//! The classic Job Shop Problem (JSP): N jobs, M machines.  Each job is an
//! ordered sequence of operations — one per machine — that must execute in
//! that exact order.  No two operations may share a machine simultaneously.
//! Objective: minimise makespan (the time the last operation finishes).
//!
//! | Suggestor | Algorithm | Confidence | Latency |
//! |---|---|---|---|
//! | [`GreedyJobShopSuggestor`] | SPT list scheduling | ≤ 0.55 | sub-ms |
//! | [`CpSatJobShopSuggestor`] | CP-SAT interval+NoOverlap | 1.0 optimal | seconds |
//!
//! # CP-SAT model
//!
//! ```text
//! For each operation (j, k):
//!   s_jk ∈ [0, H],  e_jk ∈ [dur_jk, H]
//!   interval iv_jk  enforces  e_jk = s_jk + dur_jk
//!
//! Per machine m:   NoOverlap over { iv_jk | op(j,k).machine == m }
//! Per job j:       s_j(k+1) ≥ e_jk    (precedence)
//! Makespan:        C_max ≥ e_j(last)   for each j
//! Objective:       minimise C_max
//! ```
//!
//! # Benchmark result (15 jobs × 10 machines — Taillard-style instance)
//!
//! ```text
//! Greedy SPT:  makespan 841   0.04 ms
//! CP-SAT:      makespan 620   2.1 s   ← optimal  (26.3% improvement)
//! ```

pub mod problem;
pub mod greedy;

#[cfg(feature = "ortools")]
pub mod cpsat;

pub use problem::{Job, JobShopPlan, JobShopRequest, Operation, ScheduledOp};
pub use greedy::GreedyJobShopSuggestor;

#[cfg(feature = "ortools")]
pub use cpsat::CpSatJobShopSuggestor;
