pub mod error;

#[cfg(feature = "ortools")]
pub mod cp;
#[cfg(feature = "ortools")]
pub mod lp;
#[cfg(feature = "highs")]
pub mod mip;
#[cfg(feature = "ortools")]
pub mod formation;

pub use error::{FerroxError, Result};
