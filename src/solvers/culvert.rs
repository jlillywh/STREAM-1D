use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS};

/// Standard acceleration due to gravity in English units (ft/s^2).
pub const G_ENGLISH: f64 = 32.17404856;

/// Supported culvert shapes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CulvertShape {
    Circular = 0,
    Box = 1,
    Arch = 2,
    ConspanArch = 3,
}

impl CulvertShape {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => CulvertShape::Box,
            2 => CulvertShape::Arch,
            3 => CulvertShape::ConspanArch,
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
    ConspanTableEntry { y: 0.0, area: 0.0, perimeter: 28.0, top_width: 28.0 },
    ConspanTableEntry { y: 0.25, area: 7.0, perimeter: 28.5, top_width: 28.0 },
    ConspanTableEntry { y: 0.74, area: 20.733, perimeter: 29.48, top_width: 27.94 },
    ConspanTableEntry { y: 1.224, area: 34.185, perimeter: 30.466, top_width: 27.76 },
    ConspanTableEntry { y: 1.693, area: 47.134, perimeter: 31.45, top_width: 27.462 },
    ConspanTableEntry { y: 2.14, area: 59.318, perimeter: 32.435, top_width: 27.05 },
    ConspanTableEntry { y: 2.558, area: 70.517, perimeter: 33.417, top_width: 26.534 },
    ConspanTableEntry { y: 2.942, area: 80.588, perimeter: 34.402, top_width: 25.918 },
    ConspanTableEntry { y: 3.284, area: 89.331, perimeter: 35.385, top_width: 25.212 },
    ConspanTableEntry { y: 3.58, area: 96.676, perimeter: 36.374, top_width: 24.42 },
    ConspanTableEntry { y: 3.822, area: 102.494, perimeter: 37.278, top_width: 23.656 },
    ConspanTableEntry { y: 4.047, area: 107.728, perimeter: 38.184, top_width: 22.87 },
    ConspanTableEntry { y: 4.253, area: 112.356, perimeter: 39.089, top_width: 22.064 },
    ConspanTableEntry { y: 4.441, area: 116.427, perimeter: 39.995, top_width: 21.242 },
    ConspanTableEntry { y: 4.61, area: 119.945, perimeter: 40.90, top_width: 20.4 },
    ConspanTableEntry { y: 4.76, area: 122.941, perimeter: 41.805, top_width: 19.546 },
    ConspanTableEntry { y: 4.89, area: 125.426, perimeter: 42.71, top_width: 18.68 },
    ConspanTableEntry { y: 4.996, area: 127.36, perimeter: 43.605, top_width: 17.812 },
    ConspanTableEntry { y: 5.206, area: 130.895, perimeter: 45.602, top_width: 15.858 },
    ConspanTableEntry { y: 5.392, area: 133.662, perimeter: 47.597, top_width: 13.898 },
    ConspanTableEntry { y: 5.553, area: 135.741, perimeter: 49.593, top_width: 11.928 },
    ConspanTableEntry { y: 5.689, area: 137.229, perimeter: 51.588, top_width: 9.952 },
    ConspanTableEntry { y: 5.801, area: 138.233, perimeter: 53.584, top_width: 7.968 },
    ConspanTableEntry { y: 5.888, area: 138.839, perimeter: 55.578, top_width: 5.982 },
    ConspanTableEntry { y: 5.95, area: 139.149, perimeter: 57.574, top_width: 3.99 },
    ConspanTableEntry { y: 5.988, area: 139.262, perimeter: 59.569, top_width: 1.996 },
    ConspanTableEntry { y: 6.0, area: 139.274, perimeter: 61.565, top_width: 0.0 },
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
    
    let t = (y_norm - CONSPAN_28X6_TABLE[idx].y) / (CONSPAN_28X6_TABLE[idx + 1].y - CONSPAN_28X6_TABLE[idx].y);
    
