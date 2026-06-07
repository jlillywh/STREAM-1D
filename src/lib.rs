pub mod utils;
pub mod geometry;
pub mod solvers;

use wasm_bindgen::prelude::*;

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
    m.add_function(wrap_pyfunction!(solve_unsteady_json_py, m)?)?;
    Ok(())
}
