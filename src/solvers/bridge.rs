use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS, G_METRIC};
use crate::geometry::GeometryTable;

/// Supported pier shape types (Yarnell K coefficients per HEC-RAS).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PierShape {
    Square = 0,
    Semicircular = 1,
    TwinCylinder = 2,
    Triangular = 3,
}

impl PierShape {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => PierShape::Semicircular,
            2 => PierShape::TwinCylinder,
            3 => PierShape::Triangular,
            _ => PierShape::Square,
        }
    }

    /// Yarnell pier shape coefficient K (HEC-RAS Table: Pier Shape Yarnell K Coefficients).
    pub fn coefficient(&self) -> f64 {
        match self {
            PierShape::Square => 1.25,
            PierShape::Semicircular => 0.90,
            PierShape::TwinCylinder => 0.95, // twin-cylinder with connecting diaphragm
            PierShape::Triangular => 1.05,   // 90° triangular nose and tail
        }
    }
}

/// HEC-RAS Yarnell low-flow pier head loss (Class A): drop from section 3 to section 2.
///
/// H₃₋₂ = 2K(K + 10ω − 0.6)(α + 15α⁴) V²/(2g)
///
/// * ω = (V²/2g) / y — velocity head to depth ratio at the downstream section
/// * α = A_piers / (A_flow − A_piers) — pier obstruction over unobstructed flow area
pub fn yarnell_pier_head_loss(
    q_metric: f64,
    wsel_down_metric: f64,
    z_bed_down_metric: f64,
    pier_width_m: f64,
    num_piers: i32,
    pier_shape: PierShape,
    flow_area_m2: f64,
) -> f64 {
    if q_metric <= 1e-5 || flow_area_m2 <= 1e-5 {
        return 0.0;
    }

    let depth_down = (wsel_down_metric - z_bed_down_metric).max(0.0);
    if depth_down <= 1e-5 {
        return 0.0;
    }

    let a_piers = (num_piers as f64) * pier_width_m * depth_down;
    let a_unobstructed = (flow_area_m2 - a_piers).max(1e-5);
    let a_piers_clamped = a_piers.min(a_unobstructed * 0.9);
    let alpha = a_piers_clamped / a_unobstructed;

    let v_ds = q_metric / flow_area_m2;
    let velocity_head = (v_ds * v_ds) / (2.0 * G_METRIC);
    let omega = velocity_head / depth_down;
    let k = pier_shape.coefficient();

    2.0 * k * (k + 10.0 * omega - 0.6) * (alpha + 15.0 * alpha.powi(4)) * velocity_head
}

/// Solves the upstream water surface elevation (WSEL) for a bridge section
/// based on Yarnell's pier equation, orifice flow, or combined weir/overtopping flow.
pub fn solve_bridge_wsel(
    q: f64, // Flow rate in user units (cfs or cms)
    low_chord: f64, // lowest deck elevation (user units)
    high_chord: f64, // top of roadway elevation (user units)
    pier_width: f64, // single pier width (user units)
    num_piers: i32,
    pier_shape_type: i32,
    weir_coeff: f64, // weir discharge coefficient
    orifice_coeff: f64, // orifice coefficient Cd
    z_down: f64, // downstream bed elevation (user units)
    z_up: f64, // upstream bed elevation (user units)
    tw_wsel: f64, // downstream WSEL tailwater (user units)
    units: UnitSystem,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    // 1. Convert all inputs to Metric for consistent calculation
    let (q_metric, low_chord_m, high_chord_m, pier_width_m, tw_m, z_down_m, z_up_m, weir_coeff_m) = if units == UnitSystem::USCustomary {
        (
            q * CFS_TO_CMS,
            low_chord * FT_TO_M,
            high_chord * FT_TO_M,
            pier_width * FT_TO_M,
            tw_wsel * FT_TO_M,
            z_down * FT_TO_M,
            z_up * FT_TO_M,
            weir_coeff / 1.8113, // Convert Cw from US to Metric (divide by 1.8113)
        )
    } else {
        (q, low_chord, high_chord, pier_width, tw_wsel, z_down, z_up, weir_coeff)
    };

    let tw_clamped = tw_m.max(z_down_m + 1e-4);
    let pier_shape = PierShape::from_i32(pier_shape_type);

    // 2. Evaluate downstream conditions to decide if low flow or high flow holds
    let is_low_flow_initially = tw_clamped < low_chord_m;

    let wsel_up_metric = if is_low_flow_initially {
        // --- LOW FLOW: HEC-RAS Yarnell pier equation (Class A) ---
        let row_down = table_down.interpolate(tw_clamped);
        // Use channel_area when subdivided; falls back to total area for simple sections.
        let flow_area = if row_down.channel_area > 1e-6 {
            row_down.channel_area
        } else {
            row_down.area
        };

        if flow_area > 1e-5 && q_metric > 1e-5 {
            let hl_clamped = yarnell_pier_head_loss(
                q_metric,
                tw_clamped,
                z_down_m,
                pier_width_m,
                num_piers,
                pier_shape,
                flow_area,
            );
            let wsel_up_low = tw_clamped + hl_clamped;

            if wsel_up_low < low_chord_m {
                wsel_up_low
            } else {
                // Transitioned to pressure flow due to backing up above low-chord
                solve_high_flow(
                    q_metric,
                    low_chord_m,
                    high_chord_m,
                    pier_width_m,
                    num_piers,
                    weir_coeff_m,
                    orifice_coeff,
                    z_up_m,
                    tw_clamped,
                    table_up,
                )
            }
        } else {
            tw_clamped
        }
    } else {
        // --- HIGH FLOW: Pressure flow or Weir Overtopping ---
        solve_high_flow(
            q_metric,
            low_chord_m,
            high_chord_m,
            pier_width_m,
            num_piers,
            weir_coeff_m,
            orifice_coeff,
            z_up_m,
            tw_clamped,
            table_up,
        )
    };

    // 3. Convert result back to user units
    if units == UnitSystem::USCustomary {
        wsel_up_metric / FT_TO_M
    } else {
        wsel_up_metric
    }
}

