//! JSON entrypoints shared by Python bindings and WASM glue (tested via `serde_json`).

pub(crate) fn steady_result_json(inputs_json: &str) -> Result<String, String> {
    let inputs: crate::solvers::SteadyInputs =
        serde_json::from_str(inputs_json).map_err(|e| format!("Failed to parse SteadyInputs JSON: {}", e))?;
    let result = crate::solvers::solve_steady(&inputs);
    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize SteadyResult: {}", e))
}

pub(crate) fn culvert_rating_curve_json(inputs_json: &str) -> Result<String, String> {
    let inputs: crate::solvers::CulvertRatingCurveInputs = serde_json::from_str(inputs_json)
        .map_err(|e| format!("Failed to parse CulvertRatingCurveInputs JSON: {}", e))?;
    let result = crate::solvers::compute_culvert_rating_curve(&inputs);
    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize CulvertRatingCurveResult: {}", e))
}

pub(crate) fn bridge_rating_curve_json(inputs_json: &str) -> Result<String, String> {
    let inputs: crate::solvers::BridgeRatingCurveInputs = serde_json::from_str(inputs_json)
        .map_err(|e| format!("Failed to parse BridgeRatingCurveInputs JSON: {}", e))?;
    let result = crate::solvers::compute_bridge_rating_curve(&inputs);
    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize BridgeRatingCurveResult: {}", e))
}

pub(crate) fn unsteady_result_json(inputs_json: &str) -> Result<String, String> {
    let inputs: crate::solvers::UnsteadyInputs =
        serde_json::from_str(inputs_json).map_err(|e| format!("Failed to parse UnsteadyInputs JSON: {}", e))?;
    let result = crate::solvers::solve_unsteady(&inputs);
    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize UnsteadyResult: {}", e))
}

pub(crate) fn validate_steady_inputs_json(inputs_json: &str) -> Result<String, String> {
    let inputs: crate::solvers::SteadyInputs =
        serde_json::from_str(inputs_json).map_err(|e| format!("Failed to parse SteadyInputs JSON: {}", e))?;
    let result = crate::solvers::validate_steady_inputs(&inputs);
    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize validation result: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_helpers_roundtrip() {
        let steady_json = include_str!("../tests/fixtures/wasm_steady_culvert_tier1.json");
        let steady_out = steady_result_json(steady_json).expect("steady json");
        assert!(steady_out.contains("\"wsel\""));
        let inputs: crate::solvers::SteadyInputs = serde_json::from_str(steady_json).unwrap();
        let steady_result: crate::solvers::SteadyResult = serde_json::from_str(&steady_out).unwrap();
        assert_eq!(steady_result.wsel.len(), inputs.cross_sections.len());

        let validation_out = validate_steady_inputs_json(steady_json).expect("validation json");
        assert!(validation_out.contains("warnings"));

        let culvert_json = r#"{"q_values":[50.0],"tw_wsel":12.0,"units":"USCustomary","shape_type":0,"inlet_type":1,"span":5.0,"rise":5.0,"roughness_n":0.012,"length":100.0,"entrance_loss_coeff":0.5,"exit_loss_coeff":1.0,"z_down":9.0,"z_up":10.0,"manning_n_bottom":0.012,"num_barrels":1}"#;
        assert!(culvert_rating_curve_json(culvert_json).unwrap().contains("\"q\""));

        let bridge_json = r#"{"q_values":[15.0],"low_chord":5.0,"high_chord":7.0,"z_down":0.0,"z_up":0.0,"tw_wsel":2.5,"units":"Metric","low_flow_method":3,"channel_width":10.0,"manning_n":0.03}"#;
        assert!(bridge_rating_curve_json(bridge_json).unwrap().contains("flow_regimes"));

        let unsteady_json = r#"{"cross_sections":[{"station":100.0,"x":[0.0,10.0],"y":[0.0,0.0],"n_stations":[0.0],"n_values":[0.03],"unit_system":"Metric"},{"station":0.0,"x":[0.0,10.0],"y":[0.0,0.0],"n_stations":[0.0],"n_values":[0.03],"unit_system":"Metric"}],"initial_wsel":[2.0,1.0],"initial_q":[10.0,10.0],"dt":60.0,"num_steps":2,"upstream_q_hydrograph":[10.0,10.0],"downstream_wsel_hydrograph":[1.0,1.0]}"#;
        assert!(unsteady_result_json(unsteady_json).unwrap().contains("\"wsel\""));
    }

    #[test]
    fn json_helpers_reject_invalid_input() {
        let err = steady_result_json("not json").unwrap_err();
        assert!(err.contains("Failed to parse SteadyInputs JSON"));
        let err = validate_steady_inputs_json("{").unwrap_err();
        assert!(err.contains("Failed to parse SteadyInputs JSON"));
        let err = culvert_rating_curve_json("{").unwrap_err();
        assert!(err.contains("Failed to parse CulvertRatingCurveInputs JSON"));
        let err = bridge_rating_curve_json("{").unwrap_err();
        assert!(err.contains("Failed to parse BridgeRatingCurveInputs JSON"));
        let err = unsteady_result_json("{").unwrap_err();
        assert!(err.contains("Failed to parse UnsteadyInputs JSON"));
    }
}
