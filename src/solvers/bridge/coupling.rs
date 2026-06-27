use crate::geometry::GeometryTable;
use crate::utils::{UnitSystem, CFS_TO_CMS, FT_TO_M};

use super::geometry::{build_bridge_geometry, BridgeDeckProfile};
use super::headwater::{solve_bridge_headwater_metric, solve_low_flow_tailwater};
use super::high_flow::solve_high_flow_tailwater;
use super::low_flow::classify_low_flow;
use super::section::{
    bridge_q_to_metric_magnitude, mirror_bridge_section_context, BridgeFlowDirection,
    BridgeSectionContext,
};
use super::types::{BridgeCouplingParams, LowFlowClass};

/// Result of a bridge headwater–tailwater coupling solve (steady or unsteady post-step).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeSolveResult {
    pub wsel_up: f64,
    pub wsel_down: f64,
    pub head_loss: f64,
    /// `low_a`, `low_b`, `low_c`, `pressure`, `weir`, or `energy`
    pub flow_regime: String,
}

#[allow(dead_code)]
pub(crate) fn bridge_flow_regime_label(
    tw_user: f64,
    wsel_up_user: f64,
    low_chord: f64,
    high_chord: f64,
    units: UnitSystem,
    q_user: f64,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
    coupling: &BridgeCouplingParams,
    interval_length_m: f64,
    pier_width: f64,
    num_piers: i32,
    pier_shape_type: i32,
    weir_coeff: f64,
    orifice_coeff: f64,
    z_down: f64,
    z_up: f64,
) -> String {
    let _ = q_user;
    if tw_user >= low_chord {
        if coupling.high_flow_method == 1 {
            return "energy".to_string();
        }
        if wsel_up_user >= high_chord {
            "weir".to_string()
        } else {
            "pressure".to_string()
        }
    } else {
        let geom = build_bridge_geometry(
            low_chord,
            high_chord,
            pier_width,
            num_piers,
            pier_shape_type,
            weir_coeff,
            orifice_coeff,
            z_down,
            z_up,
            units,
            coupling,
            interval_length_m,
            None,
            None,
        );
        let tw_m = if units == UnitSystem::USCustomary {
            tw_user * FT_TO_M
        } else {
            tw_user
        };
        let q_metric = if units == UnitSystem::USCustomary {
            q_user * CFS_TO_CMS
        } else {
            q_user
        };
        match classify_low_flow(q_metric, tw_m, &geom, table_up, table_down) {
            LowFlowClass::A => "low_a".to_string(),
            LowFlowClass::B => "low_b".to_string(),
            LowFlowClass::C => "low_c".to_string(),
        }
    }
}