/// Helper to solve upstream WSEL under pressure and/or weir overtopping flow (all parameters in Metric)
fn solve_high_flow(
    q_metric: f64,
    low_chord_m: f64,
    high_chord_m: f64,
    pier_width_m: f64,
    num_piers: i32,
    weir_coeff_m: f64,
    orifice_coeff: f64,
    z_up_m: f64,
    tw_clamped: f64,
    table_up: &GeometryTable,
) -> f64 {
    // Net opening area under the deck at the upstream section
    let row_up_low = table_up.interpolate(low_chord_m);
    let a_gross = row_up_low.area;
    let height_under_deck = (low_chord_m - z_up_m).max(0.0);
    let a_piers_deck = (num_piers as f64) * pier_width_m * height_under_deck;
    let a_net = (a_gross - a_piers_deck).max(1e-4);

    // 1. Calculate hypothetical pure pressure flow WSEL
    // Q = Cd * A_net * sqrt(2 * g * dH)  =>  dH = (Q / (Cd * A_net))^2 / (2 * g)
    let dh = (q_metric / (orifice_coeff * a_net)).powi(2) / (2.0 * G_METRIC);
    let wsel_up_pure = tw_clamped + dh;

    // If pure pressure flow is below the top of roadway, it does not overtop
    if wsel_up_pure < high_chord_m {
        return wsel_up_pure;
    }

    // 2. Weir overtopping combined with pressure flow
    // Q_total = Q_pressure + Q_weir
    // We solve for h_up (upstream WSEL) using bisection
    let l_weir = table_up.interpolate(high_chord_m).top_width.max(1.0);

    let residual = |h_up: f64| -> f64 {
        let h_weir = (h_up - high_chord_m).max(0.0);
        let q_weir = weir_coeff_m * l_weir * h_weir.powf(1.5);
        let q_pressure = orifice_coeff * a_net * (2.0 * G_METRIC * (h_up - tw_clamped).max(0.0)).sqrt();
        (q_pressure + q_weir) - q_metric
    };

    let mut low = high_chord_m;
    let mut high = high_chord_m + 50.0;
    let mut best_h = low;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let res = residual(mid);

        if res.abs() < 1e-8 {
            best_h = mid;
            break;
        }

        if res < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best_h = mid;
    }

    best_h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rectangular 10 m channel, WSEL = 3 m, Q = 15 cms, two 0.5 m square piers.
    /// Hand-checked against HEC-RAS Yarnell form: H₃₋₂ ≈ 0.00247 m.
    #[test]
    fn test_yarnell_pier_head_loss_hec_ras() {
        let hl = yarnell_pier_head_loss(
            15.0,
            3.0,
            0.0,
            0.5,
            2,
            PierShape::Square,
            30.0,
        );
        assert!(
            (hl - 0.00247).abs() < 1e-4,
            "Yarnell head loss should match HEC-RAS formula, got {hl}"
        );
    }

    #[test]
    fn test_yarnell_zero_piers_no_loss() {
        let hl = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 0, PierShape::Square, 30.0);
        assert_eq!(hl, 0.0);
    }

    #[test]
    fn test_yarnell_square_pier_loss_exceeds_semicircular() {
        let square = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::Square, 30.0);
        let semi = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::Semicircular, 30.0);
        assert!(square > semi, "Square piers (K=1.25) should produce more loss than semicircular (K=0.90)");
    }
}
