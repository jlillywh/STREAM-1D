//! WebAssembly `wasm-bindgen` exports (JsValue glue; logic lives in `json_api` / solvers).

use crate::json_api;
use crate::wasm_api;
use serde_wasm_bindgen;
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
/// Returns `{ warnings: string[] }` for non-fatal issues (e.g. bridge opening wider than parent XS).
/// Throws on JSON/schema parse errors.
#[wasm_bindgen(js_name = validateSteadyInputs)]
pub fn validate_steady_inputs_wasm(inputs_val: JsValue) -> Result<JsValue, JsValue> {
    let inputs: crate::solvers::SteadyInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Invalid SteadyInputs: {}", e)))?;
    let json = serde_json::to_string(&inputs)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize SteadyInputs: {}", e)))?;
    let out_json = json_api::validate_steady_inputs_json(&json)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::from_str(&out_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse validation result: {}", e)))
        .and_then(|v: serde_json::Value| {
            serde_wasm_bindgen::to_value(&v)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize validation result: {}", e)))
        })
}

/// WebAssembly entrypoint to run a steady-state hydraulic profile simulation.
/// Receives a serialized `SteadyInputs` JS object and returns a `SteadyResult` JS object.
#[wasm_bindgen(js_name = solveSteady)]
pub fn solve_steady_wasm(inputs_val: JsValue) -> Result<JsValue, JsValue> {
    let inputs: crate::solvers::SteadyInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse SteadyInputs: {}", e)))?;
    let json = serde_json::to_string(&inputs)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize SteadyInputs: {}", e)))?;
    let out_json = json_api::steady_result_json(&json)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::from_str(&out_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse SteadyResult: {}", e)))
        .and_then(|v: serde_json::Value| {
            serde_wasm_bindgen::to_value(&v)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize SteadyResult: {}", e)))
        })
}

/// Compute culvert headwater vs discharge at fixed tailwater (rating curve).
#[wasm_bindgen(js_name = computeCulvertRatingCurve)]
pub fn compute_culvert_rating_curve_wasm(inputs_val: JsValue) -> Result<JsValue, JsValue> {
    let inputs: crate::solvers::CulvertRatingCurveInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse CulvertRatingCurveInputs: {}", e)))?;
    let json = serde_json::to_string(&inputs)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize CulvertRatingCurveInputs: {}", e)))?;
    let out_json = json_api::culvert_rating_curve_json(&json)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::from_str(&out_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse CulvertRatingCurveResult: {}", e)))
        .and_then(|v: serde_json::Value| {
            serde_wasm_bindgen::to_value(&v)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize CulvertRatingCurveResult: {}", e)))
        })
}

/// Compute bridge upstream headwater vs discharge at fixed tailwater (rating curve).
#[wasm_bindgen(js_name = computeBridgeRatingCurve)]
pub fn compute_bridge_rating_curve_wasm(inputs_val: JsValue) -> Result<JsValue, JsValue> {
    let inputs: crate::solvers::BridgeRatingCurveInputs = serde_wasm_bindgen::from_value(inputs_val)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse BridgeRatingCurveInputs: {}", e)))?;
    let json = serde_json::to_string(&inputs)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize BridgeRatingCurveInputs: {}", e)))?;
    let out_json = json_api::bridge_rating_curve_json(&json)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::from_str(&out_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse BridgeRatingCurveResult: {}", e)))
        .and_then(|v: serde_json::Value| {
            serde_wasm_bindgen::to_value(&v)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize BridgeRatingCurveResult: {}", e)))
        })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_version_matches_metadata() {
        assert_eq!(get_engine_version_wasm(), wasm_api::ENGINE_VERSION);
    }
}
