pub mod utils;
pub mod geometry;
pub mod solvers;
pub mod wasm_api;

use wasm_bindgen::prelude::*;

/// Returns the engine semantic version string (e.g. `"0.1.0"`).
#[wasm_bindgen(js_name = getEngineVersion)]
pub fn get_engine_version_wasm() -> String {
    wasm_api::ENGINE_VERSION.to_string()
}

/// Returns WASM API metadata (inlet codes, shape types, Tier 1 field names, `api_version`).
///
/// Host apps should call this after `init()` to detect contract version and populate UI enums.
#[wasm_bindgen(js_name = getWasmApiMetadata)]
pub fn get_wasm_api_metadata_wasm() -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&wasm_api::build_api_metadata())
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize API metadata: {}", e)))
}

/// Validates a `SteadyInputs` object without running the solver.
///
/// Use in the web app before dispatching to a Worker to surface JSON/schema errors early.
#[wasm_bindgen(js_name = validateSteadyInputs)]
pub fn validate_steady_inputs_wasm(inputs_val: JsValue) -> Result<(), JsValue> {
    let _: solvers::SteadyInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Invalid SteadyInputs: {}", e)))?;
    Ok(())
}

/// WebAssembly entrypoint to run a steady-state hydraulic profile simulation.
/// Receives a serialized `SteadyInputs` JS object and returns a `SteadyResult` JS object.
#[wasm_bindgen(js_name = solveSteady)]
pub fn solve_steady_wasm(inputs_val: JsValue) -> Result<JsValue, JsValue> {
    // Deserialize JS value to Rust SteadyInputs
    let inputs: solvers::SteadyInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse SteadyInputs: {}", e)))?;

    // Run solver
    let result = solvers::solve_steady(&inputs);

    // Serialize Rust SteadyResult to JS value
    let js_res = serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize SteadyResult: {}", e)))?;

    Ok(js_res)
}

/// Compute culvert headwater vs discharge at fixed tailwater (rating curve).
#[wasm_bindgen(js_name = computeCulvertRatingCurve)]
pub fn compute_culvert_rating_curve_wasm(inputs_val: JsValue) -> Result<JsValue, JsValue> {
    let inputs: solvers::CulvertRatingCurveInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse CulvertRatingCurveInputs: {}", e)))?;
    let result = solvers::compute_culvert_rating_curve(&inputs);
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize CulvertRatingCurveResult: {}", e)))
}

/// Compute bridge upstream headwater vs discharge at fixed tailwater (rating curve).
#[wasm_bindgen(js_name = computeBridgeRatingCurve)]
pub fn compute_bridge_rating_curve_wasm(inputs_val: JsValue) -> Result<JsValue, JsValue> {
    let inputs: solvers::BridgeRatingCurveInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse BridgeRatingCurveInputs: {}", e)))?;
    let result = solvers::compute_bridge_rating_curve(&inputs);
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize BridgeRatingCurveResult: {}", e)))
}

/// WebAssembly entrypoint to run an unsteady-state hydraulic routing simulation.
/// Receives a serialized `UnsteadyInputs` JS object and returns an `UnsteadyResult` JS object.
#[wasm_bindgen(js_name = solveUnsteady)]
pub fn solve_unsteady_wasm(inputs_val: JsValue) -> Result<JsValue, JsValue> {
    // Deserialize JS value to Rust UnsteadyInputs
    let inputs: solvers::UnsteadyInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse UnsteadyInputs: {}", e)))?;

    // Run solver
    let result = solvers::solve_unsteady(&inputs);

    // Serialize Rust UnsteadyResult to JS value
    let js_res = serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize UnsteadyResult: {}", e)))?;

    Ok(js_res)
}

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "solve_steady_json")]
pub fn solve_steady_json_py(inputs_json: &str) -> PyResult<String> {
    let inputs: solvers::SteadyInputs = serde_json::from_str(inputs_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to parse SteadyInputs JSON: {}", e)))?;
    let result = solvers::solve_steady(&inputs);
    let res_json = serde_json::to_string(&result)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to serialize SteadyResult: {}", e)))?;
    Ok(res_json)
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "compute_culvert_rating_curve_json")]
pub fn compute_culvert_rating_curve_json_py(inputs_json: &str) -> PyResult<String> {
    let inputs: solvers::CulvertRatingCurveInputs = serde_json::from_str(inputs_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to parse CulvertRatingCurveInputs JSON: {}", e)))?;
    let result = solvers::compute_culvert_rating_curve(&inputs);
    serde_json::to_string(&result)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to serialize CulvertRatingCurveResult: {}", e)))
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "compute_bridge_rating_curve_json")]
pub fn compute_bridge_rating_curve_json_py(inputs_json: &str) -> PyResult<String> {
    let inputs: solvers::BridgeRatingCurveInputs = serde_json::from_str(inputs_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to parse BridgeRatingCurveInputs JSON: {}", e)))?;
    let result = solvers::compute_bridge_rating_curve(&inputs);
    serde_json::to_string(&result)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to serialize BridgeRatingCurveResult: {}", e)))
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "solve_unsteady_json")]
pub fn solve_unsteady_json_py(inputs_json: &str) -> PyResult<String> {
    let inputs: solvers::UnsteadyInputs = serde_json::from_str(inputs_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to parse UnsteadyInputs JSON: {}", e)))?;
    let result = solvers::solve_unsteady(&inputs);
    let res_json = serde_json::to_string(&result)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to serialize UnsteadyResult: {}", e)))?;
    Ok(res_json)
}

#[cfg(feature = "python")]
#[pymodule]
fn _streams1d(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(solve_steady_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_culvert_rating_curve_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_bridge_rating_curve_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(solve_unsteady_json_py, m)?)?;
    Ok(())
}
