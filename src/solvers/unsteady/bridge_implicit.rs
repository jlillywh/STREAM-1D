//! Bridge low-flow residual for implicit Preissmann coupling (Phase 4).

use crate::geometry::{CrossSection, GeometryTable};
use crate::solvers::bridge::build_bridge_geometry;
use crate::solvers::bridge::implicit::{
    bridge_headwater_implicit_rhs, bridge_implicit_pin_class, bridge_tailwater_implicit_rhs,
    BridgeHeadwaterImplicitResidual,
};
use crate::solvers::bridge::{
    bridge_q_to_metric_magnitude, mirror_bridge_section_context, solve_bridge_coupled,
    BridgeFlowDirection, BridgeSolveResult, LowFlowClass,
};
use crate::solvers::bridge::unsteady_coupling::{
    bridge_interval_coupling, q_metric_to_user, wsel_metric_to_user, BridgeIntervalCoupling,
};
use crate::utils::UnitSystem;

use super::culvert_implicit::ImplicitMomentumRow;
use super::{
    bridge_coupling_for, bridge_deck_profile_for, bridge_face_geometry_for, UnsteadyInputs,
};

pub(crate) struct BridgeImplicitIntervalContext {
    pub geom: crate::solvers::bridge::BridgeGeometry,
    pub table_hyd_us: GeometryTable,
    pub table_hyd_ds: GeometryTable,
    pub face_coupling: BridgeIntervalCoupling,
    pub pinned: LowFlowClass,
}

pub(crate) fn build_bridge_implicit_context(
    inputs: &UnsteadyInputs,
    b_idx: usize,
    interval_i: usize,
    raw_units: UnitSystem,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
) -> Option<BridgeImplicitIntervalContext> {
    let b = &inputs.bridge;
    let interval_length_m = (densified_stations[interval_i] - densified_stations[interval_i + 1]).abs();
    let coupling = bridge_coupling_for(inputs, b_idx);
    let deck = bridge_deck_profile_for(inputs, b_idx, raw_units);
    let num_slices = inputs.num_slices.unwrap_or(100);
    let face_geo = bridge_face_geometry_for(
        inputs,
        b_idx,
        interval_i,
        raw_units,
        num_slices,
        densified_stations,
        densified_tables,
        densified_xs,
        densified_z_mins,
        interval_length_m,
    );

    let q_user = q_metric_to_user(q_metric[interval_i], raw_units);
    let face_coupling = bridge_interval_coupling(interval_i, q_user, densified_tables.len());
    let direction = BridgeFlowDirection::from_q(q_user);
    let mirrored_sections = Some(&face_geo.sections).map(mirror_bridge_section_context);
    let (table_hyd_us, table_hyd_ds, z_hyd_us, z_hyd_ds, sections_hyd) = match direction {
        BridgeFlowDirection::Downstream => (
            &face_geo.table_up,
            &face_geo.table_down,
            face_geo.z_up_user,
            face_geo.z_down_user,
            Some(&face_geo.sections),
        ),
        BridgeFlowDirection::Upstream => (
            &face_geo.table_down,
            &face_geo.table_up,
            face_geo.z_down_user,
            face_geo.z_up_user,
            mirrored_sections.as_ref(),
        ),
    };

    let s = bridge_scalars(b, b_idx, raw_units);
    let geom = build_bridge_geometry(
        s.low_chord,
        s.high_chord,
        s.pier_width,
        s.num_piers,
        s.pier_shape,
        s.weir_coeff,
        s.orifice_coeff,
        z_hyd_ds,
        z_hyd_us,
        raw_units,
        &coupling,
        interval_length_m,
        deck.as_ref(),
        sections_hyd,
    );

    let y_tw = y_metric[face_coupling.tw_face];
    let q_mag = bridge_q_to_metric_magnitude(q_user, raw_units);
    let pinned = bridge_implicit_pin_class(q_mag, y_tw, &geom, table_hyd_us, table_hyd_ds)?;

    Some(BridgeImplicitIntervalContext {
        geom,
        table_hyd_us: table_hyd_us.clone(),
        table_hyd_ds: table_hyd_ds.clone(),
        face_coupling,
        pinned,
    })
}

struct BridgeScalars {
    low_chord: f64,
    high_chord: f64,
    pier_width: f64,
    num_piers: i32,
    pier_shape: i32,
    weir_coeff: f64,
    orifice_coeff: f64,
}

fn bridge_scalars(b: &crate::solvers::unsteady::UnsteadyBridgeInputs, b_idx: usize, raw_units: UnitSystem) -> BridgeScalars {
    let weir_default = if raw_units == UnitSystem::USCustomary {
        2.6
    } else {
        1.44
    };
    BridgeScalars {
        low_chord: b.bridge_low_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0),
        high_chord: b.bridge_high_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0),
        pier_width: b.bridge_pier_widths.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0),
        num_piers: b.bridge_num_piers.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0),
        pier_shape: b.bridge_pier_shapes.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0),
        weir_coeff: b.bridge_weir_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(weir_default),
        orifice_coeff: b.bridge_orifice_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.5),
    }
}

