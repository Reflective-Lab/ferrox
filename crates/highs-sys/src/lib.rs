#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

#[cfg(feature = "link")]
use std::os::raw::c_int;

// ── Status ────────────────────────────────────────────────────────────────────

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighsReturnStatus {
    Ok      = 0,
    Warning = 1,
    Error   = 2,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighsModelStatus {
    NotSet        = 0,
    LoadError     = 1,
    ModelError    = 2,
    Infeasible    = 8,
    Optimal       = 7,
    Unbounded     = 9,
    SolutionLimit = 11,
    TimeLimit     = 12,
}

impl HighsModelStatus {
    pub fn is_success(self) -> bool {
        matches!(self, Self::Optimal | Self::SolutionLimit | Self::TimeLimit)
    }
    pub fn is_optimal(self) -> bool {
        self == Self::Optimal
    }
}

// ── Opaque C type ─────────────────────────────────────────────────────────────

#[repr(C)] pub struct HighsHandle { _p: [u8; 0] }

// ── FFI declarations ──────────────────────────────────────────────────────────

#[cfg(feature = "link")]
extern "C" {
    pub fn highs_create() -> *mut HighsHandle;
    pub fn highs_destroy(h: *mut HighsHandle);
    pub fn highs_add_col(h: *mut HighsHandle, cost: f64, lb: f64, ub: f64) -> HighsReturnStatus;
    pub fn highs_add_row(h: *mut HighsHandle, lb: f64, ub: f64,
                         num_nz: c_int, idx: *const c_int, val: *const f64) -> HighsReturnStatus;
    pub fn highs_change_col_integer_type(h: *mut HighsHandle, col: c_int,
                                         is_integer: c_int) -> HighsReturnStatus;
    pub fn highs_set_time_limit(h: *mut HighsHandle, seconds: f64) -> HighsReturnStatus;
    pub fn highs_set_mip_rel_gap(h: *mut HighsHandle, gap: f64) -> HighsReturnStatus;
    pub fn highs_run(h: *mut HighsHandle) -> HighsReturnStatus;
    pub fn highs_get_model_status(h: *mut HighsHandle) -> HighsModelStatus;
    pub fn highs_get_objective_value(h: *mut HighsHandle) -> f64;
    pub fn highs_get_col_value(h: *mut HighsHandle, col: c_int) -> f64;
    pub fn highs_get_mip_gap(h: *mut HighsHandle) -> f64;
}

// ── Safe wrapper ──────────────────────────────────────────────────────────────

#[cfg(feature = "link")]
pub mod safe {
    use super::*;
    use std::ptr::NonNull;

    pub struct HighsSolver {
        ptr: NonNull<HighsHandle>,
        num_cols: usize,
    }

    impl HighsSolver {
        pub fn new() -> Self {
            unsafe {
                Self {
                    ptr: NonNull::new(highs_create()).expect("highs_create returned null"),
                    num_cols: 0,
                }
            }
        }

        /// Add a continuous variable. Returns the column index.
        pub fn add_col(&mut self, cost: f64, lb: f64, ub: f64) -> usize {
            unsafe { highs_add_col(self.ptr.as_ptr(), cost, lb, ub) };
            let idx = self.num_cols;
            self.num_cols += 1;
            idx
        }

        /// Add an integer variable. Returns the column index.
        pub fn add_int_col(&mut self, cost: f64, lb: f64, ub: f64) -> usize {
            let col = self.add_col(cost, lb, ub);
            unsafe { highs_change_col_integer_type(self.ptr.as_ptr(), col as i32, 1) };
            col
        }

        /// Add a binary (0/1) variable. Returns the column index.
        pub fn add_bin_col(&mut self, cost: f64) -> usize {
            self.add_int_col(cost, 0.0, 1.0)
        }

        /// Add a row constraint: lb <= sum(idx[i] * val[i]) <= ub
        pub fn add_row(&mut self, lb: f64, ub: f64, indices: &[i32], coeffs: &[f64]) {
            assert_eq!(indices.len(), coeffs.len());
            unsafe {
                highs_add_row(self.ptr.as_ptr(), lb, ub,
                              indices.len() as i32, indices.as_ptr(), coeffs.as_ptr());
            }
        }

        pub fn set_time_limit(&mut self, secs: f64) {
            unsafe { highs_set_time_limit(self.ptr.as_ptr(), secs); }
        }

        pub fn set_mip_rel_gap(&mut self, gap: f64) {
            unsafe { highs_set_mip_rel_gap(self.ptr.as_ptr(), gap); }
        }

        pub fn run(&mut self) -> HighsModelStatus {
            unsafe {
                highs_run(self.ptr.as_ptr());
                highs_get_model_status(self.ptr.as_ptr())
            }
        }

        pub fn objective_value(&self) -> f64 {
            unsafe { highs_get_objective_value(self.ptr.as_ptr()) }
        }

        pub fn col_value(&self, col: usize) -> f64 {
            unsafe { highs_get_col_value(self.ptr.as_ptr(), col as i32) }
        }

        pub fn mip_gap(&self) -> f64 {
            unsafe { highs_get_mip_gap(self.ptr.as_ptr()) }
        }
    }

    impl Default for HighsSolver {
        fn default() -> Self { Self::new() }
    }

    impl Drop for HighsSolver {
        fn drop(&mut self) { unsafe { highs_destroy(self.ptr.as_ptr()) } }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_values() {
        assert_eq!(HighsModelStatus::Optimal as i32, 7);
        assert!(HighsModelStatus::Optimal.is_success());
        assert!(!HighsModelStatus::Infeasible.is_success());
    }

    #[cfg(feature = "link")]
    mod integration {
        use super::super::safe::*;

        #[test]
        fn mip_binary_knapsack() {
            // Maximize 5x + 4y + 3z  subject to  2x + 3y + 2z <= 5,  x,y,z in {0,1}
            let mut s = HighsSolver::new();
            let x = s.add_bin_col(-5.0); // minimize negated costs
            let y = s.add_bin_col(-4.0);
            let z = s.add_bin_col(-3.0);
            s.add_row(f64::NEG_INFINITY, 5.0,
                      &[x as i32, y as i32, z as i32],
                      &[2.0, 3.0, 2.0]);
            let status = s.run();
            assert!(status.is_success());
            // Optimal: x=1 (2), z=1 (2) → capacity used=4, value=8; or x=1,y=1 → capacity=5, value=9
            assert!((-s.objective_value() - 8.0).abs() < 0.5);
        }
    }
}
