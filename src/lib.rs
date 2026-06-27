pub mod geometry;
pub mod solvers;
pub mod utils;
pub mod wasm_api;

mod json_api;
mod wasm_bindings;

#[cfg(feature = "python")]
mod python_bindings;

pub use wasm_bindings::{
    compute_bridge_rating_curve_wasm, compute_culvert_rating_curve_wasm, get_engine_version_wasm,
    get_wasm_api_metadata_wasm, solve_steady_wasm, validate_steady_inputs_wasm,
};

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn _stream1d(py: Python, m: &PyModule) -> PyResult<()> {
    python_bindings::register(py, m)
}

#[cfg(test)]
mod entrypoint_tests {
    use super::wasm_api;

    #[test]
    fn api_metadata_json_roundtrip() {
        let meta = wasm_api::build_api_metadata();
        let json = serde_json::to_string(&meta).expect("serialize metadata");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse metadata");
        assert_eq!(
            parsed.get("api_version").and_then(|v| v.as_u64()),
            Some(wasm_api::API_VERSION as u64)
        );
    }
}
