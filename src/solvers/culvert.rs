use crate::utils::{UnitSystem, CFS_TO_CMS, FT_TO_M};

/// Standard acceleration due to gravity in English units (ft/s^2).
pub const G_ENGLISH: f64 = 32.17404856;

/// Supported culvert shapes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CulvertShape {
    Circular = 0,
    Box = 1,
    Arch = 2,
    ConspanArch = 3,
    /// Corrugated metal pipe-arch: vertical legs + circular crown (FHWA-style).
    PipeArch = 4,
    /// Horizontal ellipse; span = major axis, rise = minor axis.
    Elliptical = 5,
    /// Horseshoe: circular invert + vertical legs + circular crown.
    Horseshoe = 6,
    /// User-defined custom shape.
    Custom = 7,
}

impl CulvertShape {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => CulvertShape::Box,
            2 => CulvertShape::Arch,
            3 => CulvertShape::ConspanArch,
            4 => CulvertShape::PipeArch,
            5 => CulvertShape::Elliptical,
            6 => CulvertShape::Horseshoe,
            7 => CulvertShape::Custom,
            _ => CulvertShape::Circular,
        }
    }
}

/// Helper to estimate default arch height for standard ConSpan spans using linear interpolation
pub fn get_conspan_arch_height(span: f64) -> f64 {
    if span <= 12.0 {
        3.07
    } else if span <= 14.0 {
        3.07 + (span - 12.0) * (3.00 - 3.07) / (14.0 - 12.0)
    } else if span <= 16.0 {
        3.00 + (span - 14.0) * (3.53 - 3.00) / (16.0 - 14.0)
    } else if span <= 20.0 {
        3.53 + (span - 16.0) * (4.13 - 3.53) / (20.0 - 16.0)
    } else if span <= 24.0 {
        4.13 + (span - 20.0) * (4.93 - 4.13) / (24.0 - 20.0)
    } else if span <= 28.0 {
        4.93 + (span - 24.0) * (5.76 - 4.93) / (28.0 - 24.0)
    } else if span <= 32.0 {
        5.76 + (span - 28.0) * (6.51 - 5.76) / (32.0 - 28.0)
    } else if span <= 36.0 {
        6.51 + (span - 32.0) * (7.21 - 6.51) / (36.0 - 32.0)
    } else {
        7.21
    }
}

struct ConspanTableEntry {
    y: f64,
    area: f64,
    perimeter: f64,
    top_width: f64,
}

const CONSPAN_28X6_TABLE: [ConspanTableEntry; 27] = [
    ConspanTableEntry {
        y: 0.0,
        area: 0.0,
        perimeter: 28.0,
        top_width: 28.0,
    },
    ConspanTableEntry {
        y: 0.25,
        area: 7.0,
        perimeter: 28.5,
        top_width: 28.0,
    },
    ConspanTableEntry {
        y: 0.74,
        area: 20.733,
        perimeter: 29.48,
        top_width: 27.94,
    },
    ConspanTableEntry {
        y: 1.224,
        area: 34.185,
        perimeter: 30.466,
        top_width: 27.76,
    },
    ConspanTableEntry {
        y: 1.693,
        area: 47.134,
        perimeter: 31.45,
        top_width: 27.462,
    },
    ConspanTableEntry {
        y: 2.14,
        area: 59.318,
        perimeter: 32.435,
        top_width: 27.05,
    },
    ConspanTableEntry {
        y: 2.558,
        area: 70.517,
        perimeter: 33.417,
        top_width: 26.534,
    },
    ConspanTableEntry {
        y: 2.942,
        area: 80.588,
        perimeter: 34.402,
        top_width: 25.918,
    },
    ConspanTableEntry {
        y: 3.284,
        area: 89.331,
        perimeter: 35.385,
        top_width: 25.212,
    },
    ConspanTableEntry {
        y: 3.58,
        area: 96.676,
        perimeter: 36.374,
        top_width: 24.42,
    },
    ConspanTableEntry {
        y: 3.822,
        area: 102.494,
        perimeter: 37.278,
        top_width: 23.656,
    },
    ConspanTableEntry {
        y: 4.047,
        area: 107.728,
        perimeter: 38.184,
        top_width: 22.87,
    },
    ConspanTableEntry {
        y: 4.253,
        area: 112.356,
        perimeter: 39.089,
        top_width: 22.064,
    },
    ConspanTableEntry {
        y: 4.441,
        area: 116.427,
        perimeter: 39.995,
        top_width: 21.242,
    },
    ConspanTableEntry {
        y: 4.61,
        area: 119.945,
        perimeter: 40.90,
        top_width: 20.4,
    },
    ConspanTableEntry {
        y: 4.76,
        area: 122.941,
        perimeter: 41.805,
        top_width: 19.546,
    },
    ConspanTableEntry {
        y: 4.89,
        area: 125.426,
        perimeter: 42.71,
        top_width: 18.68,
    },
    ConspanTableEntry {
        y: 4.996,
        area: 127.36,
        perimeter: 43.605,
        top_width: 17.812,
    },
    ConspanTableEntry {
        y: 5.206,
        area: 130.895,
        perimeter: 45.602,
        top_width: 15.858,
    },
    ConspanTableEntry {
        y: 5.392,
        area: 133.662,
        perimeter: 47.597,
        top_width: 13.898,
    },
    ConspanTableEntry {
        y: 5.553,
        area: 135.741,
        perimeter: 49.593,
        top_width: 11.928,
    },
    ConspanTableEntry {
        y: 5.689,
        area: 137.229,
        perimeter: 51.588,
        top_width: 9.952,
    },
    ConspanTableEntry {
        y: 5.801,
        area: 138.233,
        perimeter: 53.584,
        top_width: 7.968,
    },
    ConspanTableEntry {
        y: 5.888,
        area: 138.839,
        perimeter: 55.578,
        top_width: 5.982,
    },
    ConspanTableEntry {
        y: 5.95,
        area: 139.149,
        perimeter: 57.574,
        top_width: 3.99,
    },
    ConspanTableEntry {
        y: 5.988,
        area: 139.262,
        perimeter: 59.569,
        top_width: 1.996,
    },
    ConspanTableEntry {
        y: 6.0,
        area: 139.274,
        perimeter: 61.565,
        top_width: 0.0,
    },
];

fn interpolate_conspan(span: f64, rise: f64, y: f64, field: &str) -> f64 {
    if y <= 0.0 {
        return if field == "perimeter" { span } else { 0.0 };
    }

    // Scale depth to 28x6 nominal table
    let y_norm = (y * (6.0 / rise)).min(6.0);

    // Find interval in the table
    let mut idx = 0;
    for i in 0..CONSPAN_28X6_TABLE.len() - 1 {
        if y_norm >= CONSPAN_28X6_TABLE[i].y && y_norm <= CONSPAN_28X6_TABLE[i + 1].y {
            idx = i;
            break;
        }
    }

    let t = (y_norm - CONSPAN_28X6_TABLE[idx].y)
        / (CONSPAN_28X6_TABLE[idx + 1].y - CONSPAN_28X6_TABLE[idx].y);

    match field {
        "area" => {
            let val_norm =
                (1.0 - t) * CONSPAN_28X6_TABLE[idx].area + t * CONSPAN_28X6_TABLE[idx + 1].area;
            val_norm * (span / 28.0) * (rise / 6.0)
        }
        "perimeter" => {
            let val_norm = (1.0 - t) * CONSPAN_28X6_TABLE[idx].perimeter
                + t * CONSPAN_28X6_TABLE[idx + 1].perimeter;
            // scale the arch part and add the scaled bottom
            (val_norm - 28.0) * (rise / 6.0) + span
        }
        "top_width" => {
            if y >= rise {
                0.0
            } else {
                let val_norm = (1.0 - t) * CONSPAN_28X6_TABLE[idx].top_width
                    + t * CONSPAN_28X6_TABLE[idx + 1].top_width;
                val_norm * (span / 28.0)
            }
        }
        _ => 0.0,
    }
}

const GEOM_SLICES: usize = 64;

/// Pipe-arch crown depth (spring line to crown) from span and rise.
fn pipe_arch_crown_depth(span: f64, rise: f64) -> f64 {
    let min = span / 16.0;
    let max = rise * 0.5;
    if min > max {
        max
    } else {
        (rise * 0.375).clamp(min, max)
    }
}

/// Horseshoe invert and crown arc depths derived from span/rise.
fn horseshoe_invert_depth(span: f64, rise: f64) -> f64 {
    (span / 2.0).min(rise * 0.45)
}

fn horseshoe_crown_depth(span: f64, rise: f64, invert_depth: f64) -> f64 {
    let min = span / 8.0;
    let max = rise - invert_depth - 0.01;
    if min > max {
        max.max(0.0)
    } else {
        ((rise - invert_depth) * 0.35).clamp(min, max)
    }
}

/// Top water width at depth `y` (ft above invert) for extended shapes.
fn extended_shape_width(shape: CulvertShape, span: f64, rise: f64, y: f64) -> f64 {
    if y <= 0.0 || span <= 0.0 || rise <= 0.0 {
        return 0.0;
    }
    let y_clamp = y.min(rise);

    match shape {
        CulvertShape::Elliptical => {
            let a = span / 2.0;
            let b = rise / 2.0;
            let yc = y_clamp - b;
            if yc.abs() >= b {
                return 0.0;
            }
            2.0 * a * (1.0 - (yc / b).powi(2)).max(0.0).sqrt()
        }
        CulvertShape::PipeArch => {
            let crown_h = pipe_arch_crown_depth(span, rise);
            let spring = rise - crown_h;
            if y_clamp <= spring {
                return span;
            }
            let yi = y_clamp - spring;
            if yi >= crown_h {
                return 0.0;
            }
            let r = span * span / (8.0 * crown_h) + crown_h / 2.0;
            (2.0 * (2.0 * r * yi - yi * yi).max(0.0).sqrt()).min(span)
        }
        CulvertShape::Horseshoe => {
            let h_i = horseshoe_invert_depth(span, rise);
            let h_c = horseshoe_crown_depth(span, rise, h_i);
            let spring_top = rise - h_c;
            if y_clamp <= h_i {
                let ri = span / 2.0;
                let dy = ri - y_clamp;
                if dy.abs() >= ri {
                    return 0.0;
                }
                2.0 * (ri * ri - dy * dy).max(0.0).sqrt()
            } else if y_clamp <= spring_top {
                span
            } else {
                let yi = y_clamp - spring_top;
                if yi >= h_c {
                    return 0.0;
                }
                let r = span * span / (8.0 * h_c) + h_c / 2.0;
                (2.0 * (2.0 * r * yi - yi * yi).max(0.0).sqrt()).min(span)
            }
        }
        _ => 0.0,
    }
}

fn integrate_partial_area(width_at: impl Fn(f64) -> f64, y: f64) -> f64 {
    if y <= 0.0 {
        return 0.0;
    }
    let dy = y / GEOM_SLICES as f64;
    let mut sum = 0.0;
    for i in 0..GEOM_SLICES {
        let y0 = i as f64 * dy;
        let y1 = (i + 1) as f64 * dy;
        sum += 0.5 * (width_at(y0) + width_at(y1)) * dy;
    }
    sum
}

fn integrate_partial_perimeter(
    width_at: impl Fn(f64) -> f64,
    y: f64,
    include_flat_bottom: bool,
) -> f64 {
    if y <= 0.0 {
        return 0.0;
    }
    let dy = y / GEOM_SLICES as f64;
    let mut p = if include_flat_bottom {
        width_at(0.0)
    } else {
        0.0
    };
    for i in 0..GEOM_SLICES {
        let y0 = i as f64 * dy;
        let y1 = (i + 1) as f64 * dy;
        let w0 = width_at(y0);
        let w1 = width_at(y1);
        let dw = (w1 - w0) / 2.0;
        p += 2.0 * (dy * dy + dw * dw).sqrt();
    }
    p
}

fn extended_shape_area(shape: CulvertShape, span: f64, rise: f64, y: f64) -> f64 {
    let y_clamp = y.min(rise);
    integrate_partial_area(|d| extended_shape_width(shape, span, rise, d), y_clamp)
}

fn extended_shape_perimeter(shape: CulvertShape, span: f64, rise: f64, y: f64) -> f64 {
    let y_clamp = y.min(rise);
    let flat_bottom = matches!(shape, CulvertShape::PipeArch);
    integrate_partial_perimeter(
        |d| extended_shape_width(shape, span, rise, d),
        y_clamp,
        flat_bottom,
    )
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
        CulvertShape::ConspanArch => interpolate_conspan(span, rise, y_clamp, "area"),
        CulvertShape::PipeArch | CulvertShape::Elliptical | CulvertShape::Horseshoe => {
            extended_shape_area(shape, span, rise, y_clamp)
        }
        CulvertShape::Custom => 0.0,
    }
}

/// Computes the wetted top width (T) in ft for a given depth (y) in ft inside a culvert barrel.
pub fn get_culvert_top_width(shape: CulvertShape, span: f64, rise: f64, y: f64) -> f64 {
    if y <= 0.0 || y >= rise {
        return 0.0;
    }
    let d = rise;

    match shape {
        CulvertShape::Circular => 2.0 * (y * (d - y)).sqrt(),
        CulvertShape::Box => span,
        CulvertShape::Arch => {
            // Parabolic top width: T(y) = W * sqrt(1 - y/D)
            span * (1.0 - y / d).sqrt()
        }
        CulvertShape::ConspanArch => interpolate_conspan(span, rise, y, "top_width"),
        CulvertShape::PipeArch | CulvertShape::Elliptical | CulvertShape::Horseshoe => {
            extended_shape_width(shape, span, rise, y)
        }
        CulvertShape::Custom => 0.0,
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
                let arc_len =
                    (w / 2.0) * ((1.0 + t * t).sqrt() + (t + (1.0 + t * t).sqrt()).ln() / t);
                w + arc_len
            } else {
                let w = span;
                let t_width = get_culvert_top_width(shape, span, rise, y_clamp);
                w + 2.0 * (y_clamp * y_clamp + (w - t_width).powi(2) / 4.0).sqrt()
            }
        }
        CulvertShape::ConspanArch => interpolate_conspan(span, rise, y_clamp, "perimeter"),
        CulvertShape::PipeArch | CulvertShape::Elliptical | CulvertShape::Horseshoe => {
            extended_shape_perimeter(shape, span, rise, y_clamp)
        }
        CulvertShape::Custom => 0.0,
    }
}

/// Computes the effective flow area (A) in sq ft for a given depth (y) in ft inside a culvert barrel, accounting for blockage.
pub fn get_culvert_effective_area(
    shape: CulvertShape,
    span: f64,
    rise: f64,
    y: f64,
    depth_blocked: f64,
) -> f64 {
    let d_b = depth_blocked.min(rise);
    if y <= d_b {
        0.0
    } else {
        get_culvert_area(shape, span, rise, y) - get_culvert_area(shape, span, rise, d_b)
    }
}

/// Computes the effective wetted top width (T) in ft for a given depth (y) in ft inside a culvert barrel, accounting for blockage.
pub fn get_culvert_effective_top_width(
    shape: CulvertShape,
    span: f64,
    rise: f64,
    y: f64,
    depth_blocked: f64,
) -> f64 {
    let d_b = depth_blocked.min(rise);
    if y <= d_b {
        0.0
    } else {
        get_culvert_top_width(shape, span, rise, y)
    }
}

/// Computes the effective wetted perimeter (P) in ft for a given depth (y) in ft inside a culvert barrel, accounting for blockage.
pub fn get_culvert_effective_perimeter(
    shape: CulvertShape,
    span: f64,
    rise: f64,
    y: f64,
    depth_blocked: f64,
) -> f64 {
    let d_b = depth_blocked.min(rise);
    if y <= d_b {
        0.0
    } else {
        let y_clamp = y.min(rise);
        let p_y = get_culvert_perimeter(shape, span, rise, y_clamp);
        let p_b = get_culvert_perimeter(shape, span, rise, d_b);
        let t_b = get_culvert_top_width(shape, span, rise, d_b);
        (p_y - p_b) + t_b
    }
}

