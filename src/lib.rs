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