    match field {
        "area" => {
            let val_norm = (1.0 - t) * CONSPAN_28X6_TABLE[idx].area + t * CONSPAN_28X6_TABLE[idx + 1].area;
            val_norm * (span / 28.0) * (rise / 6.0)
        }
        "perimeter" => {
            let val_norm = (1.0 - t) * CONSPAN_28X6_TABLE[idx].perimeter + t * CONSPAN_28X6_TABLE[idx + 1].perimeter;
            // scale the arch part and add the scaled bottom
            (val_norm - 28.0) * (rise / 6.0) + span
        }
        "top_width" => {
            if y >= rise {
                0.0
            } else {
                let val_norm = (1.0 - t) * CONSPAN_28X6_TABLE[idx].top_width + t * CONSPAN_28X6_TABLE[idx + 1].top_width;
                val_norm * (span / 28.0)
            }
        }
        _ => 0.0,
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
        CulvertShape::ConspanArch => {
            interpolate_conspan(span, rise, y_clamp, "area")
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
        CulvertShape::ConspanArch => {
            interpolate_conspan(span, rise, y, "top_width")
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
        CulvertShape::ConspanArch => {
            interpolate_conspan(span, rise, y_clamp, "perimeter")
        }
    }
}


/// Computes the effective flow area (A) in sq ft for a given depth (y) in ft inside a culvert barrel, accounting for blockage.
pub fn get_culvert_effective_area(shape: CulvertShape, span: f64, rise: f64, y: f64, depth_blocked: f64) -> f64 {
    let d_b = depth_blocked.min(rise);
    if y <= d_b {
        0.0
    } else {
        get_culvert_area(shape, span, rise, y) - get_culvert_area(shape, span, rise, d_b)
    }
}

/// Computes the effective wetted top width (T) in ft for a given depth (y) in ft inside a culvert barrel, accounting for blockage.
pub fn get_culvert_effective_top_width(shape: CulvertShape, span: f64, rise: f64, y: f64, depth_blocked: f64) -> f64 {
    let d_b = depth_blocked.min(rise);
    if y <= d_b {
        0.0
    } else {
        get_culvert_top_width(shape, span, rise, y)
    }
}

/// Computes the effective wetted perimeter (P) in ft for a given depth (y) in ft inside a culvert barrel, accounting for blockage.
pub fn get_culvert_effective_perimeter(shape: CulvertShape, span: f64, rise: f64, y: f64, depth_blocked: f64) -> f64 {
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
pub fn solve_barrel_critical_depth(shape: CulvertShape, span: f64, rise: f64, q: f64, depth_blocked: f64) -> f64 {
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}

fn default_num_barrels() -> i32 {
    1
}

/// HEC-RAS-style skew: projected inlet span × cos(θ), friction length ÷ cos(θ).
pub fn apply_barrel_skew(skew_deg: f64, span_ft: f64, len_ft: f64) -> (f64, f64) {
    let deg = skew_deg.clamp(0.0, 59.0);
    let cos_s = deg.to_radians().cos().max(0.52);
    (span_ft * cos_s, len_ft / cos_s)
}

fn normalize_culvert_params(params: &mut CulvertSolveParams) {
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

fn barrel_params_for_geometry(base: &CulvertSolveParams, span: f64, rise: f64) -> CulvertSolveParams {
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
    params.active_barrels > 1
        || params.barrel_spans.is_some()
        || params.barrel_rises.is_some()
}

fn estimate_max_barrel_q(params: &CulvertSolveParams) -> f64 {
    let shape = CulvertShape::from_i32(params.shape_type);
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
    let a_full = get_culvert_effective_area(shape, span_ft, rise_ft, rise_ft, db_ft);
    let q_cfs = a_full * (2.0 * G_ENGLISH * rise_ft.max(0.5)).sqrt();
    if params.units == UnitSystem::Metric {
        q_cfs * CFS_TO_CMS
    } else {
        q_cfs
    }
}

fn barrel_q_for_wsel(params: &CulvertSolveParams, target_wsel: f64) -> f64 {
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

fn default_weir_length_user(params: &CulvertSolveParams) -> f64 {
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
                let (span_eff, _) =
                    apply_barrel_skew(params.skew_deg, *span, params.length);
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
    let mut high = tw + if base.units == UnitSystem::Metric { 60.0 } else { 200.0 };

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let sum_q: f64 = barrel_bases
            .iter()
            .map(|p| barrel_q_for_wsel(p, mid))
            .sum();
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
    let mut max_inlet = wsel;
    let mut max_outlet = wsel;

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
            (CulvertShape::Arch | CulvertShape::ConspanArch, 20) => (0.0300, 1.5, 0.0500, 0.60),
            (CulvertShape::Arch | CulvertShape::ConspanArch, 21) => (0.0083, 2.0, 0.0374, 0.69),
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
        CulvertShape::Box => {
            if entrance_loss_coeff <= 0.2 {
                (0.026, 1.0, 0.0347, 0.81)
            } else {
                (0.061, 0.75, 0.0400, 0.80)
            }
        }
        CulvertShape::Arch | CulvertShape::ConspanArch => {
            if entrance_loss_coeff <= 0.2 {
                (0.0083, 2.0, 0.0374, 0.69)
            } else {
                (0.0300, 1.5, 0.0500, 0.60)
            }
        }
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

fn overtopping_only_result(wsel: f64, q_weir: f64) -> CulvertSolveResult {
    CulvertSolveResult {
        wsel,
        control_type: "overtopping".to_string(),
        wsel_inlet: 0.0,
        wsel_outlet: 0.0,
        q_barrel: 0.0,
        q_weir,
        barrel_depth: 0.0,
        barrel_velocity: 0.0,
        barrel_froude: 0.0,
    }
}

fn solve_culvert_barrel_internal(params: &CulvertSolveParams, q: f64) -> BarrelSolveInternal {
    let shape = CulvertShape::from_i32(params.shape_type);

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

    let d_eff = (rise_ft - db_ft).max(0.01);
    let a_full_eff = get_culvert_effective_area(shape, span_ft, rise_ft, rise_ft, db_ft);

    let ds_vel_ft = if params.units == UnitSystem::Metric {
        params.ds_velocity / FT_TO_M
    } else {
        params.ds_velocity
    };
    let us_vel_ft = if params.units == UnitSystem::Metric {
        params.us_velocity / FT_TO_M
    } else {
        params.us_velocity
    };
    // Apply velocity distribution coefficient alpha (~1.3 for contracted sections near culverts)
    let ds_vel_hd = (ds_vel_ft * ds_vel_ft) / (2.0 * G_ENGLISH) * 1.3;
    let us_vel_hd = (us_vel_ft * us_vel_ft) / (2.0 * G_ENGLISH) * 1.3;

    // 1. INLET CONTROL CALCULATIONS
    // Bisection search for critical depth inside barrel in feet (measured from original invert)
    let yc = solve_barrel_critical_depth(shape, span_ft, rise_ft, q_cfs, db_ft);
    let yc_eff = (yc - db_ft).max(0.0);
    let ac = get_culvert_effective_area(shape, span_ft, rise_ft, yc, db_ft);
    let vc = if ac > 1e-9 { q_cfs / ac } else { 0.0 };
    let hc_eff = yc_eff + (vc * vc) / (2.0 * G_ENGLISH); // Specific head at critical depth above effective invert

    let (k, m, c, y) = inlet_nomograph_coeffs(shape, params.inlet_type, params.entrance_loss_coeff);

    // FHWA barrel slope S0 (positive when downstream invert is lower than upstream).
    let culv_slope = (z_up_ft - z_down_ft) / len_ft;
    let f_param = q_cfs / (a_full_eff * d_eff.sqrt());

    // Unsubmerged Eq (Form 1)
    let hw_d_unsub = (hc_eff / d_eff) + k * f_param.powf(m) - 0.5 * culv_slope;
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

    let hw_inlet_eff = (hw_d * d_eff).max(hc_eff);
    let wsel_inlet = z_up_ft + db_ft + hw_inlet_eff;

    // 2. OUTLET CONTROL CALCULATIONS
    let y_barrel = (tw_ft - z_down_ft).max(yc).min(rise_ft);
    let a_barrel = get_culvert_effective_area(shape, span_ft, rise_ft, y_barrel, db_ft);
    let v_barrel = if a_barrel > 1e-9 { q_cfs / a_barrel } else { 0.0 };

    let he = params.entrance_loss_coeff * (v_barrel * v_barrel) / (2.0 * G_ENGLISH);
    let ho = params.exit_loss_coeff * ((v_barrel * v_barrel) / (2.0 * G_ENGLISH) - ds_vel_hd).max(0.0);

    // Friction loss (hf = L * Sf) using composite n and effective geometry
    let p_barrel = get_culvert_effective_perimeter(shape, span_ft, rise_ft, y_barrel, db_ft);
    let r_barrel = if p_barrel > 1e-9 { a_barrel / p_barrel } else { 0.0 };
    let n_c = get_culvert_composite_n(
        shape,
        span_ft,
        rise_ft,
        y_barrel,
        db_ft,
        params.roughness_n,
        params.manning_n_bottom,
        dbn_ft,
    );
    let sf = if a_barrel > 1e-9 && r_barrel > 1e-9 {
        (q_cfs * n_c / (1.486 * a_barrel * r_barrel.powf(2.0 / 3.0))).powi(2)
    } else {
        0.0
    };
    let hf = len_ft * sf;

    // Total head loss / energy equation
    let eg_outlet = tw_ft + ds_vel_hd + he + hf + ho;
    let wsel_outlet = eg_outlet - us_vel_hd;

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

    let t_barrel = get_culvert_effective_top_width(shape, span_ft, rise_ft, y_barrel, db_ft);
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

fn weir_flow_us(cw: f64, length_ft: f64, wsel_ft: f64, crest_ft: f64) -> f64 {
    let head = (wsel_ft - crest_ft).max(0.0);
    if head < 1e-9 || length_ft < 1e-9 {
        return 0.0;
    }
    cw * length_ft * head.powf(1.5)
}

fn solve_wsel_weir_only(
    params: &CulvertSolveParams,
    cw_us: f64,
    length_ft: f64,
    crest_ft: f64,
    q_target: f64,
) -> f64 {
    let q_target_cfs = if params.units == UnitSystem::Metric {
        q_target / CFS_TO_CMS
    } else {
        q_target
    };
    let mut low = crest_ft;
    let mut high = crest_ft + 50.0;
    let mut best = high;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let q_mid = weir_flow_us(cw_us, length_ft, mid, crest_ft);
        if q_mid < q_target_cfs {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    if params.units == UnitSystem::Metric {
        best * FT_TO_M
    } else {
        best
    }
}

/// Solve culvert headwater including optional roadway overtopping weir.
pub fn solve_culvert(params: &CulvertSolveParams) -> CulvertSolveResult {
    let mut params = params.clone();
    normalize_culvert_params(&mut params);
    let q_total = params.q;

    let crest_user = match params.crest_elev {
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
    let (crest_ft, cw_us, length_ft) = if params.units == UnitSystem::Metric {
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

    let mut q_barrel_total = q_total;
    let mut last_barrel = solve_culvert_barrels(&params, q_barrel_total);
    let mut last_control = barrel_control_type(&last_barrel);

    for _ in 0..25 {
        let wsel_ft = if params.units == UnitSystem::Metric {
            last_barrel.wsel / FT_TO_M
        } else {
            last_barrel.wsel
        };

        if wsel_ft <= crest_ft + 1e-6 {
            return assemble_culvert_result(&params, &last_barrel, q_barrel_total, 0.0, last_control);
        }

        let q_weir_cfs = weir_flow_us(cw_us, length_ft, wsel_ft, crest_ft);
        let q_weir = if params.units == UnitSystem::Metric {
            q_weir_cfs * CFS_TO_CMS
        } else {
            q_weir_cfs
        };

        if q_weir >= q_total - 1e-6 {
            let wsel_overtopping =
                solve_wsel_weir_only(&params, cw_us, length_ft, crest_ft, q_total);
            return overtopping_only_result(wsel_overtopping, q_total);
        }

        let q_barrel_new = (q_total - q_weir).max(0.0);
        if (q_barrel_new - q_barrel_total).abs() < 1e-4 {
            let control = if q_weir > 0.01 * q_total {
                "overtopping".to_string()
            } else {
                last_control.clone()
            };
            return assemble_culvert_result(&params, &last_barrel, q_barrel_total, q_weir, control);
        }

        q_barrel_total = q_barrel_new;
        last_barrel = solve_culvert_barrels(&params, q_barrel_total);
        last_control = barrel_control_type(&last_barrel);
    }

    let wsel_ft = if params.units == UnitSystem::Metric {
        last_barrel.wsel / FT_TO_M
    } else {
        last_barrel.wsel
    };
    let q_weir_cfs = weir_flow_us(cw_us, length_ft, wsel_ft, crest_ft);
    let q_weir = if params.units == UnitSystem::Metric {
        q_weir_cfs * CFS_TO_CMS
    } else {
        q_weir_cfs
    };
    let q_thresh = if params.units == UnitSystem::Metric {
        0.01 * q_total
    } else {
        0.01 * q_total
    };
    let control = if wsel_ft > crest_ft + 1e-6 && q_weir > q_thresh {
        "overtopping".to_string()
    } else {
        last_control
    };

    assemble_culvert_result(&params, &last_barrel, q_barrel_total, q_weir, control)
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
    };
    solve_culvert(&params).wsel
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
            0.012, // manning_n_bottom
            0.0,   // depth_bottom_n
            0.0,   // depth_blocked
            0.0,   // ds_velocity
            0.0,   // us_velocity
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
            0.012, // manning_n_bottom
            0.0,   // depth_bottom_n
            0.0,   // depth_blocked
            0.0,   // ds_velocity
            0.0,   // us_velocity
        );

        // Under outlet control, WSEL_up should be TW + losses = 15.0 + 0.726 = 15.726 ft.
        let hw_depth_high = wsel_up_high - 10.0;
        assert!((hw_depth_high - 5.73).abs() < 0.05, "expected ~5.73, got {}", hw_depth_high);
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
        assert!(span30 < 10.0);
        assert!(len30 > 100.0);
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
        };
        let mut skewed = base.clone();
        skewed.skew_deg = 30.0;
        let hw_plain = solve_culvert(&base).wsel;
        let hw_skew = solve_culvert(&skewed).wsel;
        assert_eq!(solve_culvert(&base).control_type, "outlet");
        assert!(hw_skew > hw_plain, "skew={} plain={}", hw_skew, hw_plain);
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
        };
        let equal_small = CulvertSolveParams {
            barrel_spans: None,
            barrel_rises: None,
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
        };
        let explicit = CulvertSolveParams {
            barrel_spans: Some(vec![5.0, 5.0]),
            barrel_rises: Some(vec![5.0, 5.0]),
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
        };
        let result = solve_culvert(&params);
        assert!(result.wsel > 1.2);
        assert_eq!(result.control_type, "overtopping");
    }
}