/// Computes the composite Manning's n roughness coefficient for a given depth (y) in ft inside a culvert barrel.
pub fn get_culvert_composite_n(
    shape: CulvertShape,
    span: f64,
    rise: f64,
    y: f64,
    depth_blocked: f64,
    n_top: f64,
    n_bottom: f64,
    depth_bottom_n: f64,
) -> f64 {
    let d_b = depth_blocked.min(rise);
    let d_n = depth_bottom_n.min(rise);
    if d_n <= d_b || (n_bottom - n_top).abs() < 1e-9 {
        return n_top;
    }
    if y <= d_b {
        return n_bottom;
    }
    if y <= d_n {
        return n_bottom;
    }
    let p_bottom = get_culvert_effective_perimeter(shape, span, rise, d_n, d_b);
    let y_clamp = y.min(rise);
    let p_y = get_culvert_perimeter(shape, span, rise, y_clamp);
    let p_n = get_culvert_perimeter(shape, span, rise, d_n);
    let p_top = (p_y - p_n).max(0.0);
    let p_total = p_bottom + p_top;
    if p_total > 1e-9 {
        ((p_bottom * n_bottom.powf(1.5) + p_top * n_top.powf(1.5)) / p_total).powf(2.0 / 3.0)
    } else {
        n_top
    }
}

/// Solves for critical depth (yc) in ft inside the culvert barrel.
pub fn solve_barrel_critical_depth(
    shape: CulvertShape,
    span: f64,
    rise: f64,
    q: f64,
    depth_blocked: f64,
) -> f64 {
    let d_b = depth_blocked.min(rise);
    let mut low = d_b;
    let mut high = rise;
    let mut best_yc = d_b;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let area = get_culvert_effective_area(shape, span, rise, mid, d_b);
        let top_width = get_culvert_effective_top_width(shape, span, rise, mid, d_b);

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

/// Result of a culvert headwater solve.
#[derive(Debug, Clone, PartialEq)]
pub struct CulvertSolveResult {
    /// Upstream water surface elevation (user units).
    pub wsel: f64,
    /// Controlling mechanism: `"inlet"`, `"outlet"`, or `"overtopping"`.
    pub control_type: String,
    /// Headwater from inlet-control nomograph (user units).
    pub wsel_inlet: f64,
    /// Headwater from outlet-control energy balance (user units).
    pub wsel_outlet: f64,
    /// Total discharge through barrel(s) (user units).
    pub q_barrel: f64,
    /// Discharge over roadway weir when overtopping is modeled (user units).
    pub q_weir: f64,
    /// Flow depth inside barrel at downstream end (user units, above downstream invert).
    pub barrel_depth: f64,
    /// Mean velocity inside barrel (user units).
    pub barrel_velocity: f64,
    /// Froude number inside barrel (based on hydraulic depth).
    pub barrel_froude: f64,
}

/// Headwater rating curve for a single culvert at fixed tailwater.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CulvertRatingCurveInputs {
    pub q_values: Vec<f64>,
    /// Culvert geometry, losses, tailwater (`tw_wsel`); field `q` is ignored.
    #[serde(flatten)]
    pub culvert: CulvertSolveParams,
}

/// Headwater vs discharge samples for one culvert.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CulvertRatingCurveResult {
    pub q: Vec<f64>,
    pub wsel: Vec<f64>,
    pub control_types: Vec<String>,
    pub wsel_inlet: Vec<f64>,
    pub wsel_outlet: Vec<f64>,
    pub q_barrel: Vec<f64>,
    pub q_weir: Vec<f64>,
    pub barrel_depth: Vec<f64>,
    pub barrel_velocity: Vec<f64>,
    pub barrel_froude: Vec<f64>,
}

/// Parameters for a culvert headwater solve (user units unless noted).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CulvertSolveParams {
    #[serde(default)]
    pub q: f64,
    pub shape_type: i32,
    /// Inlet type for FHWA nomograph (0 = legacy Ke threshold).
    #[serde(default)]
    pub inlet_type: i32,
    pub span: f64,
    pub rise: f64,
    pub roughness_n: f64,
    pub length: f64,
    pub entrance_loss_coeff: f64,
    pub exit_loss_coeff: f64,
    pub z_down: f64,
    pub z_up: f64,
    pub tw_wsel: f64,
    pub units: UnitSystem,
    #[serde(default)]
    pub manning_n_bottom: f64,
    #[serde(default)]
    pub depth_bottom_n: f64,
    #[serde(default)]
    pub depth_blocked: f64,
    #[serde(default)]
    pub ds_velocity: f64,
    #[serde(default)]
    pub us_velocity: f64,
    /// Roadway/embankment crest elevation for overtopping weir (optional).
    pub crest_elev: Option<f64>,
    /// Weir discharge coefficient (default 2.6 US / 1.44 metric).
    #[serde(default)]
    pub weir_coeff: f64,
    /// Effective weir length for overtopping (default span × num_barrels).
    #[serde(default)]
    pub weir_length: f64,
    #[serde(default = "default_num_barrels")]
    pub num_barrels: i32,
    /// Open barrels carrying flow (≤ `num_barrels`). Zero = use all barrels.
    #[serde(default)]
    pub active_barrels: i32,
    /// Skew angle in degrees from normal to channel flow (0 = perpendicular). Clamped to 59°.
    #[serde(default)]
    pub skew_deg: f64,
    /// Per-barrel span/diameter (length = active barrels). Omit entries to use `span`.
    #[serde(default)]
    pub barrel_spans: Option<Vec<f64>>,
    /// Per-barrel rise (length = active barrels). Omit entries to use `rise`.
    #[serde(default)]
    pub barrel_rises: Option<Vec<f64>>,
    /// Custom shape table - elevation / depth above invert
    #[serde(default)]
    pub custom_shape_tbl_y: Option<Vec<f64>>,
    /// Custom shape table - wetted area
    #[serde(default)]
    pub custom_shape_tbl_area: Option<Vec<f64>>,
    /// Custom shape table - wetted perimeter
    #[serde(default)]
    pub custom_shape_tbl_perimeter: Option<Vec<f64>>,
    /// Custom shape table - top water width
    #[serde(default)]
    pub custom_shape_tbl_top_width: Option<Vec<f64>>,
    /// Roadway profile stations (optional).
    #[serde(default)]
    pub roadway_stations: Option<Vec<f64>>,
    /// Roadway profile elevations (optional).
    #[serde(default)]
    pub roadway_elevations: Option<Vec<f64>>,
    /// FHWA HDS-5 chart number (optional).
    #[serde(default)]
    pub chart_number: Option<i32>,
    /// FHWA HDS-5 scale number (optional).
    #[serde(default)]
    pub scale_number: Option<i32>,
    /// Tapered inlet type (0 = None, 1 = Side-Tapered, 2 = Slope-Tapered).
    #[serde(default)]
    pub tapered_type: i32,
    /// Tapered inlet face span/width (optional).
    pub tapered_face_span: Option<f64>,
    /// Tapered inlet face rise/height (optional).
    pub tapered_face_rise: Option<f64>,
    /// Tapered inlet vertical drop (fall) from face/crest to throat (optional).
    #[serde(default)]
    pub tapered_fall: f64,
    /// Tapered inlet crest weir length for overtopping crest control (optional).
    pub tapered_crest_weir_length: Option<f64>,
    /// Tapered inlet crest discharge coefficient (optional).
    pub tapered_crest_weir_coeff: Option<f64>,
    /// Tapered inlet face control FHWA HDS-5 chart number (optional).
    pub tapered_face_chart_number: Option<i32>,
    /// Tapered inlet face control FHWA HDS-5 scale number (optional).
    pub tapered_face_scale_number: Option<i32>,
    /// Tapered inlet throat control FHWA HDS-5 chart number (optional).
    pub tapered_throat_chart_number: Option<i32>,
    /// Tapered inlet throat control FHWA HDS-5 scale number (optional).
    pub tapered_throat_scale_number: Option<i32>,
}

fn default_num_barrels() -> i32 {
    1
}

/// HEC-RAS-style skew: projected inlet span × cos(θ), friction length ÷ cos(θ).
// Note: Handled as actual physical dimensions for culvert barrels to align with HEC-RAS.
pub fn apply_barrel_skew(_skew_deg: f64, span_ft: f64, len_ft: f64) -> (f64, f64) {
    (span_ft, len_ft)
}

pub(crate) fn normalize_culvert_params(params: &mut CulvertSolveParams) {
    if params.manning_n_bottom == 0.0 {
        params.manning_n_bottom = params.roughness_n;
    }
    if params.num_barrels < 1 {
        params.num_barrels = 1;
    }
    if params.active_barrels < 1 || params.active_barrels > params.num_barrels {
        params.active_barrels = params.num_barrels;
    }
}

fn resolve_barrel_geometries(params: &CulvertSolveParams) -> Vec<(f64, f64)> {
    let n = params.active_barrels as usize;
    (0..n)
        .map(|i| {
            let span = params
                .barrel_spans
                .as_ref()
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(params.span);
            let rise = params
                .barrel_rises
                .as_ref()
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(params.rise);
            (span, rise)
        })
        .collect()
}

fn barrel_params_for_geometry(
    base: &CulvertSolveParams,
    span: f64,
    rise: f64,
) -> CulvertSolveParams {
    let mut p = base.clone();
    p.span = span;
    p.rise = rise;
    p.num_barrels = 1;
    p.active_barrels = 1;
    p.barrel_spans = None;
    p.barrel_rises = None;
    p
}

fn use_multi_barrel_solve(params: &CulvertSolveParams) -> bool {
    params.active_barrels > 1 || params.barrel_spans.is_some() || params.barrel_rises.is_some()
}

fn estimate_max_barrel_q(params: &CulvertSolveParams) -> f64 {
    let (span_ft, rise_ft, db_ft) = if params.units == UnitSystem::Metric {
        (
            params.span / FT_TO_M,
            params.rise / FT_TO_M,
            params.depth_blocked / FT_TO_M,
        )
    } else {
        (params.span, params.rise, params.depth_blocked)
    };
    let (span_ft, _) = apply_barrel_skew(params.skew_deg, span_ft, 1.0);
    let geom = CulvertGeometry::new(params, span_ft, rise_ft);
    let a_full = geom.effective_area(rise_ft, db_ft);
    let q_cfs = a_full * (2.0 * G_ENGLISH * rise_ft.max(0.5)).sqrt();
    if params.units == UnitSystem::Metric {
        q_cfs * CFS_TO_CMS
    } else {
        q_cfs
    }
}

pub(crate) fn barrel_q_for_wsel(params: &CulvertSolveParams, target_wsel: f64) -> f64 {
    if target_wsel <= params.tw_wsel + 1e-9 {
        return 0.0;
    }
    let mut low = 0.0;
    let mut high = estimate_max_barrel_q(params).max(1.0);
    for _ in 0..45 {
        let mid = 0.5 * (low + high);
        let hw = solve_culvert_barrel_internal(params, mid).wsel;
        if hw < target_wsel - 1e-6 {
            low = mid;
        } else {
            high = mid;
        }
    }
    0.5 * (low + high)
}

pub(crate) fn default_weir_length_user(params: &CulvertSolveParams) -> f64 {
    let geoms = resolve_barrel_geometries(params);
    if params.units == UnitSystem::Metric {
        geoms
            .iter()
            .map(|(span, _)| {
                let span_ft = span / FT_TO_M;
                let len_ft = params.length / FT_TO_M;
                let (span_eff, _) = apply_barrel_skew(params.skew_deg, span_ft, len_ft);
                span_eff * FT_TO_M
            })
            .sum()
    } else {
        geoms
            .iter()
            .map(|(span, _)| {
                let (span_eff, _) = apply_barrel_skew(params.skew_deg, *span, params.length);
                span_eff
            })
            .sum()
    }
}

fn solve_multi_barrel_barrels(base: &CulvertSolveParams, q_total: f64) -> BarrelSolveInternal {
    let geoms = resolve_barrel_geometries(base);
    let barrel_bases: Vec<CulvertSolveParams> = geoms
        .iter()
        .map(|(span, rise)| barrel_params_for_geometry(base, *span, *rise))
        .collect();

    let tw = base.tw_wsel;
    let mut low = tw;
    let mut high = tw
        + if base.units == UnitSystem::Metric {
            60.0
        } else {
            200.0
        };

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let sum_q: f64 = barrel_bases.iter().map(|p| barrel_q_for_wsel(p, mid)).sum();
        if sum_q < q_total {
            low = mid;
        } else {
            high = mid;
        }
    }
    let wsel = 0.5 * (low + high);

    let mut total_depth = 0.0;
    let mut total_vel = 0.0;
    let mut total_fr = 0.0;
    let mut total_q = 0.0;
    let mut max_inlet = 0.0f64;
    let mut max_outlet = 0.0f64;

    for p in &barrel_bases {
        let q_i = barrel_q_for_wsel(p, wsel);
        if q_i < 1e-9 {
            continue;
        }
        let bi = solve_culvert_barrel_internal(p, q_i);
        total_q += q_i;
        total_depth += bi.barrel_depth_ft * q_i;
        total_vel += bi.barrel_velocity_ft * q_i;
        total_fr += bi.barrel_froude * q_i;
        max_inlet = max_inlet.max(bi.wsel_inlet);
        max_outlet = max_outlet.max(bi.wsel_outlet);
    }

    if total_q < 1e-9 {
        return solve_culvert_barrel_internal(base, q_total);
    }

    BarrelSolveInternal {
        wsel,
        wsel_inlet: max_inlet,
        wsel_outlet: max_outlet,
        barrel_depth_ft: total_depth / total_q,
        barrel_velocity_ft: total_vel / total_q,
        barrel_froude: total_fr / total_q,
    }
}

fn solve_culvert_barrels(base: &CulvertSolveParams, q_total: f64) -> BarrelSolveInternal {
    if use_multi_barrel_solve(base) {
        solve_multi_barrel_barrels(base, q_total)
    } else {
        solve_culvert_barrel_internal(base, q_total)
    }
}

/// FHWA HDS-5 inlet-control nomograph coefficients (K, M, c, Y).
///
/// `inlet_type` codes (0 = legacy Ke-threshold fallback):
/// - Circular: 1 square headwall, 2 groove end, 3 beveled 45°, 4 projecting
/// - Box: 10 square edge, 11 flared wingwalls, 12 beveled top
/// - Arch/ConSpan: 20 projecting, 21 smooth entry headwall
pub fn inlet_nomograph_coeffs(
    shape: CulvertShape,
    inlet_type: i32,
    entrance_loss_coeff: f64,
) -> (f64, f64, f64, f64) {
    if inlet_type != 0 {
        return match (shape, inlet_type) {
            (CulvertShape::Circular, 1) => (0.0098, 2.0, 0.0398, 0.67),
            (CulvertShape::Circular, 2) => (0.0018, 2.0, 0.0292, 0.74),
            (CulvertShape::Circular, 3) => (0.0023, 2.0, 0.0317, 0.715),
            (CulvertShape::Circular, 4) => (0.0340, 1.5, 0.0553, 0.54),
            (CulvertShape::Box, 10) => (0.061, 0.75, 0.0400, 0.80),
            (CulvertShape::Box, 11) => (0.026, 1.0, 0.0347, 0.81),
            (CulvertShape::Box, 12) => (0.024, 1.0, 0.0338, 0.82),
            (
                CulvertShape::Arch
                | CulvertShape::ConspanArch
                | CulvertShape::PipeArch
                | CulvertShape::Elliptical
                | CulvertShape::Horseshoe,
                20,
            ) => (0.0300, 1.5, 0.0500, 0.60),
            (
                CulvertShape::Arch
                | CulvertShape::ConspanArch
                | CulvertShape::PipeArch
                | CulvertShape::Elliptical
                | CulvertShape::Horseshoe,
                21,
            ) => (0.0083, 2.0, 0.0374, 0.69),
            _ => inlet_nomograph_coeffs(shape, 0, entrance_loss_coeff),
        };
    }

    match shape {
        CulvertShape::Circular => {
            if entrance_loss_coeff <= 0.2 {
                (0.0018, 2.0, 0.0292, 0.74)
            } else {
                (0.0098, 2.0, 0.0398, 0.67)
            }
        }
        CulvertShape::Box | CulvertShape::Custom => {
            if entrance_loss_coeff <= 0.2 {
                (0.026, 1.0, 0.0347, 0.81)
            } else {
                (0.061, 0.75, 0.0400, 0.80)
            }
        }
        CulvertShape::Arch
        | CulvertShape::ConspanArch
        | CulvertShape::PipeArch
        | CulvertShape::Elliptical
        | CulvertShape::Horseshoe => {
            if entrance_loss_coeff <= 0.2 {
                (0.0083, 2.0, 0.0374, 0.69)
            } else {
                (0.0300, 1.5, 0.0500, 0.60)
            }
        }
    }
}

