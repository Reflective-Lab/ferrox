/// Custom serde for f64 bounds that maps ±infinity to ±1e308 in JSON.
///
/// JSON has no infinity literal. This module round-trips finite values exactly
/// and maps `f64::INFINITY` ↔ `1e308` and `f64::NEG_INFINITY` ↔ `-1e308`.
/// Apply with `#[serde(with = "crate::serde_util::f64_inf")]` on lb/ub fields.
#[allow(dead_code)]
pub mod f64_inf {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    const INF_PROXY: f64 = 1e308;

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
        let proxy = if *v == f64::INFINITY {
            INF_PROXY
        } else if *v == f64::NEG_INFINITY {
            -INF_PROXY
        } else {
            *v
        };
        proxy.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
        let v = f64::deserialize(d)?;
        Ok(if v >= INF_PROXY {
            f64::INFINITY
        } else if v <= -INF_PROXY {
            f64::NEG_INFINITY
        } else {
            v
        })
    }
}