fn bridge_implicit_residual(
    ctx: &BridgeImplicitIntervalContext,
    y_metric: &[f64],
    q_metric: &[f64],
    interval_i: usize,
    raw_units: UnitSystem,
    invert_tailwater: bool,
) -> Option<BridgeHeadwaterImplicitResidual> {
    let q_user = q_metric_to_user(q_metric[interval_i], raw_units);
    let q_mag = bridge_q_to_metric_magnitude(q_user, raw_units);
    let y_hw = y_metric[ctx.face_coupling.hw_face];
    let y_tw = y_metric[ctx.face_coupling.tw_face];
    let pinned = ctx.pinned;

    if invert_tailwater {
        bridge_tailwater_implicit_rhs(
            y_hw,
            y_tw,
            q_mag,
            pinned,
            &ctx.geom,
            &ctx.table_hyd_us,
            &ctx.table_hyd_ds,
        )
    } else {
        bridge_headwater_implicit_rhs(
            y_hw,
            y_tw,
            q_mag,
            pinned,
            &ctx.geom,
            &ctx.table_hyd_us,
            &ctx.table_hyd_ds,
        )
    }
}

fn map_residual_to_momentum_row(
    residual: &BridgeHeadwaterImplicitResidual,
    interval_i: usize,
    face_coupling: &BridgeIntervalCoupling,
) -> ImplicitMomentumRow {
    let mut m1 = 0.0;
    let mut m3 = 0.0;
    if face_coupling.hw_face == interval_i {
        m1 += residual.dr_dy_hw;
    } else if face_coupling.tw_face == interval_i {
        m1 += residual.dr_dy_tw;
    }
    if face_coupling.hw_face == interval_i + 1 {
        m3 += residual.dr_dy_hw;
    } else if face_coupling.tw_face == interval_i + 1 {
        m3 += residual.dr_dy_tw;
    }
    ImplicitMomentumRow {
        m1,
        m2: 0.0,
        m3,
        m4: 0.0,
        rhs: -residual.r,
    }
}

pub(crate) fn try_bridge_implicit_momentum_row(
    inputs: &UnsteadyInputs,
    raw_units: UnitSystem,
    b_idx: usize,
    interval_i: usize,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
) -> Option<ImplicitMomentumRow> {
    let ctx = build_bridge_implicit_context(
        inputs,
        b_idx,
        interval_i,
        raw_units,
        densified_stations,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
    )?;

    let q_user = q_metric_to_user(q_metric[interval_i], raw_units);
    let face_coupling = bridge_interval_coupling(interval_i, q_user, densified_tables.len());
    let residual = bridge_implicit_residual(
        &ctx,
        y_metric,
        q_metric,
        interval_i,
        raw_units,
        face_coupling.invert_tailwater,
    )?;

    Some(map_residual_to_momentum_row(
        &residual,
        interval_i,
        &face_coupling,
    ))
}

pub(crate) fn bridge_implicit_post_step_satisfied(
    inputs: &UnsteadyInputs,
    b_idx: usize,
    interval_i: usize,
    raw_units: UnitSystem,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
) -> bool {
    if !super::culvert_implicit::unsteady_coupling_is_implicit(inputs.unsteady_structure_coupling_mode) {
        return false;
    }
    let ctx = build_bridge_implicit_context(
        inputs,
        b_idx,
        interval_i,
        raw_units,
        densified_stations,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
    );
    let Some(ctx) = ctx else {
        return false;
    };
    let q_user = q_metric_to_user(q_metric[interval_i], raw_units);
    let face_coupling = bridge_interval_coupling(interval_i, q_user, densified_tables.len());
    let Some(residual) = bridge_implicit_residual(
        &ctx,
        y_metric,
        q_metric,
        interval_i,
        raw_units,
        face_coupling.invert_tailwater,
    ) else {
        return false;
    };
    let tol = if raw_units == UnitSystem::USCustomary {
        0.01 * crate::utils::FT_TO_M
    } else {
        0.003
    };
    residual.r.abs() <= tol
}

pub(crate) fn bridge_implicit_diagnostics(
    inputs: &UnsteadyInputs,
    b_idx: usize,
    interval_i: usize,
    raw_units: UnitSystem,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
) -> BridgeSolveResult {
    let b = &inputs.bridge;
    let interval_length_m =
        (densified_stations[interval_i] - densified_stations[interval_i + 1]).abs();
    let coupling = bridge_coupling_for(inputs, b_idx);
    let deck = bridge_deck_profile_for(inputs, b_idx, raw_units);
    let num_slices = inputs.num_slices.unwrap_or(100);
    let face_geo = bridge_face_geometry_for(
        inputs,
        b_idx,
        interval_i,
        raw_units,
        num_slices,
        densified_stations,
        densified_tables,
        densified_xs,
        densified_z_mins,
        interval_length_m,
    );
    let s = bridge_scalars(b, b_idx, raw_units);
    let q_user = q_metric_to_user(q_metric[interval_i], raw_units);
    let face_coupling = bridge_interval_coupling(interval_i, q_user, densified_tables.len());
    let tw_user = wsel_metric_to_user(y_metric[face_coupling.tw_face], raw_units);
    solve_bridge_coupled(
        q_user,
        s.low_chord,
        s.high_chord,
        s.pier_width,
        s.num_piers,
        s.pier_shape,
        s.weir_coeff,
        s.orifice_coeff,
        face_geo.z_down_user,
        face_geo.z_up_user,
        tw_user,
        raw_units,
        &face_geo.table_up,
        &face_geo.table_down,
        &coupling,
        interval_length_m,
        deck.as_ref(),
        Some(&face_geo.sections),
    )
}