/// FHWA HDS-5 chart and scale lookup to determine coefficients (K, M, c, Y, is_form_2).
/// Supports Charts 1, 2, 3, 8, 9, 10, 11, 12, 13, 14, 15, 16, 29, 30, 34, 35.
pub fn fhwa_nomograph_coeffs(chart: i32, scale: i32) -> Option<(f64, f64, f64, f64, bool)> {
    match (chart, scale) {
        // Chart 1 (Circular Concrete Pipe)
        (1, 1) => Some((0.0098, 2.00, 0.0398, 0.67, false)),
        (1, 2) => Some((0.0018, 2.00, 0.0292, 0.74, false)),
        (1, 3) => Some((0.0045, 2.00, 0.0317, 0.69, false)),

        // Chart 2 (Circular CMP)
        (2, 1) => Some((0.0078, 2.00, 0.0379, 0.69, false)),
        (2, 2) => Some((0.0210, 1.33, 0.0463, 0.75, false)),
        (2, 3) => Some((0.0340, 1.50, 0.0553, 0.54, false)),

        // Chart 3 (Circular Pipe, Beveled Entrance)
        (3, 1) => Some((0.0018, 2.50, 0.0300, 0.74, false)),
        (3, 2) => Some((0.0018, 2.50, 0.0243, 0.83, false)),

        // Chart 8 (Rectangular Box, Flared Wingwalls)
        (8, 1) => Some((0.0260, 1.00, 0.0347, 0.81, false)),
        (8, 2) => Some((0.0610, 0.75, 0.0400, 0.80, false)),
        (8, 3) => Some((0.0610, 0.75, 0.0423, 0.82, false)),

        // Chart 9 (Rectangular Box, Flared Wingwalls & Top Edge Bevel)
        (9, 1) => Some((0.5100, 0.667, 0.0309, 0.80, true)),
        (9, 2) => Some((0.4860, 0.667, 0.0249, 0.83, true)),

        // Chart 10 (Rectangular Box, 90-deg Headwall, Chamfered or Beveled Inlet)
        (10, 1) => Some((0.5150, 0.667, 0.0375, 0.79, true)),
        (10, 2) => Some((0.4950, 0.667, 0.0314, 0.82, true)),
        (10, 3) => Some((0.4860, 0.667, 0.0252, 0.865, true)),

        // Chart 11 (Rectangular Box, Skewed Headwall)
        (11, 1) => Some((0.5450, 0.667, 0.04505, 0.73, true)),
        (11, 2) => Some((0.5330, 0.667, 0.04250, 0.705, true)),
        (11, 3) => Some((0.5220, 0.667, 0.04020, 0.68, true)),
        (11, 4) => Some((0.4980, 0.667, 0.03270, 0.75, true)),

        // Chart 12 (Rectangular Box, Non-offset Flared Wingwalls, Chamfered Top)
        (12, 1) => Some((0.4970, 0.667, 0.03390, 0.803, true)),
        (12, 2) => Some((0.4930, 0.667, 0.03610, 0.806, true)),
        (12, 3) => Some((0.4950, 0.667, 0.03860, 0.710, true)),

        // Chart 13 (Rectangular Box, Offset Flared Wingwalls, Beveled Top)
        (13, 1) => Some((0.4970, 0.667, 0.03020, 0.835, true)),
        (13, 2) => Some((0.4950, 0.667, 0.02520, 0.881, true)),
        (13, 3) => Some((0.4930, 0.667, 0.02270, 0.887, true)),

        // Chart 14 (Corrugated Metal Box)
        (14, 1) => Some((0.0083, 2.00, 0.0379, 0.69, false)),
        (14, 2) => Some((0.0145, 1.75, 0.0419, 0.64, false)),
        (14, 3) => Some((0.0340, 1.50, 0.0496, 0.57, false)),

        // Chart 15 (Horizontal Ellipse Concrete)
        (15, 1) => Some((0.0100, 2.00, 0.0398, 0.67, false)),
        (15, 2) => Some((0.0018, 2.50, 0.0292, 0.74, false)),
        (15, 3) => Some((0.0045, 2.00, 0.0317, 0.69, false)),

        // Chart 16 (Vertical Ellipse Concrete)
        (16, 1) => Some((0.0100, 2.00, 0.0398, 0.67, false)),
        (16, 2) => Some((0.0018, 2.50, 0.0292, 0.74, false)),
        (16, 3) => Some((0.0095, 2.00, 0.0317, 0.69, false)),

        // Chart 29 (Oval Concrete Horizontal)
        (29, 1) => Some((0.0100, 2.00, 0.0398, 0.67, false)),
        (29, 2) => Some((0.0018, 2.50, 0.0292, 0.74, false)),
        (29, 3) => Some((0.0045, 2.00, 0.0317, 0.69, false)),

        // Chart 30 (Oval Concrete Vertical)
        (30, 1) => Some((0.0100, 2.00, 0.0398, 0.67, false)),
        (30, 2) => Some((0.0018, 2.50, 0.0292, 0.74, false)),
        (30, 3) => Some((0.0095, 2.00, 0.0317, 0.69, false)),

        // Chart 34 (CMP Pipe Arch)
        (34, 1) => Some((0.0083, 2.00, 0.0379, 0.69, false)),
        (34, 2) => Some((0.0300, 1.00, 0.0463, 0.75, false)),
        (34, 3) => Some((0.0340, 1.50, 0.0496, 0.57, false)),

        // Chart 35 (Structural Plate Pipe Arch)
        (35, 1) => Some((0.0300, 1.50, 0.0496, 0.57, false)),
        (35, 2) => Some((0.0088, 2.00, 0.0368, 0.68, false)),
        (35, 3) => Some((0.0030, 2.00, 0.0269, 0.77, false)),

        // Chart 55 (Side-Tapered Circular Pipe, Throat Control)
        (55, 1) => Some((0.534, 0.333, 0.0196, 0.89, true)),
        (55, 2) => Some((0.519, 0.640, 0.0210, 0.90, true)),

        // Chart 56 (Side-Tapered Circular Pipe, Face Control)
        (56, 1) => Some((0.536, 0.622, 0.0368, 0.83, true)),
        (56, 2) => Some((0.5035, 0.719, 0.0478, 0.80, true)),

        // Chart 57 (Side/Slope-Tapered Box, Throat Control)
        (57, 1) => Some((0.534, 0.333, 0.0196, 0.89, true)),
        (57, 2) => Some((0.519, 0.640, 0.0210, 0.90, true)),

        // Chart 58 (Side-Tapered Box, Face Control)
        (58, 1) => Some((0.536, 0.622, 0.0368, 0.83, true)),
        (58, 2) => Some((0.5035, 0.719, 0.0478, 0.80, true)),

        // Chart 59 (Slope-Tapered Box, Face Control)
        (59, 1) => Some((0.536, 0.622, 0.0368, 0.83, true)),
        (59, 2) => Some((0.5035, 0.719, 0.0478, 0.80, true)),

        _ => None,
    }
}

struct BarrelSolveInternal {
    wsel: f64,
    wsel_inlet: f64,
    wsel_outlet: f64,
    barrel_depth_ft: f64,
    barrel_velocity_ft: f64,
    barrel_froude: f64,
}

fn assemble_culvert_result(
    params: &CulvertSolveParams,
    barrel: &BarrelSolveInternal,
    q_barrel: f64,
    q_weir: f64,
    control_type: String,
) -> CulvertSolveResult {
    let (barrel_depth, barrel_velocity) = if params.units == UnitSystem::Metric {
        (
            barrel.barrel_depth_ft * FT_TO_M,
            barrel.barrel_velocity_ft * FT_TO_M,
        )
    } else {
        (barrel.barrel_depth_ft, barrel.barrel_velocity_ft)
    };

    CulvertSolveResult {
        wsel: barrel.wsel,
        control_type,
        wsel_inlet: barrel.wsel_inlet,
        wsel_outlet: barrel.wsel_outlet,
        q_barrel,
        q_weir,
        barrel_depth,
        barrel_velocity,
        barrel_froude: barrel.barrel_froude,
    }
}

struct CulvertGeometry {
    shape: CulvertShape,
    span: f64,                           // feet
    rise: f64,                           // feet
    custom_ys: Option<Vec<f64>>,         // feet
    custom_areas: Option<Vec<f64>>,      // sq feet
    custom_perimeters: Option<Vec<f64>>, // feet
    custom_top_widths: Option<Vec<f64>>, // feet
}

impl CulvertGeometry {
    fn new(params: &CulvertSolveParams, span_ft: f64, rise_ft: f64) -> Self {
        let shape = CulvertShape::from_i32(params.shape_type);
        if shape != CulvertShape::Custom {
            return Self {
                shape,
                span: span_ft,
                rise: rise_ft,
                custom_ys: None,
                custom_areas: None,
                custom_perimeters: None,
                custom_top_widths: None,
            };
        }

        let scale_l = if params.units == UnitSystem::Metric {
            1.0 / FT_TO_M
        } else {
            1.0
        };
        let scale_a = scale_l * scale_l;

        let custom_ys = params
            .custom_shape_tbl_y
            .as_ref()
            .map(|v| v.iter().map(|&val| val * scale_l).collect());
        let custom_areas = params
            .custom_shape_tbl_area
            .as_ref()
            .map(|v| v.iter().map(|&val| val * scale_a).collect());
        let custom_perimeters = params
            .custom_shape_tbl_perimeter
            .as_ref()
            .map(|v| v.iter().map(|&val| val * scale_l).collect());
        let custom_top_widths = params
            .custom_shape_tbl_top_width
            .as_ref()
            .map(|v| v.iter().map(|&val| val * scale_l).collect());

        Self {
            shape,
            span: span_ft,
            rise: rise_ft,
            custom_ys,
            custom_areas,
            custom_perimeters,
            custom_top_widths,
        }
    }

    fn area(&self, y: f64) -> f64 {
        if self.shape == CulvertShape::Custom {
            self.interpolate_custom(y, "area")
        } else {
            get_culvert_area(self.shape, self.span, self.rise, y)
        }
    }

    fn perimeter(&self, y: f64) -> f64 {
        if self.shape == CulvertShape::Custom {
            self.interpolate_custom(y, "perimeter")
        } else {
            get_culvert_perimeter(self.shape, self.span, self.rise, y)
        }
    }

    fn top_width(&self, y: f64) -> f64 {
        if self.shape == CulvertShape::Custom {
            self.interpolate_custom(y, "top_width")
        } else {
            get_culvert_top_width(self.shape, self.span, self.rise, y)
        }
    }

    fn interpolate_custom(&self, y: f64, field: &str) -> f64 {
        let ys = match &self.custom_ys {
            Some(v) => v,
            None => return 0.0,
        };
        let n = ys.len();
        if n < 2 {
            return 0.0;
        }
        if y <= ys[0] {
            return match field {
                "perimeter" => self.span,
                _ => 0.0,
            };
        }
        let y_clamp = y.min(ys[n - 1]);

        let mut idx = 0;
        for i in 0..n - 1 {
            if y_clamp >= ys[i] && y_clamp <= ys[i + 1] {
                idx = i;
                break;
            }
        }
        let dy = ys[idx + 1] - ys[idx];
        let t = if dy > 1e-9 {
            (y_clamp - ys[idx]) / dy
        } else {
            0.0
        };

        match field {
            "area" => {
                if let Some(areas) = &self.custom_areas {
                    (1.0 - t) * areas[idx] + t * areas[idx + 1]
                } else {
                    0.0
                }
            }
            "perimeter" => {
                if let Some(perims) = &self.custom_perimeters {
                    (1.0 - t) * perims[idx] + t * perims[idx + 1]
                } else {
                    0.0
                }
            }
            "top_width" => {
                if y >= self.rise {
                    0.0
                } else {
                    if let Some(widths) = &self.custom_top_widths {
                        (1.0 - t) * widths[idx] + t * widths[idx + 1]
                    } else {
                        0.0
                    }
                }
            }
            _ => 0.0,
        }
    }

    fn effective_area(&self, y: f64, depth_blocked: f64) -> f64 {
        let d_b = depth_blocked.min(self.rise);
        if y <= d_b {
            0.0
        } else {
            self.area(y) - self.area(d_b)
        }
    }

    fn effective_top_width(&self, y: f64, depth_blocked: f64) -> f64 {
        let d_b = depth_blocked.min(self.rise);
        if y <= d_b {
            0.0
        } else {
            self.top_width(y)
        }
    }

    fn effective_perimeter(&self, y: f64, depth_blocked: f64) -> f64 {
        let d_b = depth_blocked.min(self.rise);
        if y <= d_b {
            0.0
        } else if d_b < 1e-9 {
            self.perimeter(y.min(self.rise))
        } else {
            let y_clamp = y.min(self.rise);
            let p_y = self.perimeter(y_clamp);
            let p_b = self.perimeter(d_b);
            let t_b = self.top_width(d_b);
            (p_y - p_b) + t_b
        }
    }

    fn composite_n(
        &self,
        y: f64,
        depth_blocked: f64,
        n_top: f64,
        n_bottom: f64,
        depth_bottom_n: f64,
    ) -> f64 {
        let d_b = depth_blocked.min(self.rise);
        let d_n = depth_bottom_n.min(self.rise);
        if d_n <= d_b || (n_bottom - n_top).abs() < 1e-9 {
            return n_top;
        }
        if y <= d_b {
            return n_bottom;
        }
        if y <= d_n {
            return n_bottom;
        }
        let p_bottom = self.effective_perimeter(d_n, d_b);
        let y_clamp = y.min(self.rise);
        let p_y = self.perimeter(y_clamp);
        let p_n = self.perimeter(d_n);
        let p_top = (p_y - p_n).max(0.0);
        let p_total = p_bottom + p_top;
        if p_total > 1e-9 {
            ((p_bottom * n_bottom.powf(1.5) + p_top * n_top.powf(1.5)) / p_total).powf(2.0 / 3.0)
        } else {
            n_top
        }
    }

    fn critical_depth(&self, q: f64, depth_blocked: f64) -> f64 {
        let d_b = depth_blocked.min(self.rise);
        let mut low = d_b;
        let mut high = self.rise;
        let mut best_yc = d_b;

        for _ in 0..50 {
            let mid = 0.5 * (low + high);
            let area = self.effective_area(mid, d_b);
            let top_width = self.effective_top_width(mid, d_b);

            if area < 1e-9 {
                low = mid;
                continue;
            }

            let fr_sq = (q * q * top_width) / (G_ENGLISH * area.powi(3));
            if top_width < 1e-9 || fr_sq > 1.0 {
                low = mid;
            } else {
                high = mid;
            }
            best_yc = mid;
        }
        best_yc
    }

    fn static_moment(&self, y: f64, depth_blocked: f64) -> f64 {
        let d_b = depth_blocked.min(self.rise);
        if y <= d_b {
            return 0.0;
        }
        let n_steps = 20;
        let dy = (y - d_b) / n_steps as f64;
        let mut sum = 0.0;
        for i in 0..n_steps {
            let eta_i = d_b + i as f64 * dy;
            let eta_ip1 = eta_i + dy;
            let w_i = self.effective_top_width(eta_i, d_b);
            let w_ip1 = self.effective_top_width(eta_ip1, d_b);
            let term_i = (y - eta_i) * w_i;
            let term_ip1 = (y - eta_ip1) * w_ip1;
            sum += 0.5 * (term_i + term_ip1) * dy;
        }
        sum
    }

    fn specific_force(&self, y: f64, q: f64, depth_blocked: f64) -> f64 {
        let d_b = depth_blocked.min(self.rise);
        if y <= d_b {
            return f64::INFINITY;
        }
        let area = self.effective_area(y, d_b);
        if area < 1e-9 {
            return f64::INFINITY;
        }
        let static_mom = self.static_moment(y, d_b);
        static_mom + (q * q) / (G_ENGLISH * area)
    }
}