/// Couples reach BU/BD WSELs for inline bridge routing.
///
/// `tw_wsel` is tailwater on the **hydraulic downstream** face: BD when `q > 0`, BU when `q < 0`.
/// Returns reach-frame `wsel_up` (BU) and `wsel_down` (BD).
pub fn solve_bridge_coupled(
    q: f64,
    low_chord: f64,
    high_chord: f64,
    pier_width: f64,
    num_piers: i32,
    pier_shape_type: i32,
    weir_coeff: f64,
    orifice_coeff: f64,
    z_down: f64,
    z_up: f64,
    tw_wsel: f64,
    units: UnitSystem,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
    coupling: &BridgeCouplingParams,
    interval_length_m: f64,
    deck: Option<&BridgeDeckProfile>,
    sections: Option<&BridgeSectionContext>,
) -> BridgeSolveResult {
    let direction = BridgeFlowDirection::from_q(q);
    let q_metric = bridge_q_to_metric_magnitude(q, units);
    let mirrored_sections = sections.map(mirror_bridge_section_context);
    let (table_hyd_us, table_hyd_ds, z_hyd_us, z_hyd_ds, sections_hyd) = match direction {
        BridgeFlowDirection::Downstream => (table_up, table_down, z_up, z_down, sections),
        BridgeFlowDirection::Upstream => (
            table_down,
            table_up,
            z_down,
            z_up,
            mirrored_sections.as_ref(),
        ),
    };
    let geom = build_bridge_geometry(
        low_chord,
        high_chord,
        pier_width,
        num_piers,
        pier_shape_type,
        weir_coeff,
        orifice_coeff,
        z_hyd_ds,
        z_hyd_us,
        units,
        coupling,
        interval_length_m,
        deck,
        sections_hyd,
    );
    let tw_m = if units == UnitSystem::USCustomary {
        tw_wsel * FT_TO_M
    } else {
        tw_wsel
    };
    let tw_clamped = tw_m.max(geom.z_down_m + 1e-4);
    let solved =
        solve_bridge_headwater_metric(q_metric, tw_clamped, &geom, table_hyd_us, table_hyd_ds);
    let hw_hyd_user = if units == UnitSystem::USCustomary {
        solved.wsel_m / FT_TO_M
    } else {
        solved.wsel_m
    };
    let (wsel_bu, wsel_bd) = match direction {
        BridgeFlowDirection::Downstream => (hw_hyd_user, tw_wsel),
        BridgeFlowDirection::Upstream => (tw_wsel, hw_hyd_user),
    };
    let head_loss = (hw_hyd_user - tw_wsel).max(0.0);
    BridgeSolveResult {
        wsel_up: wsel_bu,
        wsel_down: wsel_bd,
        head_loss,
        flow_regime: solved.regime.as_str().to_string(),
    }
}
/// Reach BU-face WSEL after subcritical coupling (`tw_wsel` on hydraulic downstream face).
pub fn solve_bridge_wsel(
    q: f64,
    low_chord: f64,
    high_chord: f64,
    pier_width: f64,
    num_piers: i32,
    pier_shape_type: i32,
    weir_coeff: f64,
    orifice_coeff: f64,
    z_down: f64,
    z_up: f64,
    tw_wsel: f64,
    units: UnitSystem,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
    coupling: &BridgeCouplingParams,
    interval_length_m: f64,
    deck: Option<&BridgeDeckProfile>,
    sections: Option<&BridgeSectionContext>,
) -> f64 {
    solve_bridge_coupled(
        q,
        low_chord,
        high_chord,
        pier_width,
        num_piers,
        pier_shape_type,
        weir_coeff,
        orifice_coeff,
        z_down,
        z_up,
        tw_wsel,
        units,
        table_up,
        table_down,
        coupling,
        interval_length_m,
        deck,
        sections,
    )
    .wsel_up
}

/// Reach BD-face WSEL after supercritical coupling (`hw_wsel` on hydraulic upstream face).
pub fn solve_bridge_tailwater(
    q: f64,
    low_chord: f64,
    high_chord: f64,
    pier_width: f64,
    num_piers: i32,
    pier_shape_type: i32,
    weir_coeff: f64,
    orifice_coeff: f64,
    z_down: f64,
    z_up: f64,
    hw_wsel: f64,
    units: UnitSystem,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
    coupling: &BridgeCouplingParams,
    interval_length_m: f64,
    deck: Option<&BridgeDeckProfile>,
    sections: Option<&BridgeSectionContext>,
) -> f64 {
    let direction = BridgeFlowDirection::from_q(q);
    let q_metric = bridge_q_to_metric_magnitude(q, units);
    let mirrored_sections = sections.map(mirror_bridge_section_context);
    let (table_hyd_us, table_hyd_ds, z_hyd_us, z_hyd_ds, sections_hyd) = match direction {
        BridgeFlowDirection::Downstream => (table_up, table_down, z_up, z_down, sections),
        BridgeFlowDirection::Upstream => (
            table_down,
            table_up,
            z_down,
            z_up,
            mirrored_sections.as_ref(),
        ),
    };
    let geom = build_bridge_geometry(
        low_chord,
        high_chord,
        pier_width,
        num_piers,
        pier_shape_type,
        weir_coeff,
        orifice_coeff,
        z_hyd_ds,
        z_hyd_us,
        units,
        coupling,
        interval_length_m,
        deck,
        sections_hyd,
    );

    let hw_m = if units == UnitSystem::USCustomary {
        hw_wsel * FT_TO_M
    } else {
        hw_wsel
    };

    let tw_metric = if hw_m >= geom.low_chord_m {
        solve_high_flow_tailwater(q_metric, &geom, hw_m, table_hyd_us, table_hyd_ds)
    } else {
        solve_low_flow_tailwater(q_metric, hw_m, &geom, table_hyd_us, table_hyd_ds)
    };

    if units == UnitSystem::USCustomary {
        tw_metric / FT_TO_M
    } else {
        tw_metric
    }
}
