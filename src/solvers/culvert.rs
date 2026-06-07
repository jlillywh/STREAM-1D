use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS};

/// Standard acceleration due to gravity in English units (ft/s^2).
pub const G_ENGLISH: f64 = 32.17404856;

/// Supported culvert shapes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CulvertShape {
    Circular = 0,
    Box = 1,
    Arch = 2,
}

impl CulvertShape {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => CulvertShape::Box,
            2 => CulvertShape::Arch,
            _ => CulvertShape::Circular,
        }
    }
}

/// Computes the cross-sectional flow area (A) in sq ft for a given depth (y) in ft inside a culvert barrel.
pub fn get_culvert_area(shape: CulvertShape, span: f64, rise: f64, y: f64) -> f64 {
    if y <= 0.0 {
        return 0.0;
    }
    let d = rise; // Internal height (rise) of the barrel
    let y_clamp = y.min(d);

    match shape {
        CulvertShape::Circular => {
            // span is diameter D
            let r = d / 2.0;
            if y_clamp >= d {
                std::f64::consts::PI * r * r
            } else {
                let theta = 2.0 * (1.0 - y_clamp / r).acos();
                r * r * (theta - theta.sin()) / 2.0
            }
        }
        CulvertShape::Box => {
            // span is width W
            span * y_clamp
        }
        CulvertShape::Arch => {
            // Parabolic arch profile: area = 2/3 * W * D * (1 - (1 - y/D)^1.5)
            let w = span;
            if y_clamp >= d {
                (2.0 / 3.0) * w * d
            } else {
                (2.0 / 3.0) * w * d * (1.0 - (1.0 - y_clamp / d).powf(1.5))
            }
        }
    }
}

/// Computes the wetted top width (T) in ft for a given depth (y) in ft inside a culvert barrel.
pub fn get_culvert_top_width(shape: CulvertShape, span: f64, rise: f64, y: f64) -> f64 {
    if y <= 0.0 || y >= rise {
        return 0.0;
    }
    let d = rise;

    match shape {
        CulvertShape::Circular => {
            2.0 * (y * (d - y)).sqrt()
        }
        CulvertShape::Box => {
            span
        }
        CulvertShape::Arch => {
            // Parabolic top width: T(y) = W * sqrt(1 - y/D)
            span * (1.0 - y / d).sqrt()
        }
    }
}

/// Computes the wetted perimeter (P) in ft for a given depth (y) in ft inside a culvert barrel.
pub fn get_culvert_perimeter(shape: CulvertShape, span: f64, rise: f64, y: f64) -> f64 {
    if y <= 0.0 {
        return 0.0;
    }
    let d = rise;
    let y_clamp = y.min(d);

    match shape {
        CulvertShape::Circular => {
            if y_clamp >= d {
                std::f64::consts::PI * d
            } else {
                let theta = 2.0 * (1.0 - y_clamp / (d / 2.0)).acos();
                (d / 2.0) * theta
            }
        }
        CulvertShape::Box => {
            if y_clamp >= d {
                2.0 * span + 2.0 * d
            } else {
                span + 2.0 * y_clamp
            }
        }
        CulvertShape::Arch => {
            // Bottom width + arc length of the parabolic arch sides.
            if y_clamp >= d {
                // Full perimeter using parabolic arc length formula.
                // Parabola: y = D * (1 - 4x^2 / W^2)
                let w = span;
                let t = 4.0 * d / w;
                let arc_len = (w / 2.0) * ( (1.0 + t*t).sqrt() + (t + (1.0 + t*t).sqrt()).ln() / t );
                w + arc_len
            } else {
                let w = span;
                let t_width = get_culvert_top_width(shape, span, rise, y_clamp);
                w + 2.0 * (y_clamp * y_clamp + (w - t_width).powi(2) / 4.0).sqrt()
            }
        }
    }
}

/// Solves for critical depth (yc) in ft inside the culvert barrel.
pub fn solve_barrel_critical_depth(shape: CulvertShape, span: f64, rise: f64, q: f64) -> f64 {
    let d = rise;
    let mut low = 0.0;
    let mut high = d;
    let mut best_yc = 0.0;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let area = get_culvert_area(shape, span, rise, mid);
        let top_width = get_culvert_top_width(shape, span, rise, mid);

        if area < 1e-9 {
            low = mid;
            continue;
        }

        // Fr^2 = Q^2 * T / (g * A^3)
        let fr_sq = (q * q * top_width) / (G_ENGLISH * area.powi(3));
        if top_width < 1e-9 || fr_sq > 1.0 {
            // Depth too low (supercritical)
            low = mid;
        } else {
            // Depth too high (subcritical)
            high = mid;
        }
        best_yc = mid;
    }
    best_yc
}