fn solve_inlet_control_elevation(
    geom: &CulvertGeometry,
    q_cfs: f64,
    span_ft: f64,
    rise_ft: f64,
    len_ft: f64,
    z_up_ft: f64,
    z_down_ft: f64,
    db_ft: f64,
    chart_number: Option<i32>,
    scale_number: Option<i32>,
    inlet_type: i32,
    entrance_loss_coeff: f64,
) -> f64 {
    let d_eff = (rise_ft - db_ft).max(0.01);
    let a_full_eff = geom.effective_area(rise_ft, db_ft);

    let yc = geom.critical_depth(q_cfs, db_ft);
    let yc_eff = (yc - db_ft).max(0.0);
    let ac = geom.effective_area(yc, db_ft);
    let vc = if ac > 1e-9 { q_cfs / ac } else { 0.0 };
    let hc_eff = yc_eff + (vc * vc) / (2.0 * G_ENGLISH);

    let (k, m, c, y, is_form_2) = if let (Some(chart), Some(scale)) = (chart_number, scale_number) {
        if chart > 0 && scale > 0 {
            if let Some((k_val, m_val, c_val, y_val, form2)) = fhwa_nomograph_coeffs(chart, scale) {
                (k_val, m_val, c_val, y_val, form2)
            } else {
                let (k_val, m_val, c_val, y_val) = inlet_nomograph_coeffs(geom.shape, inlet_type, entrance_loss_coeff);
                (k_val, m_val, c_val, y_val, false)
            }
        } else {
            let (k_val, m_val, c_val, y_val) = inlet_nomograph_coeffs(geom.shape, inlet_type, entrance_loss_coeff);
            (k_val, m_val, c_val, y_val, false)
        }
    } else {
        let (k_val, m_val, c_val, y_val) = inlet_nomograph_coeffs(geom.shape, inlet_type, entrance_loss_coeff);
        (k_val, m_val, c_val, y_val, false)
    };

    let culv_slope = (z_up_ft - z_down_ft) / len_ft.max(1.0);
    let f_param = q_cfs / (a_full_eff * d_eff.sqrt());

    let hw_d_unsub = if is_form_2 {
        k * f_param.powf(m)
    } else {
        (hc_eff / d_eff) + k * f_param.powf(m) - 0.5 * culv_slope
    };
    let hw_d_sub = c * f_param.powi(2) + y - 0.5 * culv_slope;

    let hw_d = if f_param <= 3.0 {
        hw_d_unsub
    } else if f_param >= 4.0 {
        hw_d_sub
    } else {
        let t = (f_param - 3.0) / (4.0 - 3.0);
        (1.0 - t) * hw_d_unsub + t * hw_d_sub
    };

    let hw_inlet_eff = if is_form_2 {
        hw_d * d_eff
    } else {
        (hw_d * d_eff).max(hc_eff)
    };
    z_up_ft + db_ft + hw_inlet_eff
}

fn solve_gvf_step(
    geom: &CulvertGeometry,
    q: f64,
    n_top: f64,
    n_bottom: f64,
    depth_bottom_n: f64,
    depth_blocked: f64,
    y1: f64,
    z1: f64,
    z2: f64,
    dx: f64,
    is_subcritical: bool,
    yc: f64,
) -> f64 {
    let d_b = depth_blocked.min(geom.rise);
    let area1 = geom.effective_area(y1, d_b);
    if area1 < 1e-9 {
        return d_b;
    }
    let n1 = geom.composite_n(y1, d_b, n_top, n_bottom, depth_bottom_n);
    let p1 = geom.effective_perimeter(y1, d_b);
    let r1 = if p1 > 1e-9 { area1 / p1 } else { 0.0 };
    let k1 = if n1 > 1e-9 && r1 > 0.0 {
        (1.486 / n1) * area1 * r1.powf(2.0 / 3.0)
    } else {
        0.0
    };
    let sf1 = if k1 > 1e-9 { (q / k1).powi(2) } else { 0.0 };
    let v1 = q / area1;
    let hv1 = (v1 * v1) / (2.0 * G_ENGLISH);
    let eg1 = y1 + z1 + hv1;

    let mut low = if is_subcritical { yc } else { d_b };
    let mut high = if is_subcritical { geom.rise + 50.0 } else { yc };
    let mut best_y2 = 0.5 * (low + high);

    for _ in 0..40 {
        let y2 = 0.5 * (low + high);
        
        let (area2, k2, sf2, v2, hv2) = if y2 >= geom.rise {
            let area_full = geom.effective_area(geom.rise, d_b);
            let p_full = geom.effective_perimeter(geom.rise, d_b);
            let r_full = if p_full > 1e-9 { area_full / p_full } else { 0.0 };
            let n_full = geom.composite_n(geom.rise, d_b, n_top, n_bottom, depth_bottom_n);
            let k_full = if n_full > 1e-9 && r_full > 0.0 {
                (1.486 / n_full) * area_full * r_full.powf(2.0 / 3.0)
            } else {
                0.0
            };
            let sf_full = if k_full > 1e-9 { (q / k_full).powi(2) } else { 0.0 };
            let v_full = q / area_full;
            let hv_full = (v_full * v_full) / (2.0 * G_ENGLISH);
            (area_full, k_full, sf_full, v_full, hv_full)
        } else {
            let area2 = geom.effective_area(y2, d_b);
            let n2 = geom.composite_n(y2, d_b, n_top, n_bottom, depth_bottom_n);
            let p2 = geom.effective_perimeter(y2, d_b);
            let r2 = if p2 > 1e-9 { area2 / p2 } else { 0.0 };
            let k2 = if n2 > 1e-9 && r2 > 0.0 {
                (1.486 / n2) * area2 * r2.powf(2.0 / 3.0)
            } else {
                0.0
            };
            let sf2 = if k2 > 1e-9 { (q / k2).powi(2) } else { 0.0 };
            let v2 = if area2 > 1e-9 { q / area2 } else { 0.0 };
            let hv2 = (v2 * v2) / (2.0 * G_ENGLISH);
            (area2, k2, sf2, v2, hv2)
        };

        let sf_avg = 0.5 * (sf1 + sf2);
        let hf = dx * sf_avg;

        let eg2_calc = y2 + z2 + hv2;
        let target_eg = if is_subcritical { eg1 + hf } else { eg1 - hf };

        if is_subcritical {
            if eg2_calc < target_eg {
                low = y2;
            } else {
                high = y2;
            }
        } else {
            if eg2_calc < target_eg {
                high = y2;
            } else {
                low = y2;
            }
        }
        best_y2 = y2;
    }
    best_y2
}

fn compute_gvf_outlet_control(
    geom: &CulvertGeometry,
    q_cfs: f64,
    n_top: f64,
    n_bottom: f64,
    depth_bottom_n: f64,
    depth_blocked: f64,
    tw_ft: f64,
    z_down_ft: f64,
    z_up_ft: f64,
    len_ft: f64,
    yc: f64,
    entrance_loss_coeff: f64,
    exit_loss_coeff: f64,
    ds_vel_hd: f64,
    us_vel_hd: f64,
) -> f64 {
    let rise = geom.rise;
    let db = depth_blocked.min(rise);
    let tw_depth = tw_ft - z_down_ft;
    
    let (mut y_start, mut is_full) = if tw_depth >= rise {
        (rise, true)
    } else {
        (tw_depth.max(yc).min(rise), false)
    };

    let area_exit = geom.effective_area(y_start, db);
    let v_exit = if area_exit > 1e-9 { q_cfs / area_exit } else { 0.0 };
    let v_exit_hd = (v_exit * v_exit) / (2.0 * G_ENGLISH);
    
    let eg_start = if is_full {
        tw_ft + ds_vel_hd + exit_loss_coeff * (v_exit_hd - ds_vel_hd).max(0.0)
    } else if tw_depth >= yc {
        tw_ft + ds_vel_hd + exit_loss_coeff * (v_exit_hd - ds_vel_hd).max(0.0)
    } else {
        z_down_ft + y_start + v_exit_hd
    };

    let n_steps = 10;
    let dx = len_ft / n_steps as f64;
    let mut y_curr = y_start;
    let mut eg_curr = eg_start;

    for i in 0..n_steps {
        let z_i = z_down_ft + (i as f64) * (z_up_ft - z_down_ft) / n_steps as f64;
        let z_ip1 = z_down_ft + ((i + 1) as f64) * (z_up_ft - z_down_ft) / n_steps as f64;

        if is_full {
            let area_full = geom.effective_area(rise, db);
            let p_full = geom.effective_perimeter(rise, db);
            let r_full = if p_full > 1e-9 { area_full / p_full } else { 0.0 };
            let n_full = geom.composite_n(rise, db, n_top, n_bottom, depth_bottom_n);
            let k_full = if n_full > 1e-9 && r_full > 0.0 {
                (1.486 / n_full) * area_full * r_full.powf(2.0 / 3.0)
            } else {
                0.0
            };
            let sf_full = if k_full > 1e-9 { (q_cfs / k_full).powi(2) } else { 0.0 };
            eg_curr += dx * sf_full; 
            
            let hgl_depth = eg_curr - z_ip1 - (q_cfs * q_cfs) / (2.0 * G_ENGLISH * area_full * area_full);
            if hgl_depth < rise {
                is_full = false;
                y_curr = hgl_depth.max(db);
            } else {
                y_curr = rise;
            }
        } else {
            let y_next = solve_gvf_step(
                geom,
                q_cfs,
                n_top,
                n_bottom,
                depth_bottom_n,
                depth_blocked,
                y_curr,
                z_i,
                z_ip1,
                dx,
                true,
                yc,
            );
            if y_next <= yc + 1e-4 && yc <= rise {
                return z_down_ft; // Subcritical GVF profile drew down to critical depth; wave cannot propagate upstream
            }
            y_curr = y_next;

            if y_curr >= rise {
                is_full = true;
                let area_full = geom.effective_area(rise, db);
                let v_full = q_cfs / area_full;
                let hv_full = (v_full * v_full) / (2.0 * G_ENGLISH);
                eg_curr = y_curr + z_ip1 + hv_full;
            } else {
                let area_curr = geom.effective_area(y_curr, db);
                let v_curr = if area_curr > 1e-9 { q_cfs / area_curr } else { 0.0 };
                let hv_curr = (v_curr * v_curr) / (2.0 * G_ENGLISH);
                eg_curr = y_curr + z_ip1 + hv_curr;
            }
        }
    }

    let area_inlet = geom.effective_area(y_curr, db);
    let v_inlet = if area_inlet > 1e-9 { q_cfs / area_inlet } else { 0.0 };
    let v_inlet_hd = (v_inlet * v_inlet) / (2.0 * G_ENGLISH);
    let he = entrance_loss_coeff * v_inlet_hd;
    let eg_inlet = eg_curr + he;
    let wsel_outlet = eg_inlet - us_vel_hd;
    wsel_outlet
}

fn solve_culvert_barrel_internal(params: &CulvertSolveParams, q: f64) -> BarrelSolveInternal {
    // Convert inputs to English units for calculation
    let (q_cfs, span_ft, rise_ft, len_ft, z_down_ft, z_up_ft, tw_ft, db_ft, dbn_ft) =
        if params.units == UnitSystem::Metric {
            (
                q / CFS_TO_CMS,
                params.span / FT_TO_M,
                params.rise / FT_TO_M,
                params.length / FT_TO_M,
                params.z_down / FT_TO_M,
                params.z_up / FT_TO_M,
                params.tw_wsel / FT_TO_M,
                params.depth_blocked / FT_TO_M,
                params.depth_bottom_n / FT_TO_M,
            )
        } else {
            (
                q,
                params.span,
                params.rise,
                params.length,
                params.z_down,
                params.z_up,
                params.tw_wsel,
                params.depth_blocked,
                params.depth_bottom_n,
            )
        };

    let (span_ft, len_ft) = apply_barrel_skew(params.skew_deg, span_ft, len_ft);
    let geom = CulvertGeometry::new(params, span_ft, rise_ft);

    let d_eff = (rise_ft - db_ft).max(0.01);
    let a_full_eff = geom.effective_area(rise_ft, db_ft);

    let us_vel_ft = if params.units == UnitSystem::Metric {
        params.us_velocity / FT_TO_M
    } else {
        params.us_velocity
    };
    // Apply velocity distribution coefficient alpha (~1.3 for contracted sections near culverts)
    let us_vel_hd = (us_vel_ft * us_vel_ft) / (2.0 * G_ENGLISH) * 1.3;

    let ds_vel_ft = if params.units == UnitSystem::Metric {
        params.ds_velocity / FT_TO_M
    } else {
        params.ds_velocity
    };
    let ds_vel_hd = (ds_vel_ft * ds_vel_ft) / (2.0 * G_ENGLISH);

    // 1. INLET CONTROL CALCULATIONS
    let yc = geom.critical_depth(q_cfs, db_ft);

    // Throat Control WSEL
    let throat_chart = params.tapered_throat_chart_number.or(params.chart_number);
    let throat_scale = params.tapered_throat_scale_number.or(params.scale_number);
    let wsel_throat = solve_inlet_control_elevation(
        &geom,
        q_cfs,
        span_ft,
        rise_ft,
        len_ft,
        z_up_ft,
        z_down_ft,
        db_ft,
        throat_chart,
        throat_scale,
        params.inlet_type,
        params.entrance_loss_coeff,
    );

    // Face Control WSEL (if tapered)
    let mut wsel_face = 0.0;
    let fall_ft = if params.units == UnitSystem::Metric {
        params.tapered_fall / FT_TO_M
    } else {
        params.tapered_fall
    };
    let z_face_ft = z_up_ft + fall_ft;
    if params.tapered_type == 1 || params.tapered_type == 2 {
        let face_span_ft = params.tapered_face_span
            .map(|v| if params.units == UnitSystem::Metric { v / FT_TO_M } else { v })
            .unwrap_or(span_ft);
        let face_rise_ft = params.tapered_face_rise
            .map(|v| if params.units == UnitSystem::Metric { v / FT_TO_M } else { v })
            .unwrap_or(rise_ft);
        let face_geom = CulvertGeometry::new(params, face_span_ft, face_rise_ft);
        let face_chart = params.tapered_face_chart_number.or(params.chart_number);
        let face_scale = params.tapered_face_scale_number.or(params.scale_number);
        wsel_face = solve_inlet_control_elevation(
            &face_geom,
            q_cfs,
            face_span_ft,
            face_rise_ft,
            len_ft,
            z_face_ft,
            z_down_ft,
            db_ft,
            face_chart,
            face_scale,
            params.inlet_type,
            params.entrance_loss_coeff,
        );
    }

    // Crest Control WSEL (slope-tapered only)
    let mut wsel_crest = 0.0;
    if params.tapered_type == 2 {
        let crest_len_ft = params.tapered_crest_weir_length
            .map(|v| if params.units == UnitSystem::Metric { v / FT_TO_M } else { v })
            .unwrap_or(span_ft);
        let crest_coeff = if params.units == UnitSystem::Metric {
            if let Some(coeff) = params.tapered_crest_weir_coeff {
                if coeff > 0.0 {
                    coeff / CFS_TO_CMS * FT_TO_M.powf(2.5)
                } else {
                    3.0
                }
            } else {
                3.0
            }
        } else {
            params.tapered_crest_weir_coeff.unwrap_or(3.0)
        };


        let hc = if crest_len_ft > 1e-9 && crest_coeff > 1e-9 {
            (q_cfs / (crest_coeff * crest_len_ft)).powf(2.0 / 3.0)
        } else {
            0.0
        };
        wsel_crest = z_face_ft + hc;
    }

    // Governing Inlet Control WSEL
    let mut wsel_inlet = wsel_throat;
    if wsel_face > wsel_inlet {
        wsel_inlet = wsel_face;
    }
    if wsel_crest > wsel_inlet {
        wsel_inlet = wsel_crest;
    }

    let wsel_outlet = compute_gvf_outlet_control(
        &geom,
        q_cfs,
        params.roughness_n,
        params.manning_n_bottom,
        dbn_ft,
        db_ft,
        tw_ft,
        z_down_ft,
        z_up_ft,
        len_ft,
        yc,
        params.entrance_loss_coeff,
        params.exit_loss_coeff,
        ds_vel_hd,
        us_vel_hd,
    );

    let wsel_up_ft = wsel_inlet.max(wsel_outlet);

    let wsel_user = if params.units == UnitSystem::Metric {
        wsel_up_ft * FT_TO_M
    } else {
        wsel_up_ft
    };
    let wsel_inlet_user = if params.units == UnitSystem::Metric {
        wsel_inlet * FT_TO_M
    } else {
        wsel_inlet
    };
    let wsel_outlet_user = if params.units == UnitSystem::Metric {
        wsel_outlet * FT_TO_M
    } else {
        wsel_outlet
    };

    let y_barrel = (tw_ft - z_down_ft).max(yc).min(rise_ft);
    let a_barrel = geom.effective_area(y_barrel, db_ft);
    let v_barrel = if a_barrel > 1e-9 { q_cfs / a_barrel } else { 0.0 };

    let t_barrel = geom.effective_top_width(y_barrel, db_ft);
    let d_hyd = if t_barrel > 1e-9 {
        a_barrel / t_barrel
    } else {
        0.0
    };
    let barrel_froude = if d_hyd > 1e-9 {
        v_barrel / (G_ENGLISH * d_hyd).sqrt()
    } else {
        0.0
    };

    BarrelSolveInternal {
        wsel: wsel_user,
        wsel_inlet: wsel_inlet_user,
        wsel_outlet: wsel_outlet_user,
        barrel_depth_ft: y_barrel,
        barrel_velocity_ft: v_barrel,
        barrel_froude,
    }
}

fn barrel_control_type(barrel: &BarrelSolveInternal) -> String {
    if barrel.wsel_inlet >= barrel.wsel_outlet - 1e-6 {
        "inlet".to_string()
    } else {
        "outlet".to_string()
    }
}

const BRADLEY_SUBMERGENCE_PCT: [f64; 12] = [
    0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 95.0, 98.0,
];
const BRADLEY_FLOW_FACTOR: [f64; 12] = [
    1.0, 1.0, 0.99, 0.97, 0.94, 0.90, 0.84, 0.75, 0.62, 0.40, 0.22, 0.08,
];

fn bradley_weir_submergence_factor(submergence_ratio: f64) -> f64 {
    if submergence_ratio <= 0.0 {
        return 1.0;
    }
    if submergence_ratio >= 1.0 {
        return 0.0;
    }
    let pct = submergence_ratio * 100.0;
    if pct > 98.0 {
        // Linearly interpolate from 0.08 at 98% to 0.0 at 100%
        let t = (pct - 98.0) / (100.0 - 98.0);
        return 0.08 * (1.0 - t);
    }
    for i in 1..BRADLEY_SUBMERGENCE_PCT.len() {
        if pct <= BRADLEY_SUBMERGENCE_PCT[i] {
            let t = (pct - BRADLEY_SUBMERGENCE_PCT[i - 1])
                / (BRADLEY_SUBMERGENCE_PCT[i] - BRADLEY_SUBMERGENCE_PCT[i - 1]);
            return BRADLEY_FLOW_FACTOR[i - 1]
                + t * (BRADLEY_FLOW_FACTOR[i] - BRADLEY_FLOW_FACTOR[i - 1]);
        }
    }
    0.0
}

pub(crate) fn weir_flow_us(
    cw: f64,
    length_ft: f64,
    wsel_ft: f64,
    crest_ft: f64,
    tw_ft: f64,
) -> f64 {
    let head = (wsel_ft - crest_ft).max(0.0);
    if head < 1e-9 || length_ft < 1e-9 {
        return 0.0;
    }
    let tail_above = (tw_ft - crest_ft).max(0.0);
    let submergence_ratio = (tail_above / head).clamp(0.0, 1.0);
    let factor = bradley_weir_submergence_factor(submergence_ratio);
    cw * length_ft * head.powf(1.5) * factor
}

pub(crate) fn roadway_profile_weir_flow(
    cw_us: f64,
    wsel_ft: f64,
    tw_ft: f64,
    stations_ft: &[f64],
    elevations_ft: &[f64],
    skew_deg: f64,
) -> f64 {
    if stations_ft.len() < 2 || elevations_ft.len() != stations_ft.len() {
        return 0.0;
    }
    let skew_cos = skew_deg.clamp(0.0, 59.0).to_radians().cos().max(0.52);
    let mut total_q = 0.0;
    for i in 0..stations_ft.len().saturating_sub(1) {
        let w = (stations_ft[i + 1] - stations_ft[i]) * skew_cos;
        if w <= 0.0 {
            continue;
        }
        let crest_ft = 0.5 * (elevations_ft[i] + elevations_ft[i + 1]);
        let head = (wsel_ft - crest_ft).max(0.0);
        if head < 1e-9 {
            continue;
        }
        let tail_above = (tw_ft - crest_ft).max(0.0);
        let submergence_ratio = (tail_above / head).clamp(0.0, 1.0);
        let factor = bradley_weir_submergence_factor(submergence_ratio);
        total_q += cw_us * w * head.powf(1.5) * factor;
    }
    total_q
}

/// Solve culvert headwater including optional roadway overtopping weir.
pub fn solve_culvert(params: &CulvertSolveParams) -> CulvertSolveResult {
    let mut params = params.clone();
    normalize_culvert_params(&mut params);
    let q_total = params.q;

    let roadway_stations_ft: Option<Vec<f64>> = params.roadway_stations.as_ref().map(|v| {
        if params.units == UnitSystem::Metric {
            v.iter().map(|&x| x / FT_TO_M).collect()
        } else {
            v.clone()
        }
    });
    let roadway_elevations_ft: Option<Vec<f64>> = params.roadway_elevations.as_ref().map(|v| {
        if params.units == UnitSystem::Metric {
            v.iter().map(|&y| y / FT_TO_M).collect()
        } else {
            v.clone()
        }
    });

    let profile_min_elev_user = if let (Some(ref sts), Some(ref els)) =
        (&params.roadway_stations, &params.roadway_elevations)
    {
        if !els.is_empty() && els.len() == sts.len() {
            Some(els.iter().copied().fold(f64::INFINITY, f64::min))
        } else {
            None
        }
    } else {
        None
    };

    let crest_user = match params.crest_elev.or(profile_min_elev_user) {
        Some(c) => c,
        None => {
            let barrel = solve_culvert_barrels(&params, q_total);
            return assemble_culvert_result(
                &params,
                &barrel,
                q_total,
                0.0,
                barrel_control_type(&barrel),
            );
        }
    };

    let default_weir_len_user = default_weir_length_user(&params);
    let tw_ft = if params.units == UnitSystem::Metric {
        params.tw_wsel / FT_TO_M
    } else {
        params.tw_wsel
    };
    let (mut crest_ft, cw_us, length_ft) = if params.units == UnitSystem::Metric {
        (
            crest_user / FT_TO_M,
            if params.weir_coeff > 0.0 {
                params.weir_coeff / CFS_TO_CMS * FT_TO_M.powf(2.5)
            } else {
                2.6
            },
            if params.weir_length > 0.0 {
                params.weir_length / FT_TO_M
            } else {
                default_weir_len_user / FT_TO_M
            },
        )
    } else {
        (
            crest_user,
            if params.weir_coeff > 0.0 {
                params.weir_coeff
            } else {
                2.6
            },
            if params.weir_length > 0.0 {
                params.weir_length
            } else {
                default_weir_len_user
            },
        )
    };

    if let Some(ref elevs_ft) = roadway_elevations_ft {
        if !elevs_ft.is_empty() {
            crest_ft = elevs_ft.iter().copied().fold(f64::INFINITY, f64::min);
        }
    }

    let get_q_weir_cfs = |wsel_f: f64| -> f64 {
        if let (Some(ref sts), Some(ref els)) = (&roadway_stations_ft, &roadway_elevations_ft) {
            roadway_profile_weir_flow(cw_us, wsel_f, tw_ft, sts, els, params.skew_deg)
        } else {
            weir_flow_us(cw_us, length_ft, wsel_f, crest_ft, tw_ft)
        }
    };

    let barrel_full = solve_culvert_barrels(&params, q_total);
    let last_control = barrel_control_type(&barrel_full);
    let wsel_full_ft = if params.units == UnitSystem::Metric {
        barrel_full.wsel / FT_TO_M
    } else {
        barrel_full.wsel
    };

    if wsel_full_ft <= crest_ft + 1e-6 {
        return assemble_culvert_result(&params, &barrel_full, q_total, 0.0, last_control);
    }

    // Bisect on q_barrel in [0.0, q_total]
    let mut low_q = 0.0;
    let mut high_q = q_total;
    let mut best_q_barrel = 0.0;
    let mut best_q_weir = q_total;
    let mut best_barrel = barrel_full;

    for _ in 0..30 {
        let mid_q = 0.5 * (low_q + high_q);
        let mid_barrel = solve_culvert_barrels(&params, mid_q);
        let mid_wsel_ft = if params.units == UnitSystem::Metric {
            mid_barrel.wsel / FT_TO_M
        } else {
            mid_barrel.wsel
        };

        let q_weir_cfs = get_q_weir_cfs(mid_wsel_ft);
        let q_weir = if params.units == UnitSystem::Metric {
            q_weir_cfs * CFS_TO_CMS
        } else {
            q_weir_cfs
        };

        let q_sum = mid_q + q_weir;

        if q_sum < q_total {
            low_q = mid_q;
        } else {
            high_q = mid_q;
        }
        best_q_barrel = mid_q;
        best_q_weir = q_weir;
        best_barrel = mid_barrel;
    }

    let control = if best_q_weir > 0.01 * q_total {
        "overtopping".to_string()
    } else {
        barrel_control_type(&best_barrel)
    };

    assemble_culvert_result(&params, &best_barrel, best_q_barrel, best_q_weir, control)
}