/// Computes the upstream WSEL (in user units) for a culvert based on Inlet and Outlet Control comparisons.
pub fn solve_culvert_wsel(
    q: f64, // Flow rate in user units (cfs or cms)
    shape_type: i32,
    span: f64,
    rise: f64,
    roughness_n: f64,
    length: f64,
    entrance_loss_coeff: f64,
    exit_loss_coeff: f64,
    z_down: f64, // downstream invert (user units)
    z_up: f64,   // upstream invert (user units)
    tw_wsel: f64, // tailwater elevation (user units)
    units: UnitSystem,
) -> f64 {
    let shape = CulvertShape::from_i32(shape_type);

    // Convert inputs to English units for calculation
    let (q_cfs, span_ft, rise_ft, len_ft, z_down_ft, z_up_ft, tw_ft) = if units == UnitSystem::Metric {
        (
            q / CFS_TO_CMS,
            span / FT_TO_M,
            rise / FT_TO_M,
            length / FT_TO_M,
            z_down / FT_TO_M,
            z_up / FT_TO_M,
            tw_wsel / FT_TO_M,
        )
    } else {
        (q, span, rise, length, z_down, z_up, tw_wsel)
    };

    let d = rise_ft;
    let a_full = get_culvert_area(shape, span_ft, rise_ft, d);

    // 1. INLET CONTROL CALCULATIONS
    // Bisection search for critical depth inside barrel in feet
    let yc = solve_barrel_critical_depth(shape, span_ft, rise_ft, q_cfs);
    let ac = get_culvert_area(shape, span_ft, rise_ft, yc);
    let vc = if ac > 1e-9 { q_cfs / ac } else { 0.0 };
    let hc = yc + (vc * vc) / (2.0 * G_ENGLISH); // Specific head at critical depth

    // Select Inlet Control Coefficients based on shape and Ke
    let (k, m, c, y) = match shape {
        CulvertShape::Circular => {
            if entrance_loss_coeff <= 0.2 {
                // Groove end with headwall
                (0.0018, 2.0, 0.0292, 0.74)
            } else {
                // Square edge with headwall
                (0.0098, 2.0, 0.0398, 0.67)
            }
        }
        CulvertShape::Box => {
            if entrance_loss_coeff <= 0.2 {
                // Flared wingwalls
                (0.026, 1.0, 0.0347, 0.81)
            } else {
                // Square edge 90 deg
                (0.061, 0.75, 0.0400, 0.80)
            }
        }
        CulvertShape::Arch => {
            if entrance_loss_coeff <= 0.2 {
                // Smooth entry
                (0.0083, 2.0, 0.0374, 0.69)
            } else {
                // Projecting entry
                (0.0300, 1.5, 0.0500, 0.60)
            }
        }
    };

    let culv_slope = ((z_up_ft - z_down_ft) / len_ft).max(0.0);
    let f_param = q_cfs / (a_full * d.sqrt());

    // Unsubmerged Eq (Form 1)
    let hw_d_unsub = (hc / d) + k * f_param.powf(m) - 0.5 * culv_slope;
    // Submerged Eq
    let hw_d_sub = c * f_param.powi(2) + y - 0.5 * culv_slope;

    // Transition between unsubmerged (F <= 3.0) and submerged (F >= 4.0)
    let hw_d = if f_param <= 3.0 {
        hw_d_unsub
    } else if f_param >= 4.0 {
        hw_d_sub
    } else {
        let t = (f_param - 3.0) / (4.0 - 3.0);
        (1.0 - t) * hw_d_unsub + t * hw_d_sub
    };

    let hw_inlet = (hw_d * d).max(hc);
    let wsel_inlet = z_up_ft + hw_inlet;

    // 2. OUTLET CONTROL CALCULATIONS
    let _tw_depth = (tw_ft - z_down_ft).max(0.0);
    let v_full = if a_full > 1e-9 { q_cfs / a_full } else { 0.0 };

    // Entrance loss (he = Ke * V^2 / 2g)
    let he = entrance_loss_coeff * (v_full * v_full) / (2.0 * G_ENGLISH);
    // Exit loss (ho = Kx * V^2 / 2g)
    let ho = exit_loss_coeff * (v_full * v_full) / (2.0 * G_ENGLISH);

    // Friction loss (hf = L * Sf)
    let p_full = get_culvert_perimeter(shape, span_ft, rise_ft, d);
    let r_full = if p_full > 1e-9 { a_full / p_full } else { 0.0 };
    let sf = if a_full > 1e-9 && r_full > 1e-9 {
        (q_cfs * roughness_n / (1.486 * a_full * r_full.powf(2.0 / 3.0))).powi(2)
    } else {
        0.0
    };
    let hf = len_ft * sf;

    // Total head loss
    let total_losses = he + hf + ho;
    let wsel_outlet = tw_ft + total_losses;

    // 3. SELECTION & TRANSITION
    let wsel_up_ft = wsel_inlet.max(wsel_outlet);

    // Convert back to original unit system
    if units == UnitSystem::Metric {
        wsel_up_ft * FT_TO_M
    } else {
        wsel_up_ft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_areas() {
        // Box area
        assert_eq!(get_culvert_area(CulvertShape::Box, 6.0, 4.0, 2.0), 12.0);
        assert_eq!(get_culvert_area(CulvertShape::Box, 6.0, 4.0, 5.0), 24.0); // clamped to rise

        // Circular area full
        let area_circ_full = get_culvert_area(CulvertShape::Circular, 5.0, 5.0, 5.0);
        let expected_circ_full = std::f64::consts::PI * 2.5 * 2.5;
        assert!((area_circ_full - expected_circ_full).abs() < 1e-6);

        // Circular area half full (should be exactly half of full area)
        let area_circ_half = get_culvert_area(CulvertShape::Circular, 5.0, 5.0, 2.5);
        assert!((area_circ_half - expected_circ_full / 2.0).abs() < 1e-6);

        // Arch area full (2/3 * W * D)
        let area_arch_full = get_culvert_area(CulvertShape::Arch, 6.0, 4.0, 4.0);
        assert!((area_arch_full - 16.0).abs() < 1e-6);
    }

    #[test]
    fn test_shape_perimeters() {
        // Box perimeter
        assert_eq!(get_culvert_perimeter(CulvertShape::Box, 6.0, 4.0, 2.0), 10.0); // W + 2y
        assert_eq!(get_culvert_perimeter(CulvertShape::Box, 6.0, 4.0, 4.0), 20.0); // 2W + 2D

        // Circular perimeter
        let p_circ_full = get_culvert_perimeter(CulvertShape::Circular, 5.0, 5.0, 5.0);
        assert!((p_circ_full - std::f64::consts::PI * 5.0).abs() < 1e-6);
        let p_circ_half = get_culvert_perimeter(CulvertShape::Circular, 5.0, 5.0, 2.5);
        assert!((p_circ_half - std::f64::consts::PI * 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_circular_culvert_benchmark() {
        // Benchmark test case described in implementation plan:
        // Concrete circular pipe: D = 5.0 ft, L = 100 ft, Q = 100 cfs, slope = 0.01
        // z_down = 9.0 ft, z_up = 10.0 ft, Manning's n = 0.012, Ke = 0.5, exit Kx = 1.0.
        // Let's check under low tailwater (inlet control dominates)
        let tw_low = 12.0; // depth = 3.0 ft
        let wsel_up_low = solve_culvert_wsel(
            100.0,
            0, // Circular
            5.0,
            5.0,
            0.012,
            100.0,
            0.5,
            1.0,
            9.0,
            10.0,
            tw_low,
            UnitSystem::USCustomary,
        );

        // Under inlet control, HW depth above inlet invert is ~4.25 ft.
        // So WSEL_up should be ~14.25 ft. Let's verify.
        let hw_depth_low = wsel_up_low - 10.0;
        assert!((hw_depth_low - 4.25).abs() < 0.05, "expected ~4.25, got {}", hw_depth_low);

        // Now test under high tailwater (outlet control dominates)
        let tw_high = 15.0; // depth = 6.0 ft
        let wsel_up_high = solve_culvert_wsel(
            100.0,
            0,
            5.0,
            5.0,
            0.012,
            100.0,
            0.5,
            1.0,
            9.0,
            10.0,
            tw_high,
            UnitSystem::USCustomary,
        );

        // Under outlet control, WSEL_up should be TW + losses = 15.0 + 0.726 = 15.726 ft.
        let hw_depth_high = wsel_up_high - 10.0;
        assert!((hw_depth_high - 5.73).abs() < 0.05, "expected ~5.73, got {}", hw_depth_high);
    }
}