/// Supercritical routing: given upstream headwater and discharge, solve downstream tailwater.
/// Finds the minimum tailwater that produces the target headwater (inlet-control flat rating limbs).
pub fn solve_culvert_from_headwater(
    params: &CulvertSolveParams,
    hw_wsel: f64,
) -> (f64, CulvertSolveResult) {
    let mut base = params.clone();
    normalize_culvert_params(&mut base);

    let headwater_at = |tw: f64| {
        let mut p = base.clone();
        p.tw_wsel = tw;
        solve_culvert(&p)
    };

    let rise = base.rise.max(0.01);
    let mut lo = base.z_down;
    let mut hi = (hw_wsel + rise).max(lo + 0.1);

    if headwater_at(lo).wsel > hw_wsel + 1e-4 {
        let result = headwater_at(lo);
        return (lo, result);
    }

    for _ in 0..50 {
        let mid = 0.5 * (lo + hi);
        if headwater_at(mid).wsel >= hw_wsel - 1e-4 {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    let tw = hi;
    (tw, headwater_at(tw))
}

/// Compute headwater vs discharge at fixed tailwater (culvert rating curve).
pub fn compute_culvert_rating_curve(inputs: &CulvertRatingCurveInputs) -> CulvertRatingCurveResult {
    let mut q = Vec::with_capacity(inputs.q_values.len());
    let mut wsel = Vec::with_capacity(inputs.q_values.len());
    let mut control_types = Vec::with_capacity(inputs.q_values.len());
    let mut wsel_inlet = Vec::with_capacity(inputs.q_values.len());
    let mut wsel_outlet = Vec::with_capacity(inputs.q_values.len());
    let mut q_barrel = Vec::with_capacity(inputs.q_values.len());
    let mut q_weir = Vec::with_capacity(inputs.q_values.len());
    let mut barrel_depth = Vec::with_capacity(inputs.q_values.len());
    let mut barrel_velocity = Vec::with_capacity(inputs.q_values.len());
    let mut barrel_froude = Vec::with_capacity(inputs.q_values.len());

    let mut base = inputs.culvert.clone();
    normalize_culvert_params(&mut base);

    for &q_sample in &inputs.q_values {
        let mut params = base.clone();
        params.q = q_sample;
        let result = solve_culvert(&params);
        q.push(q_sample);
        wsel.push(result.wsel);
        control_types.push(result.control_type);
        wsel_inlet.push(result.wsel_inlet);
        wsel_outlet.push(result.wsel_outlet);
        q_barrel.push(result.q_barrel);
        q_weir.push(result.q_weir);
        barrel_depth.push(result.barrel_depth);
        barrel_velocity.push(result.barrel_velocity);
        barrel_froude.push(result.barrel_froude);
    }

    CulvertRatingCurveResult {
        q,
        wsel,
        control_types,
        wsel_inlet,
        wsel_outlet,
        q_barrel,
        q_weir,
        barrel_depth,
        barrel_velocity,
        barrel_froude,
    }
}

/// Legacy barrel-only solve (returns WSEL only). Prefer `solve_culvert`.
pub fn solve_culvert_wsel(
    q: f64,
    shape_type: i32,
    span: f64,
    rise: f64,
    roughness_n: f64,
    length: f64,
    entrance_loss_coeff: f64,
    exit_loss_coeff: f64,
    z_down: f64,
    z_up: f64,
    tw_wsel: f64,
    units: UnitSystem,
    manning_n_bottom: f64,
    depth_bottom_n: f64,
    depth_blocked: f64,
    ds_velocity: f64,
    us_velocity: f64,
) -> f64 {
    let params = CulvertSolveParams {
        q,
        shape_type,
        inlet_type: 0,
        span,
        rise,
        roughness_n,
        length,
        entrance_loss_coeff,
        exit_loss_coeff,
        z_down,
        z_up,
        tw_wsel,
        units,
        manning_n_bottom,
        depth_bottom_n,
        depth_blocked,
        ds_velocity,
        us_velocity,
        crest_elev: None,
        weir_coeff: 0.0,
        weir_length: 0.0,
        num_barrels: 1,
        active_barrels: 0,
        skew_deg: 0.0,
        barrel_spans: None,
        barrel_rises: None,
        custom_shape_tbl_y: None,
        custom_shape_tbl_area: None,
        custom_shape_tbl_perimeter: None,
        custom_shape_tbl_top_width: None,
        roadway_stations: None,
        roadway_elevations: None,

        chart_number: None,

        scale_number: None,
        ..Default::default()
    };
    solve_culvert(&params).wsel
}

/// Implicit Preissmann residual $R = y_\mathrm{us} - \mathrm{HW}_\mathrm{inlet}$ and partial derivatives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CulvertHeadwaterResidual {
    pub r: f64,
    pub dr_dy_us: f64,
    pub dr_dy_ds: f64,
    pub dr_dq: f64,
}

fn wsel_user_to_metric(wsel_user: f64, units: UnitSystem) -> f64 {
    if units == UnitSystem::USCustomary {
        wsel_user * FT_TO_M
    } else {
        wsel_user
    }
}

/// True when implicit Preissmann culvert residual may replace the reach momentum row.
pub fn culvert_implicit_eligible(params: &CulvertSolveParams) -> bool {
    if params.q <= 1e-6 {
        return false;
    }
    // Roadway overtopping splits barrel/weir iteratively — explicit fallback for now.
    params.crest_elev.is_none()
}

/// True when legacy inlet-only analytic shortcut may be used (tests / fast path).
pub fn culvert_implicit_inlet_eligible(params: &CulvertSolveParams) -> bool {
    let shape = CulvertShape::from_i32(params.shape_type);
    if !matches!(shape, CulvertShape::Circular | CulvertShape::Box) {
        return false;
    }
    if !culvert_implicit_eligible(params) {
        return false;
    }
    if params.barrel_spans.is_some() || params.barrel_rises.is_some() {
        return false;
    }
    if params.num_barrels > 1 || params.active_barrels > 1 {
        return false;
    }
    true
}

fn culvert_inlet_wsel_metric(
    y_ds_metric: f64,
    q_metric: f64,
    params: &CulvertSolveParams,
) -> Option<f64> {
    if !culvert_implicit_inlet_eligible(params) {
        return None;
    }
    let mut p = params.clone();
    if p.units == UnitSystem::USCustomary {
        p.q = q_metric / CFS_TO_CMS;
        p.tw_wsel = y_ds_metric / FT_TO_M;
    } else {
        p.q = q_metric;
        p.tw_wsel = y_ds_metric;
    }
    normalize_culvert_params(&mut p);
    let barrel = solve_culvert_barrels(&p, p.q);
    if barrel.wsel_inlet + 1e-4 < barrel.wsel_outlet {
        return None;
    }
    Some(wsel_user_to_metric(barrel.wsel_inlet, p.units))
}

fn culvert_solve_headwater_metric(
    _y_us_metric: f64,
    y_ds_metric: f64,
    q_metric: f64,
    params: &CulvertSolveParams,
) -> Option<f64> {
    if !culvert_implicit_eligible(params) {
        return None;
    }
    let mut p = params.clone();
    if p.units == UnitSystem::USCustomary {
        p.q = q_metric / CFS_TO_CMS;
        p.tw_wsel = y_ds_metric / FT_TO_M;
    } else {
        p.q = q_metric;
        p.tw_wsel = y_ds_metric;
    }
    normalize_culvert_params(&mut p);
    let result = solve_culvert(&p);
    Some(wsel_user_to_metric(result.wsel, p.units))
}

/// Headwater residual at metric faces. Uses inlet fast path when applicable, else full `solve_culvert`.
pub fn culvert_headwater_residual(
    y_us_metric: f64,
    y_ds_metric: f64,
    q_metric: f64,
    params: &CulvertSolveParams,
) -> Option<CulvertHeadwaterResidual> {
    if let Some(wsel_inlet_metric) = culvert_inlet_wsel_metric(y_ds_metric, q_metric, params) {
        let r = y_us_metric - wsel_inlet_metric;
        let dq_metric = (q_metric.abs() * 1e-4).max(1e-4);
        let q_lo = (q_metric - dq_metric).max(1e-6);
        let q_hi = q_metric + dq_metric;
        let w_lo =
            culvert_inlet_wsel_metric(y_ds_metric, q_lo, params).unwrap_or(wsel_inlet_metric);
        let w_hi =
            culvert_inlet_wsel_metric(y_ds_metric, q_hi, params).unwrap_or(wsel_inlet_metric);
        let dr_dq = -(w_hi - w_lo) / (q_hi - q_lo);
        return Some(CulvertHeadwaterResidual {
            r,
            dr_dy_us: 1.0,
            dr_dy_ds: 0.0,
            dr_dq,
        });
    }

    let hw = culvert_solve_headwater_metric(y_us_metric, y_ds_metric, q_metric, params)?;
    let r = y_us_metric - hw;
    let dy = if params.units == UnitSystem::USCustomary {
        0.01 * FT_TO_M
    } else {
        0.003
    };

    let hw_us_p = culvert_solve_headwater_metric(y_us_metric + dy, y_ds_metric, q_metric, params)
        .unwrap_or(hw);
    let hw_us_m = culvert_solve_headwater_metric(y_us_metric - dy, y_ds_metric, q_metric, params)
        .unwrap_or(hw);
    let dr_dy_us = 1.0 - (hw_us_p - hw_us_m) / (2.0 * dy);

    let hw_ds_p = culvert_solve_headwater_metric(y_us_metric, y_ds_metric + dy, q_metric, params)
        .unwrap_or(hw);
    let hw_ds_m = culvert_solve_headwater_metric(y_us_metric, y_ds_metric - dy, q_metric, params)
        .unwrap_or(hw);
    let dr_dy_ds = -(hw_ds_p - hw_ds_m) / (2.0 * dy);

    let dq_metric = (q_metric.abs() * 1e-4).max(1e-4);
    let hw_q_p =
        culvert_solve_headwater_metric(y_us_metric, y_ds_metric, q_metric + dq_metric, params)
            .unwrap_or(hw);
    let hw_q_m = culvert_solve_headwater_metric(
        y_us_metric,
        y_ds_metric,
        (q_metric - dq_metric).max(1e-6),
        params,
    )
    .unwrap_or(hw);
    let dr_dq = -(hw_q_p - hw_q_m) / (2.0 * dq_metric);

    Some(CulvertHeadwaterResidual {
        r,
        dr_dy_us,
        dr_dy_ds,
        dr_dq,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Standard US circular pipe case used across regression tests.
    fn us_circular_baseline() -> CulvertSolveParams {
        CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 1,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 1,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,

            chart_number: None,
            scale_number: None,
            ..Default::default()
        }
    }

    #[test]
    fn culvert_headwater_residual_matches_solve_culvert_outlet() {
        let mut params = us_circular_baseline();
        params.tw_wsel = 16.0;
        let solved = solve_culvert(&params);
        assert_eq!(solved.control_type, "outlet");
        let y_ds = if params.units == UnitSystem::Metric {
            params.tw_wsel
        } else {
            params.tw_wsel * FT_TO_M
        };
        let q = if params.units == UnitSystem::Metric {
            params.q
        } else {
            params.q * CFS_TO_CMS
        };
        let y_us = if params.units == UnitSystem::Metric {
            solved.wsel
        } else {
            solved.wsel * FT_TO_M
        };
        let residual = culvert_headwater_residual(y_us, y_ds, q, &params).expect("outlet residual");
        assert!(residual.r.abs() < 0.05);
        assert!(
            residual.dr_dy_ds.abs() > 1e-6,
            "outlet HW should depend on tailwater"
        );
    }

    #[test]
    fn culvert_implicit_eligible_includes_conspan_shape() {
        let mut params = us_circular_baseline();
        params.shape_type = 3;
        assert!(culvert_implicit_eligible(&params));
        assert!(!culvert_implicit_inlet_eligible(&params));
    }

    #[test]
    fn culvert_headwater_residual_matches_solve_culvert() {
        let params = us_circular_baseline();
        let solved = solve_culvert(&params);
        assert_eq!(solved.control_type, "inlet");
        let y_ds = if params.units == UnitSystem::Metric {
            params.tw_wsel
        } else {
            params.tw_wsel * FT_TO_M
        };
        let q = if params.units == UnitSystem::Metric {
            params.q
        } else {
            params.q * CFS_TO_CMS
        };
        let y_us = if params.units == UnitSystem::Metric {
            solved.wsel
        } else {
            solved.wsel * FT_TO_M
        };
        let residual = culvert_headwater_residual(y_us, y_ds, q, &params).expect("inlet residual");
        assert!(residual.r.abs() < 0.05);
    }

    #[test]
    fn test_solve_culvert_from_headwater_roundtrip() {
        let base = us_circular_baseline();
        let forward = solve_culvert(&base);
        let (tw_recovered, inverse) = solve_culvert_from_headwater(&base, forward.wsel);
        assert!((inverse.wsel - forward.wsel).abs() < 0.05);
        assert!(tw_recovered <= base.tw_wsel + 1e-3);

        let mut outlet = us_circular_baseline();
        outlet.tw_wsel = 16.0;
        let forward_outlet = solve_culvert(&outlet);
        assert_eq!(forward_outlet.control_type, "outlet");
        let (tw_out, inverse_out) = solve_culvert_from_headwater(&outlet, forward_outlet.wsel);
        assert!((inverse_out.wsel - forward_outlet.wsel).abs() < 0.05);
        assert!((tw_out - outlet.tw_wsel).abs() < 0.1);
    }

    #[test]
    fn test_solve_culvert_from_headwater_edge_cases() {
        let mut early = us_circular_baseline();
        early.q = 800.0;
        let (tw_early, result_early) = solve_culvert_from_headwater(&early, 8.0);
        assert!(result_early.wsel > 8.0);
        assert!((tw_early - early.z_down).abs() < 0.05);

        let mut bisect = us_circular_baseline();
        bisect.q = 200.0;
        let (tw, result) = solve_culvert_from_headwater(&bisect, 18.0);
        assert!((result.wsel - 18.0).abs() < 0.1);
        assert!(tw >= bisect.z_down);
    }

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
        assert_eq!(
            get_culvert_perimeter(CulvertShape::Box, 6.0, 4.0, 2.0),
            10.0
        ); // W + 2y
        assert_eq!(
            get_culvert_perimeter(CulvertShape::Box, 6.0, 4.0, 4.0),
            20.0
        ); // 2W + 2D

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
            0.012, // manning_n_bottom
            0.0,   // depth_bottom_n
            0.0,   // depth_blocked
            0.0,   // ds_velocity
            0.0,   // us_velocity
        );

        // Under inlet control, HW depth above inlet invert is ~4.25 ft.
        // So WSEL_up should be ~14.25 ft. Let's verify.
        let hw_depth_low = wsel_up_low - 10.0;
        assert!(
            (hw_depth_low - 4.25).abs() < 0.05,
            "expected ~4.25, got {}",
            hw_depth_low
        );

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
            0.012, // manning_n_bottom
            0.0,   // depth_bottom_n
            0.0,   // depth_blocked
            0.0,   // ds_velocity
            0.0,   // us_velocity
        );

        // Under outlet control, WSEL_up should be TW + losses = 15.0 + 0.726 = 15.726 ft.
        let hw_depth_high = wsel_up_high - 10.0;
        assert!(
            (hw_depth_high - 5.73).abs() < 0.05,
            "expected ~5.73, got {}",
            hw_depth_high
        );
    }

    #[test]
    fn test_explicit_inlet_type_differs_from_legacy() {
        let base = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 0,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let legacy = solve_culvert(&base).wsel;
        let mut projecting = base.clone();
        projecting.inlet_type = 4;
        let explicit = solve_culvert(&projecting).wsel;
        assert!((legacy - explicit).abs() > 0.01);
    }

    #[test]
    fn test_culvert_invert_override_raises_headwater() {
        let params = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 0,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 12.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let bed = CulvertSolveParams {
            z_up: 10.0,
            ..params.clone()
        };
        let raised = solve_culvert(&params);
        let bed_result = solve_culvert(&bed);
        assert!(raised.wsel > bed_result.wsel);
    }

    #[test]
    fn test_roadway_overtopping_control() {
        let params = CulvertSolveParams {
            q: 500.0,
            shape_type: 0,
            inlet_type: 1,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 10.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: Some(14.0),
            weir_coeff: 2.6,
            weir_length: 20.0,
            num_barrels: 2,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let result = solve_culvert(&params);
        assert!(result.wsel > 14.0);
        assert_eq!(result.control_type, "overtopping");
    }

    #[test]
    fn test_barrel_control_type_reporting() {
        let params = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 0,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        assert_eq!(solve_culvert(&params).control_type, "inlet");

        let mut outlet_params = params.clone();
        outlet_params.tw_wsel = 15.0;
        assert_eq!(solve_culvert(&outlet_params).control_type, "outlet");
    }

    #[test]
    fn test_conspan_arch_geometry() {
        let span = 28.0;
        let rise = 6.0; // greater than default arch height 5.76, so wall height is 0.24 ft.

        // At y = 0.24 (exactly wall height)
        let area_at_wall = get_culvert_area(CulvertShape::ConspanArch, span, rise, 0.24);
        assert!((area_at_wall - 28.0 * 0.24).abs() < 1e-6);

        let p_at_wall = get_culvert_perimeter(CulvertShape::ConspanArch, span, rise, 0.24);
        assert!((p_at_wall - (28.0 + 2.0 * 0.24)).abs() < 1e-6);

        let t_at_wall = get_culvert_top_width(CulvertShape::ConspanArch, span, rise, 0.24);
        assert_eq!(t_at_wall, 28.0);

        // At full rise — use manufacturer table area (not parabolic arch approximation)
        let area_full = get_culvert_area(CulvertShape::ConspanArch, span, rise, 6.0);
        let expected_full = CONSPAN_28X6_TABLE[CONSPAN_28X6_TABLE.len() - 1].area;
        assert!((area_full - expected_full).abs() < 1e-3);
    }

    #[test]
    fn test_inlet_nomograph_all_types() {
        let legacy_groove = inlet_nomograph_coeffs(CulvertShape::Circular, 0, 0.1);
        let square_circ = inlet_nomograph_coeffs(CulvertShape::Circular, 1, 0.1);
        assert_ne!(legacy_groove, square_circ);

        let box_square = inlet_nomograph_coeffs(CulvertShape::Box, 10, 0.5);
        let box_wing = inlet_nomograph_coeffs(CulvertShape::Box, 11, 0.2);
        assert_ne!(box_square, box_wing);

        let arch_proj = inlet_nomograph_coeffs(CulvertShape::Arch, 20, 0.5);
        let arch_smooth = inlet_nomograph_coeffs(CulvertShape::Arch, 21, 0.2);
        assert_ne!(arch_proj, arch_smooth);

        // Unknown code falls back to legacy
        let fallback = inlet_nomograph_coeffs(CulvertShape::Box, 99, 0.5);
        assert_eq!(fallback, inlet_nomograph_coeffs(CulvertShape::Box, 0, 0.5));
    }

    #[test]
    fn test_fhwa_nomograph_lookup() {
        // Chart 1 Scale 1 (circular concrete pipe - square edge w/ headwall)
        let c1s1 = fhwa_nomograph_coeffs(1, 1).unwrap();
        assert_eq!(c1s1, (0.0098, 2.00, 0.0398, 0.67, false));

        // Chart 8 Scale 2 (box culvert - 90 or 15 deg flares)
        let c8s2 = fhwa_nomograph_coeffs(8, 2).unwrap();
        assert_eq!(c8s2, (0.0610, 0.75, 0.0400, 0.80, false));

        // Chart 9 Scale 1 (box culvert, flared wingwalls & top edge bevel - Form 2)
        let c9s1 = fhwa_nomograph_coeffs(9, 1).unwrap();
        assert_eq!(c9s1, (0.5100, 0.667, 0.0309, 0.80, true));

        // Invalid chart/scale returns None
        assert!(fhwa_nomograph_coeffs(99, 1).is_none());
        assert!(fhwa_nomograph_coeffs(1, 99).is_none());
    }

    #[test]
    fn test_fhwa_chart_selection_solves() {
        let mut params = us_circular_baseline();

        // Solve with legacy inlet_type = 1 (K=0.0098, M=2.00, c=0.0398, Y=0.67)
        params.inlet_type = 1;
        let legacy_wsel = solve_culvert(&params).wsel;

        // Reset inlet_type and solve with direct Chart 1 / Scale 1
        params.inlet_type = 0;
        params.chart_number = Some(1);
        params.scale_number = Some(1);
        let direct_wsel = solve_culvert(&params).wsel;

        // They must match exactly!
        assert!((legacy_wsel - direct_wsel).abs() < 1e-6);

        // Solve with Form 2 Chart 9 Scale 1 (box culvert with top edge bevel)
        let mut box_params = us_circular_baseline();
        box_params.shape_type = 1; // Box
        box_params.chart_number = Some(9);
        box_params.scale_number = Some(1);
        let box_wsel = solve_culvert(&box_params).wsel;
        assert!(box_wsel > 10.0);
    }

    #[test]
    fn test_crest_set_but_barrel_controls() {
        let params = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 0,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: Some(20.0),
            weir_coeff: 2.6,
            weir_length: 10.0,
            num_barrels: 1,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let result = solve_culvert(&params);
        assert!(result.wsel < 20.0);
        assert_eq!(result.control_type, "inlet");
    }

    #[test]
    fn test_extended_diagnostics_inlet_control() {
        let params = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 0,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let result = solve_culvert(&params);
        assert_eq!(result.control_type, "inlet");
        assert!((result.wsel - result.wsel_inlet).abs() < 1e-6);
        assert!(result.wsel_outlet < result.wsel_inlet);
        assert!((result.q_barrel - 100.0).abs() < 1e-6);
        assert!(result.q_weir.abs() < 1e-6);
        assert!(result.barrel_depth > 0.0);
        assert!(result.barrel_velocity > 0.0);
        assert!(result.barrel_froude > 0.0);
    }

    #[test]
    fn test_adverse_barrel_slope_increases_headwater() {
        let base = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 1,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 10.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let flat = CulvertSolveParams {
            z_down: 10.0,
            ..base.clone()
        };
        let adverse = CulvertSolveParams {
            z_down: 11.0,
            ..base.clone()
        };
        let downhill = CulvertSolveParams {
            z_down: 9.0,
            ..base
        };
        let flat_hw = solve_culvert(&flat).wsel;
        let adverse_hw = solve_culvert(&adverse).wsel;
        let downhill_hw = solve_culvert(&downhill).wsel;
        assert!(
            adverse_hw > flat_hw && flat_hw > downhill_hw,
            "adverse={} flat={} downhill={}",
            adverse_hw,
            flat_hw,
            downhill_hw
        );
    }

    #[test]
    fn test_culvert_rating_curve() {
        let inputs = CulvertRatingCurveInputs {
            q_values: vec![50.0, 100.0, 150.0],
            culvert: CulvertSolveParams {
                q: 0.0,
                shape_type: 0,
                inlet_type: 1,
                span: 5.0,
                rise: 5.0,
                roughness_n: 0.012,
                length: 100.0,
                entrance_loss_coeff: 0.5,
                exit_loss_coeff: 1.0,
                z_down: 9.0,
                z_up: 10.0,
                tw_wsel: 12.0,
                units: UnitSystem::USCustomary,
                manning_n_bottom: 0.012,
                depth_bottom_n: 0.0,
                depth_blocked: 0.0,
                ds_velocity: 0.0,
                us_velocity: 0.0,
                crest_elev: None,
                weir_coeff: 0.0,
                weir_length: 0.0,
                num_barrels: 1,
                active_barrels: 0,
                skew_deg: 0.0,
                barrel_spans: None,
                barrel_rises: None,
                custom_shape_tbl_y: None,
                custom_shape_tbl_area: None,
                custom_shape_tbl_perimeter: None,
                custom_shape_tbl_top_width: None,
                roadway_stations: None,
                roadway_elevations: None,
                chart_number: None,
                scale_number: None,
                ..Default::default()
            },
        };
        let curve = compute_culvert_rating_curve(&inputs);
        assert_eq!(curve.q.len(), 3);
        assert!(curve.wsel[1] > curve.wsel[0]);
        assert!(curve.wsel[2] > curve.wsel[1]);
        assert_eq!(curve.q_barrel[0], 50.0);
    }

    #[test]
    fn test_overtopping_reports_flow_split() {
        let params = CulvertSolveParams {
            q: 500.0,
            shape_type: 0,
            inlet_type: 1,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 10.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: Some(14.0),
            weir_coeff: 2.6,
            weir_length: 20.0,
            num_barrels: 2,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let result = solve_culvert(&params);
        assert_eq!(result.control_type, "overtopping");
        assert!(result.q_weir > 0.0);
        assert!((result.q_barrel + result.q_weir - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_apply_barrel_skew_geometry() {
        let (span, len) = apply_barrel_skew(0.0, 10.0, 100.0);
        assert!((span - 10.0).abs() < 1e-6);
        assert!((len - 100.0).abs() < 1e-6);
        let (span30, len30) = apply_barrel_skew(30.0, 10.0, 100.0);
        assert!((span30 - 10.0).abs() < 1e-6);
        assert!((len30 - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_skew_increases_outlet_control_headwater() {
        let base = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 1,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 15.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let mut skewed = base.clone();
        skewed.skew_deg = 30.0;
        let hw_plain = solve_culvert(&base).wsel;
        let hw_skew = solve_culvert(&skewed).wsel;
        assert_eq!(solve_culvert(&base).control_type, "outlet");
        assert!(
            (hw_skew - hw_plain).abs() < 1e-6,
            "skew={} plain={}",
            hw_skew,
            hw_plain
        );
    }

    #[test]
    fn test_blocked_barrel_increases_headwater() {
        let base = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 1,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 2,
            active_barrels: 2,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let mut blocked = base.clone();
        blocked.active_barrels = 1;
        assert!(solve_culvert(&blocked).wsel > solve_culvert(&base).wsel);
    }

    #[test]
    fn test_unequal_barrel_geometry_lowers_headwater() {
        let small_only = CulvertSolveParams {
            q: 120.0,
            shape_type: 0,
            inlet_type: 1,
            span: 4.0,
            rise: 4.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 2,
            active_barrels: 2,
            skew_deg: 0.0,
            barrel_spans: Some(vec![8.0, 4.0]),
            barrel_rises: Some(vec![8.0, 4.0]),
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let equal_small = CulvertSolveParams {
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,

            chart_number: None,

            scale_number: None,
            ..small_only.clone()
        };
        let hw_mixed = solve_culvert(&small_only).wsel;
        let hw_equal_small = solve_culvert(&equal_small).wsel;
        assert!(
            hw_mixed < hw_equal_small,
            "mixed barrels should need less head than two small barrels: mixed={} equal_small={}",
            hw_mixed,
            hw_equal_small
        );
    }

    #[test]
    fn test_per_barrel_geometry_matches_uniform_multi_barrel() {
        let uniform = CulvertSolveParams {
            q: 100.0,
            shape_type: 0,
            inlet_type: 1,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 2,
            active_barrels: 2,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let explicit = CulvertSolveParams {
            barrel_spans: Some(vec![5.0, 5.0]),
            barrel_rises: Some(vec![5.0, 5.0]),
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,

            chart_number: None,

            scale_number: None,
            ..uniform.clone()
        };
        let hw_uniform = solve_culvert(&uniform).wsel;
        let hw_explicit = solve_culvert(&explicit).wsel;
        assert!(
            (hw_uniform - hw_explicit).abs() < 0.05,
            "uniform={} explicit={}",
            hw_uniform,
            hw_explicit
        );
    }

    #[test]
    fn test_effective_geometry_blockage_reduces_area() {
        let shape = CulvertShape::Box;
        let full = get_culvert_area(shape, 6.0, 4.0, 3.0);
        let blocked = get_culvert_effective_area(shape, 6.0, 4.0, 3.0, 1.0);
        assert!(blocked < full);
        assert!(
            (blocked - get_culvert_area(shape, 6.0, 4.0, 3.0)
                + get_culvert_area(shape, 6.0, 4.0, 1.0))
            .abs()
                < 1e-6
        );
        assert_eq!(get_culvert_effective_area(shape, 6.0, 4.0, 0.5, 1.0), 0.0);
    }

    #[test]
    fn test_composite_n_between_top_and_bottom() {
        let n = get_culvert_composite_n(CulvertShape::Box, 6.0, 4.0, 3.0, 0.0, 0.012, 0.030, 1.0);
        assert!(n > 0.012 && n < 0.030);
        assert_eq!(
            get_culvert_composite_n(CulvertShape::Box, 6.0, 4.0, 3.0, 0.0, 0.012, 0.012, 1.0),
            0.012
        );
    }

    #[test]
    fn test_barrel_critical_depth_within_rise() {
        let yc = solve_barrel_critical_depth(CulvertShape::Circular, 5.0, 5.0, 50.0, 0.0);
        assert!(yc > 0.0 && yc <= 5.0);
        let yc_blocked = solve_barrel_critical_depth(CulvertShape::Circular, 5.0, 5.0, 50.0, 1.0);
        assert!(yc_blocked >= 1.0);
    }

    #[test]
    fn test_sediment_blockage_increases_headwater() {
        let clear = solve_culvert(&us_circular_baseline());
        let mut blocked = us_circular_baseline();
        blocked.depth_blocked = 1.0;
        assert!(solve_culvert(&blocked).wsel > clear.wsel);
    }

    #[test]
    fn test_composite_bottom_n_increases_headwater() {
        let mut outlet = us_circular_baseline();
        outlet.tw_wsel = 15.0;
        let uniform = solve_culvert(&outlet).wsel;
        outlet.manning_n_bottom = 0.030;
        outlet.depth_bottom_n = 2.0;
        assert!(solve_culvert(&outlet).wsel > uniform);
    }

    #[test]
    fn test_channel_velocities_affect_outlet_headwater() {
        let mut base = us_circular_baseline();
        base.tw_wsel = 14.0;
        let still = solve_culvert(&base);
        assert_eq!(still.control_type, "outlet");

        // Higher approach velocity head subtracts from outlet-control WSEL.
        base.us_velocity = 6.0;
        assert!(solve_culvert(&base).wsel_outlet < still.wsel_outlet);

        // Downstream velocity recovery reduces the exit-loss term under outlet control.
        let mut partial = us_circular_baseline();
        partial.tw_wsel = 14.0;
        let no_ds = solve_culvert(&partial).wsel_outlet;
        partial.ds_velocity = 4.0;
        assert!(solve_culvert(&partial).wsel_outlet <= no_ds);
    }

    #[test]
    fn test_entrance_and_exit_loss_increase_headwater() {
        let base = us_circular_baseline();
        let mut outlet = base.clone();
        outlet.tw_wsel = 15.0;
        let low_ke = solve_culvert(&outlet).wsel;
        outlet.entrance_loss_coeff = 1.5;
        assert!(solve_culvert(&outlet).wsel > low_ke);

        let mut outlet2 = base.clone();
        outlet2.tw_wsel = 15.0;
        let low_kx = solve_culvert(&outlet2).wsel;
        outlet2.exit_loss_coeff = 2.5;
        assert!(solve_culvert(&outlet2).wsel > low_kx);
    }

    #[test]
    fn test_longer_barrel_and_higher_n_increase_outlet_headwater() {
        let mut outlet = us_circular_baseline();
        outlet.tw_wsel = 15.0;
        let short = solve_culvert(&outlet).wsel;
        outlet.length = 250.0;
        assert!(solve_culvert(&outlet).wsel > short);

        outlet.length = 100.0;
        outlet.roughness_n = 0.020;
        outlet.manning_n_bottom = 0.020;
        assert!(solve_culvert(&outlet).wsel > short);
    }

    #[test]
    fn test_skew_angle_clamped_at_59_degrees() {
        let (s59, l59) = apply_barrel_skew(59.0, 10.0, 100.0);
        let (s70, l70) = apply_barrel_skew(70.0, 10.0, 100.0);
        assert!((s59 - s70).abs() < 1e-6);
        assert!((l59 - l70).abs() < 1e-6);
    }

    #[test]
    fn test_extended_shape_geometry() {
        let span = 8.0;
        let rise = 6.0;
        for shape in [
            CulvertShape::PipeArch,
            CulvertShape::Elliptical,
            CulvertShape::Horseshoe,
        ] {
            let a_half = get_culvert_area(shape, span, rise, rise / 2.0);
            let a_full = get_culvert_area(shape, span, rise, rise);
            assert!(a_half > 0.0 && a_full > a_half);
            assert!(a_full < span * rise);
            assert!(get_culvert_top_width(shape, span, rise, rise / 2.0) > 0.0);
            assert!(get_culvert_perimeter(shape, span, rise, rise) > span);
        }
        // Ellipse full area ≈ πab
        let a_ellipse = get_culvert_area(CulvertShape::Elliptical, 8.0, 6.0, 6.0);
        let expected = std::f64::consts::PI * 4.0 * 3.0;
        assert!((a_ellipse - expected).abs() / expected < 0.02);
    }

    #[test]
    fn test_all_shapes_produce_physical_headwater() {
        let cases: [(i32, f64, f64); 7] = [
            (0, 5.0, 5.0),  // Circular
            (1, 8.0, 6.0),  // Box
            (2, 8.0, 6.0),  // Arch
            (3, 28.0, 6.0), // ConSpan
            (4, 8.0, 6.0),  // Pipe-arch
            (5, 8.0, 6.0),  // Elliptical
            (6, 8.0, 6.0),  // Horseshoe
        ];
        for (shape, span, rise) in cases {
            let mut p = us_circular_baseline();
            p.shape_type = shape;
            p.span = span;
            p.rise = rise;
            if shape >= 2 {
                p.inlet_type = 21;
            }
            let r = solve_culvert(&p);
            assert!(r.wsel > p.tw_wsel, "shape {} wsel={}", shape, r.wsel);
            assert!((r.q_barrel - p.q).abs() < 1e-6);
            assert!(r.barrel_depth > 0.0);
            assert!(r.barrel_velocity > 0.0);
            assert!(matches!(
                r.control_type.as_str(),
                "inlet" | "outlet" | "overtopping"
            ));
        }
    }

    #[test]
    fn test_all_inlet_types_solve() {
        let circular = [0, 1, 2, 3, 4];
        let box_types = [0, 10, 11, 12];
        let arch_types = [0, 20, 21];
        let mut last_hw = 0.0;
        for inlet in circular {
            let mut p = us_circular_baseline();
            p.inlet_type = inlet;
            let r = solve_culvert(&p);
            assert!(r.wsel > 10.0);
            if inlet > 0 {
                assert!(r.wsel.is_finite());
            }
            last_hw = r.wsel;
        }
        assert!(last_hw > 0.0);

        for inlet in box_types {
            let mut p = us_circular_baseline();
            p.shape_type = 1;
            p.span = 8.0;
            p.rise = 6.0;
            p.inlet_type = inlet;
            assert!(solve_culvert(&p).wsel > p.tw_wsel);
        }
        for inlet in arch_types {
            let mut p = us_circular_baseline();
            p.shape_type = 2;
            p.span = 8.0;
            p.rise = 6.0;
            p.inlet_type = inlet;
            assert!(solve_culvert(&p).wsel > p.tw_wsel);
        }
    }

    #[test]
    fn test_box_culvert_inlet_and_outlet_regimes() {
        let mut inlet_case = us_circular_baseline();
        inlet_case.shape_type = 1;
        inlet_case.span = 8.0;
        inlet_case.rise = 6.0;
        inlet_case.inlet_type = 11;
        assert_eq!(solve_culvert(&inlet_case).control_type, "inlet");

        inlet_case.tw_wsel = 15.0;
        let outlet = solve_culvert(&inlet_case);
        assert_eq!(outlet.control_type, "outlet");
        assert!((outlet.wsel - outlet.wsel_outlet).abs() < 1e-4);
    }

    #[test]
    fn test_metric_units_circular_inlet_control() {
        let mut p = us_circular_baseline();
        p.units = UnitSystem::Metric;
        p.q = 2.83;
        p.span = 1.524;
        p.rise = 1.524;
        p.length = 30.48;
        p.z_down = 2.743;
        p.z_up = 3.048;
        p.tw_wsel = 3.658;
        let r = solve_culvert(&p);
        assert_eq!(r.control_type, "inlet");
        let hw_depth = r.wsel - p.z_up;
        assert!(hw_depth > 1.0 && hw_depth < 2.0);
    }

    #[test]
    fn test_partial_overtopping_splits_barrel_and_weir_flow() {
        let mut p = us_circular_baseline();
        p.crest_elev = Some(14.15);
        p.weir_coeff = 2.6;
        p.weir_length = 5.0;
        let r = solve_culvert(&p);
        assert!(r.q_barrel > 0.0);
        assert!(r.q_weir > 0.0);
        assert!((r.q_barrel + r.q_weir - p.q).abs() < 1.0);
        assert!(r.wsel > 14.15);
    }

    #[test]
    fn test_default_weir_length_matches_explicit_barrel_spans() {
        let mut auto = us_circular_baseline();
        auto.num_barrels = 2;
        auto.active_barrels = 2;
        auto.crest_elev = Some(14.15);
        auto.weir_coeff = 2.6;
        auto.weir_length = 0.0;
        let r_auto = solve_culvert(&auto);

        let mut explicit = auto.clone();
        explicit.weir_length = 10.0;
        let r_explicit = solve_culvert(&explicit);
        assert!(
            (r_auto.wsel - r_explicit.wsel).abs() < 0.05,
            "auto={} explicit={}",
            r_auto.wsel,
            r_explicit.wsel
        );
    }

    #[test]
    fn test_skew_with_unequal_barrel_geometry() {
        let mut p = us_circular_baseline();
        p.num_barrels = 2;
        p.active_barrels = 2;
        p.barrel_spans = Some(vec![6.0, 4.0]);
        p.barrel_rises = Some(vec![6.0, 4.0]);
        let no_skew = solve_culvert(&p).wsel;
        p.skew_deg = 25.0;
        assert!((solve_culvert(&p).wsel - no_skew).abs() < 1e-6);
    }

    #[test]
    fn test_multi_barrel_conserves_total_discharge() {
        let mut p = us_circular_baseline();
        p.num_barrels = 3;
        p.active_barrels = 3;
        p.barrel_spans = Some(vec![5.0, 6.0, 4.0]);
        p.barrel_rises = Some(vec![5.0, 6.0, 4.0]);
        let r = solve_culvert(&p);
        assert!((r.q_barrel - p.q).abs() < 1.0);
    }

    #[test]
    fn test_rating_curve_all_shapes_monotonic() {
        let shapes: [(i32, f64, f64, i32); 7] = [
            (0, 5.0, 5.0, 1),
            (1, 8.0, 6.0, 10),
            (2, 8.0, 6.0, 21),
            (3, 28.0, 6.0, 20),
            (4, 8.0, 6.0, 21),
            (5, 8.0, 6.0, 21),
            (6, 8.0, 6.0, 21),
        ];
        for (shape, span, rise, inlet) in shapes {
            let mut base = us_circular_baseline();
            base.shape_type = shape;
            base.span = span;
            base.rise = rise;
            base.inlet_type = inlet;
            base.q = 0.0;
            let curve = compute_culvert_rating_curve(&CulvertRatingCurveInputs {
                q_values: vec![25.0, 50.0, 100.0],
                culvert: base,
            });
            assert_eq!(curve.wsel.len(), 3);
            assert!(curve.wsel[1] > curve.wsel[0]);
            assert!(curve.wsel[2] > curve.wsel[1]);
            assert!(curve.barrel_froude.iter().all(|f| *f > 0.0));
        }
    }

    #[test]
    fn test_extended_diagnostics_outlet_control() {
        let mut p = us_circular_baseline();
        p.tw_wsel = 15.0;
        let r = solve_culvert(&p);
        assert_eq!(r.control_type, "outlet");
        assert!((r.wsel - r.wsel_outlet).abs() < 1e-4);
        assert!(r.wsel_inlet < r.wsel_outlet);
        assert!(r.barrel_depth > 0.0);
        assert!(r.barrel_velocity > 0.0);
    }

    #[test]
    fn test_metric_overtopping_control() {
        let params = CulvertSolveParams {
            q: 15.0,
            shape_type: 1,
            inlet_type: 10,
            span: 2.0,
            rise: 1.5,
            roughness_n: 0.013,
            length: 30.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 0.0,
            z_up: 0.1,
            tw_wsel: 0.5,
            units: UnitSystem::Metric,
            manning_n_bottom: 0.013,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: Some(1.2),
            weir_coeff: 1.44,
            weir_length: 4.0,
            num_barrels: 1,
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };
        let result = solve_culvert(&params);
        assert!(result.wsel > 1.2);
        assert_eq!(result.control_type, "overtopping");
    }

    #[test]
    fn test_overtopping_with_submergence_reduces_weir_flow() {
        // Case 1: Unsubmerged overtopping
        let mut p1 = us_circular_baseline();
        p1.crest_elev = Some(14.15);
        p1.weir_coeff = 2.6;
        p1.weir_length = 5.0;
        p1.tw_wsel = 12.0; // below crest
        let r1 = solve_culvert(&p1);

        // Case 2: Submerged overtopping
        let mut p2 = p1.clone();
        p2.tw_wsel = 14.5; // above crest, creating submergence
        let r2 = solve_culvert(&p2);

        // Since the weir flow is submerged, it should be less efficient.
        // Therefore, the upstream WSEL (r2.wsel) must be higher than r1.wsel to pass the same total flow (q = 100.0).
        assert!(
            r2.wsel > r1.wsel,
            "Submerged WSEL {} should be higher than unsubmerged WSEL {}",
            r2.wsel,
            r1.wsel
        );

        // Let's also verify that for the same headwater, weir flow is reduced under submergence.
        // E.g. at wsel = 15.0:
        // Unsubmerged weir flow (crest 14.15, tw 12.0)
        let q_unsub = weir_flow_us(2.6, 5.0, 15.0, 14.15, 12.0);
        // Submerged weir flow (crest 14.15, tw 14.4)
        let q_sub = weir_flow_us(2.6, 5.0, 15.0, 14.15, 14.4);
        assert!(
            q_sub < q_unsub,
            "Submerged weir flow {} should be less than unsubmerged {}",
            q_sub,
            q_unsub
        );
    }

    #[test]
    fn test_custom_shape_matches_box() {
        let mut ys = Vec::new();
        let mut areas = Vec::new();
        let mut perims = Vec::new();
        let mut widths = Vec::new();
        let steps = 40;
        let dy = 4.0 / steps as f64;
        for i in 0..=steps {
            let y = i as f64 * dy;
            ys.push(y);
            areas.push(y * 6.0);
            let p = if y >= 4.0 {
                2.0 * 6.0 + 2.0 * 4.0
            } else {
                6.0 + 2.0 * y
            };
            perims.push(p);
            let w = if y >= 4.0 { 0.0 } else { 6.0 };
            widths.push(w);
        }

        let mut box_params = CulvertSolveParams {
            q: 120.0,
            shape_type: 1, // Box
            inlet_type: 1,
            span: 6.0,
            rise: 4.0,
            roughness_n: 0.013,
            length: 80.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 10.0,
            z_up: 11.0,
            tw_wsel: 13.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.013,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 1,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            roadway_elevations: None,
            chart_number: None,
            scale_number: None,
            ..Default::default()
        };

        let mut custom_params = box_params.clone();
        custom_params.shape_type = 7; // Custom
        custom_params.custom_shape_tbl_y = Some(ys);
        custom_params.custom_shape_tbl_area = Some(areas);
        custom_params.custom_shape_tbl_perimeter = Some(perims);
        custom_params.custom_shape_tbl_top_width = Some(widths);

        // Test case 1: Partially full outlet flow (low tailwater)
        let solved_box_part = solve_culvert(&box_params);
        let solved_custom_part = solve_culvert(&custom_params);
        assert_eq!(
            solved_box_part.control_type,
            solved_custom_part.control_type
        );
        assert!((solved_box_part.wsel - solved_custom_part.wsel).abs() < 0.01);

        // Test case 2: Fully submerged outlet flow (high tailwater)
        box_params.tw_wsel = 16.0;
        custom_params.tw_wsel = 16.0;
        let solved_box_full = solve_culvert(&box_params);
        let solved_custom_full = solve_culvert(&custom_params);
        assert_eq!(
            solved_box_full.control_type,
            solved_custom_full.control_type
        );
        assert!((solved_box_full.wsel - solved_custom_full.wsel).abs() < 0.01);

        // Cover get_culvert_* wildcard Custom match arms
        assert_eq!(get_culvert_area(CulvertShape::Custom, 6.0, 4.0, 2.0), 0.0);
        assert_eq!(
            get_culvert_top_width(CulvertShape::Custom, 6.0, 4.0, 2.0),
            0.0
        );
        assert_eq!(
            get_culvert_perimeter(CulvertShape::Custom, 6.0, 4.0, 2.0),
            0.0
        );

        // Cover entrance_loss_coeff <= 0.2 branch in inlet_nomograph_coeffs for Custom
        let coeffs_low = inlet_nomograph_coeffs(CulvertShape::Custom, 0, 0.1);
        assert_eq!(coeffs_low.0, 0.026);

        // Cover Metric unit scaling in CulvertGeometry
        let mut metric_params = custom_params.clone();
        metric_params.units = UnitSystem::Metric;
        metric_params.span = 6.0 * FT_TO_M;
        metric_params.rise = 4.0 * FT_TO_M;
        metric_params.z_down = 10.0 * FT_TO_M;
        metric_params.z_up = 11.0 * FT_TO_M;
        metric_params.tw_wsel = 13.0 * FT_TO_M;
        metric_params.q = 120.0 * CFS_TO_CMS;
        metric_params.custom_shape_tbl_y = Some(
            custom_params
                .custom_shape_tbl_y
                .as_ref()
                .unwrap()
                .iter()
                .map(|&val| val * FT_TO_M)
                .collect(),
        );
        metric_params.custom_shape_tbl_area = Some(
            custom_params
                .custom_shape_tbl_area
                .as_ref()
                .unwrap()
                .iter()
                .map(|&val| val * FT_TO_M * FT_TO_M)
                .collect(),
        );
        metric_params.custom_shape_tbl_perimeter = Some(
            custom_params
                .custom_shape_tbl_perimeter
                .as_ref()
                .unwrap()
                .iter()
                .map(|&val| val * FT_TO_M)
                .collect(),
        );
        metric_params.custom_shape_tbl_top_width = Some(
            custom_params
                .custom_shape_tbl_top_width
                .as_ref()
                .unwrap()
                .iter()
                .map(|&val| val * FT_TO_M)
                .collect(),
        );
        let solved_metric = solve_culvert(&metric_params);
        assert!(solved_metric.wsel > 0.0);

        // Cover interpolate_custom none fields / empty / short table branches
        let mut empty_params = custom_params.clone();
        empty_params.custom_shape_tbl_y = None;
        let geom_empty = CulvertGeometry::new(&empty_params, 6.0, 4.0);
        assert_eq!(geom_empty.area(2.0), 0.0);

        let mut none_fields = custom_params.clone();
        none_fields.custom_shape_tbl_area = None;
        none_fields.custom_shape_tbl_perimeter = None;
        none_fields.custom_shape_tbl_top_width = None;
        let geom_none = CulvertGeometry::new(&none_fields, 6.0, 4.0);
        assert_eq!(geom_none.area(2.0), 0.0);
        assert_eq!(geom_none.perimeter(2.0), 0.0);
        assert_eq!(geom_none.top_width(2.0), 0.0);

        let mut short_params = custom_params.clone();
        short_params.custom_shape_tbl_y = Some(vec![0.0]);
        let geom_short = CulvertGeometry::new(&short_params, 6.0, 4.0);
        assert_eq!(geom_short.area(2.0), 0.0);

        // Cover y <= d_b branch in effective_area, effective_top_width, effective_perimeter
        let geom_test = CulvertGeometry::new(&custom_params, 6.0, 4.0);
        assert_eq!(geom_test.effective_area(1.0, 2.0), 0.0);
        assert_eq!(geom_test.effective_top_width(1.0, 2.0), 0.0);
        assert_eq!(geom_test.effective_perimeter(1.0, 2.0), 0.0);

        // Cover y <= d_b in composite_n
        assert_eq!(geom_test.composite_n(1.0, 2.0, 0.013, 0.015, 1.0), 0.013);
        assert_eq!(geom_test.composite_n(1.5, 0.0, 0.013, 0.015, 2.0), 0.015);

        // Cover p_total <= 1e-9 in composite_n
        let mut zero_perims = custom_params.clone();
        zero_perims.custom_shape_tbl_perimeter = Some(vec![0.0; 41]);
        let geom_zero_p = CulvertGeometry::new(&zero_perims, 6.0, 4.0);
        assert_eq!(geom_zero_p.composite_n(2.0, 0.0, 0.013, 0.015, 1.0), 0.013);
    }

    #[test]
    fn test_roadway_profile_weir_flow_unit() {
        let stations = vec![0.0, 50.0, 100.0];
        let elevations = vec![10.0, 8.0, 10.0];

        // 1. Invalid inputs
        assert_eq!(
            roadway_profile_weir_flow(2.6, 12.0, 5.0, &vec![0.0], &vec![10.0], 0.0),
            0.0
        );
        assert_eq!(
            roadway_profile_weir_flow(2.6, 12.0, 5.0, &stations, &vec![10.0], 0.0),
            0.0
        );

        // 2. Normal flow (unsubmerged)
        let q_unsub = roadway_profile_weir_flow(2.6, 11.0, 5.0, &stations, &elevations, 0.0);
        assert!(q_unsub > 0.0);

        // 3. Submerged flow (Bradley reduction)
        let q_sub = roadway_profile_weir_flow(2.6, 11.0, 10.8, &stations, &elevations, 0.0);
        assert!(q_sub < q_unsub);

        // 4. Skewed flow
        let q_skew = roadway_profile_weir_flow(2.6, 11.0, 5.0, &stations, &elevations, 30.0);
        assert!(q_skew < q_unsub);

        // 5. Head below crest
        assert_eq!(
            roadway_profile_weir_flow(2.6, 7.0, 5.0, &stations, &elevations, 0.0),
            0.0
        );
    }

    #[test]
    fn test_solve_culvert_with_roadway_profile_unit() {
        // 1. US Customary with profile
        let mut params = us_circular_baseline();
        params.q = 150.0;
        params.roadway_stations = Some(vec![0.0, 50.0, 100.0]);
        params.roadway_elevations = Some(vec![15.0, 12.0, 15.0]);
        params.weir_coeff = 2.6;
        let res_us = solve_culvert(&params);
        assert!(res_us.wsel > 12.0);

        // 2. Metric with profile
        let mut params_metric = us_circular_baseline();
        params_metric.units = UnitSystem::Metric;
        params_metric.q = 5.0; // cms
        params_metric.span = 1.5; // m
        params_metric.rise = 1.5; // m
        params_metric.length = 30.0; // m
        params_metric.z_down = 3.0; // m
        params_metric.z_up = 3.3; // m
        params_metric.tw_wsel = 4.0; // m
        params_metric.roadway_stations = Some(vec![0.0, 15.0, 30.0]);
        params_metric.roadway_elevations = Some(vec![5.0, 4.2, 5.0]);
        params_metric.weir_coeff = 1.44;
        let res_metric = solve_culvert(&params_metric);
        assert!(res_metric.wsel > 4.2);

        // 3. Overtopping only case (weir only overtopping bisection)
        let mut params_overtop = params.clone();
        params_overtop.active_barrels = 0; // force all flow through weir
        let res_overtop = solve_culvert(&params_overtop);
        assert_eq!(res_overtop.control_type, "overtopping");
    }

    #[test]
    fn test_tapered_inlet_throat_and_face_control() {
        let mut params = us_circular_baseline();
        params.q = 150.0;
        params.shape_type = 1; // Box
        params.span = 5.0;
        params.rise = 5.0;
        params.tw_wsel = 8.0;

        params.tapered_type = 0;
        let standard_wsel = solve_culvert(&params).wsel;

        params.tapered_type = 1;
        params.tapered_face_span = Some(8.0);
        params.tapered_face_rise = Some(5.0);
        params.tapered_fall = 1.0;
        params.tapered_throat_chart_number = Some(57);
        params.tapered_throat_scale_number = Some(1);
        params.tapered_face_chart_number = Some(58);
        params.tapered_face_scale_number = Some(1);

        let tapered_wsel = solve_culvert(&params).wsel;
        assert!(tapered_wsel != standard_wsel);
    }

    #[test]
    fn test_slope_tapered_crest_control() {
        let mut params = us_circular_baseline();
        params.q = 50.0;
        params.shape_type = 1; // Box
        params.span = 5.0;
        params.rise = 5.0;
        params.tw_wsel = 8.0;

        params.tapered_type = 2;
        params.tapered_face_span = Some(8.0);
        params.tapered_face_rise = Some(5.0);
        params.tapered_fall = 2.0;
        params.tapered_crest_weir_length = Some(10.0);
        params.tapered_crest_weir_coeff = Some(3.0);
        params.tapered_throat_chart_number = Some(57);
        params.tapered_throat_scale_number = Some(1);
        params.tapered_face_chart_number = Some(59);
        params.tapered_face_scale_number = Some(1);

        let res = solve_culvert(&params);
        assert!(res.wsel > 12.0);
    }

    #[test]
    fn test_gvf_step_drawdown() {
        let mut params = us_circular_baseline();
        params.shape_type = 1; // Box
        params.span = 5.0;
        params.rise = 5.0;
        let geom = CulvertGeometry::new(&params, 5.0, 5.0);
        let q = 100.0;
        let yc = geom.critical_depth(q, 0.0);

        let y2 = solve_gvf_step(
            &geom,
            q,
            0.013,
            0.013,
            0.0,
            0.0,
            4.0,
            10.0,
            10.1,
            10.0,
            true,
            yc,
        );
        assert!(y2 > yc);
        assert!(y2 < 5.0);
    }

    #[test]
    fn test_gvf_outlet_control_pressurized_flow() {
        let mut params = us_circular_baseline();
        params.shape_type = 1; // Box
        params.span = 5.0;
        params.rise = 5.0;
        let geom = CulvertGeometry::new(&params, 5.0, 5.0);
        let q = 150.0;
        let yc = geom.critical_depth(q, 0.0);

        let wsel = compute_gvf_outlet_control(
            &geom,
            q,
            0.013,
            0.013,
            0.0,
            0.0,
            16.0,
            10.0,
            11.0,
            100.0,
            yc,
            0.5,
            1.0,
            0.0,
            0.0,
        );
        assert!(wsel > 16.0);
    }

    #[test]
    fn test_partially_blocked_barrel_increases_headwater() {
        let base = us_circular_baseline();
        let solved_base = solve_culvert(&base);

        let mut blocked = base.clone();
        blocked.depth_blocked = 1.5; // 1.5 ft of sediment blockage at the bottom
        let solved_blocked = solve_culvert(&blocked);

        assert!(
            solved_blocked.wsel > solved_base.wsel,
            "WSEL should increase with partial sediment blockage: base={}, blocked={}",
            solved_base.wsel,
            solved_blocked.wsel
        );
    }

    #[test]
    fn test_roadway_weir_overtopping() {
        let mut params = us_circular_baseline();
        params.q = 100.0; // moderate flow split
        params.crest_elev = Some(13.0); // low crest elevation relative to flow
        params.weir_coeff = 2.6;
        params.weir_length = 2.0; // short weir length to prevent weir-only overtopping

        let solved = solve_culvert(&params);
        println!(
            "DEBUG OVERTOPPING: wsel={}, q_barrel={}, q_weir={}, control={}",
            solved.wsel, solved.q_barrel, solved.q_weir, solved.control_type
        );
        assert_eq!(solved.control_type, "overtopping");
        assert!(solved.q_weir > 0.0, "weir flow should be positive");
        assert!(solved.q_barrel > 0.0, "barrel flow should be positive");
        assert!(
            (solved.q_barrel + solved.q_weir - params.q).abs() < 1e-3,
            "total flow should equal barrel + weir flow"
        );
    }

    #[test]
    fn test_roadway_profile_overtopping() {
        let mut params = us_circular_baseline();
        params.q = 100.0;
        params.roadway_stations = Some(vec![0.0, 50.0, 100.0]);
        params.roadway_elevations = Some(vec![15.0, 13.0, 15.0]);
        params.weir_coeff = 2.6;
        params.weir_length = 2.0;

        let solved = solve_culvert(&params);
        assert_eq!(solved.control_type, "overtopping");
        assert!(solved.q_weir > 0.0);
        assert!(solved.q_barrel > 0.0);
        assert!((solved.q_barrel + solved.q_weir - params.q).abs() < 1e-3);
    }
}
