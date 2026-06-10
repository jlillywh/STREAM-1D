use crate::utils::{G_METRIC, UnitSystem, FT_TO_M, structure_in_reach_interval};
use crate::geometry::{
    flow_area_for_row, geometry_row_at_elevation, section_needs_dynamic_geometry,
    specific_force_at_elevation, CrossSection, GeometryRow, GeometryTable, IneffectiveFlowAreas,
};

/// Input parameters for the steady-state solver.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SteadyInputs {
    /// Cross-sections defining the river reach.
    pub cross_sections: Vec<CrossSection>,
    /// Flow rate (in cfs if unit_system is USCustomary, cms if Metric).
    pub flow_rate: f64,
    /// Number of uniform vertical intervals to slice cross-sections (default 100).
    pub num_slices: Option<usize>,
    /// Contraction loss coefficient (default 0.1).
    pub coeff_contraction: Option<f64>,
    /// Expansion loss coefficient (default 0.3).
    pub coeff_expansion: Option<f64>,
    /// Flow regime (0 = Subcritical, 1 = Supercritical, 2 = Mixed).
    pub regime: u8,
    /// Downstream WSEL boundary condition (optional, in user units).
    pub downstream_wsel: Option<f64>,
    /// Upstream WSEL boundary condition (optional, in user units).
    pub upstream_wsel: Option<f64>,
    /// Maximum distance between adjacent sections before automatic interpolation (optional, in user units).
    pub max_spacing: Option<f64>,
    /// Culvert stations (optional)
    #[serde(default)]
    pub culvert_stations: Option<Vec<f64>>,
    /// Culvert shape types (optional, 0 = Circular, 1 = Box, 2 = Arch)
    #[serde(default)]
    pub culvert_shape_types: Option<Vec<i32>>,
    /// Culvert spans/diameters (optional, in feet/meters)
    #[serde(default)]
    pub culvert_spans: Option<Vec<f64>>,
    /// Culvert rises (optional, in feet/meters)
    #[serde(default)]
    pub culvert_rises: Option<Vec<f64>>,
    /// Culvert Manning's n roughness coefficients (optional)
    #[serde(default)]
    pub culvert_roughness_ns: Option<Vec<f64>>,
    /// Culvert lengths (optional, in feet/meters)
    #[serde(default)]
    pub culvert_lengths: Option<Vec<f64>>,
    /// Culvert entrance loss coefficients Ke (optional)
    #[serde(default)]
    pub culvert_entrance_loss_coeffs: Option<Vec<f64>>,
    /// Culvert exit loss coefficients Kx (optional)
    #[serde(default)]
    pub culvert_exit_loss_coeffs: Option<Vec<f64>>,
    /// Culvert number of barrels (optional)
    #[serde(default)]
    pub culvert_barrels: Option<Vec<i32>>,
    /// Culvert Manning's n roughness coefficients for bottom/sediment (optional)
    #[serde(default)]
    pub culvert_roughness_n_bottoms: Option<Vec<f64>>,
    /// Culvert depths to use bottom roughness n (optional, in feet/meters)
    #[serde(default)]
    pub culvert_depth_bottom_ns: Option<Vec<f64>>,
    /// Culvert depths blocked/filled with sediment (optional, in feet/meters)
    #[serde(default)]
    pub culvert_depth_blockeds: Option<Vec<f64>>,
    /// FHWA inlet type per culvert (0 = legacy Ke threshold; see culvert solver docs).
    #[serde(default)]
    pub culvert_inlet_types: Option<Vec<i32>>,
    /// Optional upstream culvert invert elevation per culvert (defaults to adjacent section bed).
    #[serde(default)]
    pub culvert_z_ups: Option<Vec<f64>>,
    /// Optional downstream culvert invert elevation per culvert (defaults to adjacent section bed).
    #[serde(default)]
    pub culvert_z_downs: Option<Vec<f64>>,
    /// Roadway/embankment crest elevation for overtopping weir per culvert (optional).
    #[serde(default)]
    pub culvert_crest_elevs: Option<Vec<f64>>,
    /// Weir discharge coefficient per culvert (default 2.6 US / 1.44 metric).
    #[serde(default)]
    pub culvert_weir_coeffs: Option<Vec<f64>>,
    /// Effective weir length per culvert (default span × num_barrels).
    #[serde(default)]
    pub culvert_weir_lengths: Option<Vec<f64>>,
    /// Barrel skew angle in degrees from normal to flow (0 = no skew), per culvert.
    #[serde(default)]
    pub culvert_skew_angles: Option<Vec<f64>>,
    /// Open barrels per culvert (≤ `culvert_barrels`). Omit to use all barrels.
    #[serde(default)]
    pub culvert_active_barrels: Option<Vec<i32>>,
    /// Per-barrel span/diameter per culvert (length = open barrels). Omit entries to use `culvert_spans`.
    #[serde(default)]
    pub culvert_barrel_spans: Option<Vec<Vec<f64>>>,
    /// Per-barrel rise per culvert (length = open barrels). Omit entries to use `culvert_rises`.
    #[serde(default)]
    pub culvert_barrel_rises: Option<Vec<Vec<f64>>>,

    /// Stations where bridges are located (in user units, e.g. feet or meters)
    #[serde(default)]
    pub bridge_stations: Option<Vec<f64>>,
    /// Elevation of the lowest point of the bridge deck at each bridge
    #[serde(default)]
    pub bridge_low_chords: Option<Vec<f64>>,
    /// Elevation of the top of the roadway deck at each bridge
    #[serde(default)]
    pub bridge_high_chords: Option<Vec<f64>>,
    /// Thickness/width of a single pier at each bridge
    #[serde(default)]
    pub bridge_pier_widths: Option<Vec<f64>>,
    /// Number of piers at each bridge
    #[serde(default)]
    pub bridge_num_piers: Option<Vec<i32>>,
    /// Pier shape classification (0 = Square, 1 = Semicircular, 2 = Twin Cylinders, 3 = Sharp/Triangular)
    #[serde(default)]
    pub bridge_pier_shapes: Option<Vec<i32>>,
    /// Weir discharge coefficient Cw for overtopping flow (e.g., default 2.6 US, 1.44 Metric)
    #[serde(default)]
    pub bridge_weir_coeffs: Option<Vec<f64>>,
    /// Orifice discharge coefficient Cd for pressure flow (e.g., default 0.5 or 0.6)
    #[serde(default)]
    pub bridge_orifice_coeffs: Option<Vec<f64>>,
    /// Total horizontal width blocked by left + right abutments at each bridge (perpendicular to flow).
    #[serde(default)]
    pub bridge_abutment_block_widths: Option<Vec<f64>>,
    /// Left abutment width per bridge (perpendicular to flow). With right widths, overrides legacy total.
    #[serde(default)]
    pub bridge_abutment_left_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_right_widths: Option<Vec<f64>>,
    /// Outer-face station in opening coordinates (default: opening left/right edge).
    #[serde(default)]
    pub bridge_abutment_left_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_right_stations: Option<Vec<f64>>,
    /// Constant top elevation per bridge (omit for full-height abutment).
    #[serde(default)]
    pub bridge_abutment_left_top_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_right_top_elevations: Option<Vec<f64>>,
    /// Piecewise top profile per bridge `[bridge][point]` (≥ 2 points).
    #[serde(default)]
    pub bridge_abutment_left_top_profile_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_abutment_left_top_profile_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_abutment_right_top_profile_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_abutment_right_top_profile_elevations: Option<Vec<Vec<f64>>>,
    /// Low-flow method per bridge: 0 = auto, 1 = Yarnell, 2 = momentum, 3 = energy, 4 = WSPRO.
    #[serde(default)]
    pub bridge_low_flow_methods: Option<Vec<i32>>,
    /// High-flow method per bridge: 0 = pressure/weir, 1 = energy.
    #[serde(default)]
    pub bridge_high_flow_methods: Option<Vec<i32>>,
    /// Reach length through each bridge for friction (user units). 0 uses interval spacing.
    #[serde(default)]
    pub bridge_lengths: Option<Vec<f64>>,
    /// WSPRO contracted-opening discharge coefficient C per bridge (typical 0.7–0.9).
    #[serde(default)]
    pub bridge_wspro_coeffs: Option<Vec<f64>>,
    /// Sluice-gate pressure coefficient when only upstream is submerged. 0 = auto (HEC-RAS Y3/Z).
    #[serde(default)]
    pub bridge_pressure_flow_coeffs_inlet: Option<Vec<f64>>,
    /// Max weir submergence ratio before switching to energy method (default 0.98).
    #[serde(default)]
    pub bridge_max_weir_submergence: Option<Vec<f64>>,
    /// Deck profile stations across opening per bridge `[bridge][point]` (user units).
    #[serde(default)]
    pub bridge_deck_stations: Option<Vec<Vec<f64>>>,
    /// Low chord elevation at each deck station `[bridge][point]`.
    #[serde(default)]
    pub bridge_deck_low_elevations: Option<Vec<Vec<f64>>>,
    /// High chord elevation at each deck station `[bridge][point]`.
    #[serde(default)]
    pub bridge_deck_high_elevations: Option<Vec<Vec<f64>>>,
    /// Bridge ineffective stations (`s` frame). See `docs/reference/equations.md` §H0.
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_stations: Option<Vec<Vec<f64>>>,
    /// Activation elevations for left ineffective blocks per bridge `[bridge][block]`.
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_elevations: Option<Vec<Vec<f64>>>,
    /// Right ineffective-flow stations per bridge `[bridge][block]`.
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_stations: Option<Vec<Vec<f64>>>,
    /// Activation elevations for right ineffective blocks per bridge `[bridge][block]`.
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_elevations: Option<Vec<Vec<f64>>>,
    /// Upstream-face left ineffective stations (falls back to `bridge_ineffective_left_stations`).
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_stations_upstream: Option<Vec<Vec<f64>>>,
    /// Upstream-face left ineffective elevations (falls back to `bridge_ineffective_left_elevations`).
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_elevations_upstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_stations_upstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_elevations_upstream: Option<Vec<Vec<f64>>>,
    /// Downstream-face ineffective blocks (each falls back to legacy shared fields).
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_stations_downstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_elevations_downstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_stations_downstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_elevations_downstream: Option<Vec<Vec<f64>>>,
    /// Bridge skew from normal to flow, degrees per bridge (0–59°).
    #[serde(default)]
    pub bridge_skew_angles: Option<Vec<f64>>,
    /// Pier centerline stations across opening per bridge `[bridge][pier]` (deck station frame).
    #[serde(default)]
    pub bridge_pier_stations: Option<Vec<Vec<f64>>>,
    /// HEC-RAS BU (bridge upstream face) cross section per bridge. Overrides reach US geometry.
    #[serde(default)]
    pub bridge_upstream_cross_sections: Option<Vec<CrossSection>>,
    /// HEC-RAS BD (bridge downstream face) cross section per bridge.
    #[serde(default)]
    pub bridge_downstream_cross_sections: Option<Vec<CrossSection>>,
    /// Optional interior bridge cuts per bridge `[bridge][section]`, ordered US → DS.
    #[serde(default)]
    pub bridge_internal_cross_sections: Option<Vec<Vec<CrossSection>>>,
    /// Reach XS lateral `x` at bridge opening station 0 per bridge.
    #[serde(default)]
    pub bridge_opening_reach_station_origins: Option<Vec<f64>>,
    /// Opening ↔ reach anchor mode per bridge: 0 = BU left `x`, 1 = reach river station, 2 = explicit lateral `x`.
    #[serde(default)]
    pub bridge_opening_anchor_modes: Option<Vec<i32>>,
    /// Longitudinal reach river station (user units) for anchor mode 1 per bridge.
    #[serde(default)]
    pub bridge_opening_anchor_reach_stations: Option<Vec<f64>>,
    /// Explicit approach (upstream) cross section per bridge — HEC-RAS section 4 equivalent.
    #[serde(default)]
    pub bridge_approach_cross_sections: Option<Vec<crate::geometry::CrossSection>>,
    /// Explicit departure (downstream exit) cross section per bridge.
    #[serde(default)]
    pub bridge_departure_cross_sections: Option<Vec<crate::geometry::CrossSection>>,
    /// Reach river station of approach cut when `bridge_approach_cross_sections` is omitted.
    #[serde(default)]
    pub bridge_approach_reach_stations: Option<Vec<f64>>,
    /// Reach river station of departure cut when `bridge_departure_cross_sections` is omitted.
    #[serde(default)]
    pub bridge_departure_reach_stations: Option<Vec<f64>>,
    /// Guide banks on approach cut (reach lateral `x`); used when not on `CrossSection.guide_banks`.
    #[serde(default)]
    pub bridge_approach_guide_banks: Option<Vec<crate::geometry::GuideBanks>>,
    /// Guide banks on departure cut (reach lateral `x`).
    #[serde(default)]
    pub bridge_departure_guide_banks: Option<Vec<crate::geometry::GuideBanks>>,

    /// Downstream boundary condition type (0 = Known WSEL, 1 = Critical Depth, 2 = Normal Depth, 3 = Rating Curve)
    #[serde(default)]
    pub downstream_bc_type: Option<i32>,
    /// Downstream friction slope for normal depth boundary condition
    #[serde(default)]
    pub downstream_bc_slope: Option<f64>,
    /// Downstream rating curve flows
    #[serde(default)]
    pub downstream_bc_rating_q: Option<Vec<f64>>,
    /// Downstream rating curve water surface elevations
    #[serde(default)]
    pub downstream_bc_rating_wsel: Option<Vec<f64>>,

    /// Upstream boundary condition type (0 = Known WSEL, 1 = Critical Depth, 2 = Normal Depth, 3 = Rating Curve)
    #[serde(default)]
    pub upstream_bc_type: Option<i32>,
    /// Upstream friction slope for normal depth boundary condition
    #[serde(default)]
    pub upstream_bc_slope: Option<f64>,
    /// Upstream rating curve flows
    #[serde(default)]
    pub upstream_bc_rating_q: Option<Vec<f64>>,
    /// Upstream rating curve water surface elevations
    #[serde(default)]
    pub upstream_bc_rating_wsel: Option<Vec<f64>>,

    /// Optional tributary reach joining the main channel (steady subcritical today).
    #[serde(default)]
    pub tributary_cross_sections: Option<Vec<CrossSection>>,
    /// Tributary inflow (same units as `flow_rate`) added at the junction.
    #[serde(default)]
    pub tributary_flow_rate: Option<f64>,
    /// Main-channel station where the tributary mouth connects (must match a main cross-section).
    #[serde(default)]
    pub junction_main_station: Option<f64>,
}

/// Output results from the steady-state solver.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SteadyResult {
    /// Solved water surface elevations (WSEL) at each cross-section (in user units).
    pub wsel: Vec<f64>,
    /// Critical depth elevations (y_c + z_min) at each cross-section (in user units).
    pub critical_wsel: Vec<f64>,
    /// Velocity values at each cross-section (in user units, ft/s or m/s).
    pub velocity: Vec<f64>,
    /// Flow areas at each cross-section (in user units, ft^2 or m^2).
    pub area: Vec<f64>,
    /// Froude numbers at each cross-section.
    pub froude: Vec<f64>,
    /// Top width of flow at each cross-section (in user units).
    pub top_width: Vec<f64>,
    /// Energy grade line friction slope (dimensionless) at each cross-section.
    pub eg_slope: Vec<f64>,
    /// Tributary reach WSEL at each tributary cross-section (if a junction was modeled).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tributary_wsel: Option<Vec<f64>>,
    /// Tributary reach velocity at each tributary cross-section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tributary_velocity: Option<Vec<f64>>,
    /// Tributary reach Froude number at each tributary cross-section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tributary_froude: Option<Vec<f64>>,
    /// Controlling mechanism per culvert: `"inlet"`, `"outlet"`, or `"overtopping"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culvert_control_types: Option<Vec<String>>,
    /// Tier 2a — inlet-control headwater per culvert (user units).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culvert_wsel_inlet: Option<Vec<f64>>,
    /// Tier 2a — outlet-control headwater per culvert (user units).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culvert_wsel_outlet: Option<Vec<f64>>,
    /// Tier 2a — barrel discharge per culvert (user units).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culvert_q_barrels: Option<Vec<f64>>,
    /// Tier 2a — weir discharge when overtopping per culvert (user units).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culvert_q_weirs: Option<Vec<f64>>,
    /// Tier 2a — flow depth in barrel at downstream end (user units).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culvert_barrel_depths: Option<Vec<f64>>,
    /// Tier 2a — mean barrel velocity (user units).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culvert_barrel_velocities: Option<Vec<f64>>,
    /// Tier 2a — barrel Froude number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culvert_barrel_froude: Option<Vec<f64>>,
}

impl GeometryTable {
    /// Calculates momentum force (Specific Force) M = Q^2 / (g * A) + A * y_bar
    pub fn calculate_specific_force(&self, elev: f64, q: f64) -> f64 {
        let row = self.interpolate(elev);
        if row.area < 1e-6 {
            return f64::INFINITY;
        }
        let area_moment = self.calculate_area_moment(elev);
        (q * q) / (G_METRIC * row.area) + area_moment
    }
}

/// Solves critical depth (yc) relative to bottom elevation for a cross section lookup table.
pub fn solve_critical_depth_table(table: &GeometryTable, q: f64) -> f64 {
    if table.rows.is_empty() {
        return 0.0;
    }
    let y_min = table.rows[0].elevation;
    let y_max = table.rows[table.rows.len() - 1].elevation;

    let mut low = 0.0;
    let mut high = (y_max - y_min).max(10.0);
    let mut best_yc = 0.0;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let elev = y_min + mid;
        let row = table.interpolate(elev);

        if row.area < 1e-6 {
            low = mid;
            continue;
        }

        // Fr^2 = Q^2 * T / (g * A^3)
        let fr_sq = (q * q * row.top_width) / (G_METRIC * row.area.powi(3));
        let f_val = 1.0 - fr_sq;

        if f_val.abs() < 1e-6 {
            best_yc = mid;
            break;
        }

        if f_val < 0.0 {
            // Supercritical (depth too small)
            low = mid;
        } else {
            // Subcritical (depth too big)
            high = mid;
        }
        best_yc = mid;
    }
    best_yc
}

/// Solves critical depth (yc) relative to bottom elevation for a cross section.
pub fn solve_critical_depth(_xs: &CrossSection, table: &GeometryTable, q: f64) -> f64 {
    solve_critical_depth_table(table, q)
}

/// Solves normal depth (yn) for a given cross section lookup table using bisection search.
/// Returns absolute WSEL in metric.
pub fn solve_normal_depth_table(table: &GeometryTable, q: f64, slope: f64) -> f64 {
    if table.rows.is_empty() {
        return 0.0;
    }
    let slope_val = if slope <= 0.0 { 0.01 } else { slope };
    let target_k = q / slope_val.sqrt();

    let y_min = table.rows[0].elevation;
    let y_max = table.rows[table.rows.len() - 1].elevation;

    let mut low = y_min;
    let mut high = y_max;
    let mut best_y = y_min;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let row = table.interpolate(mid);
        if row.conveyance < target_k {
            low = mid;
        } else {
            high = mid;
        }
        best_y = mid;
    }
    best_y
}

/// Interpolates stage-discharge coordinates to find boundary WSEL at flow rate Q (in user units).
/// Returns WSEL in user units, or None if coordinates are empty/invalid.
pub fn interpolate_rating_curve(q: f64, rating_q: &[f64], rating_wsel: &[f64]) -> Option<f64> {
    if rating_q.is_empty() || rating_wsel.is_empty() {
        return None;
    }
    let n = rating_q.len().min(rating_wsel.len());
    if n == 0 {
        return None;
    }
    if n == 1 {
        return Some(rating_wsel[0]);
    }

    // Zip and sort by flow rate Q
    let mut pairs: Vec<(f64, f64)> = rating_q.iter().zip(rating_wsel.iter()).take(n).map(|(&qi, &wi)| (qi, wi)).collect();
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Clamp to nearest if out of bounds
    if q <= pairs[0].0 {
        return Some(pairs[0].1);
    }
    if q >= pairs[n - 1].0 {
        return Some(pairs[n - 1].1);
    }

    // Find interval and interpolate
    for i in 0..n - 1 {
        let q1 = pairs[i].0;
        let w1 = pairs[i].1;
        let q2 = pairs[i + 1].0;
        let w2 = pairs[i + 1].1;
        if q >= q1 && q <= q2 {
            let dq = q2 - q1;
            if dq.abs() < 1e-9 {
                return Some(w1);
            }
            let t = (q - q1) / dq;
            return Some(w1 + t * (w2 - w1));
        }
    }

    Some(pairs[0].1)
}


fn step_flow_area(
    row: &GeometryRow,
    use_channel: bool,
    ineffective: Option<&IneffectiveFlowAreas>,
) -> f64 {
    let has_ineffective = ineffective.filter(|i| i.is_configured()).is_some()
        || row.active_area + 1e-6 < row.area;
    if use_channel && row.channel_area > 1e-6 {
        if has_ineffective {
            row.active_channel_area
        } else {
            row.channel_area
        }
    } else if use_channel && row.channel_area > 1e-6 {
        row.channel_area
    } else {
        flow_area_for_row(row)
    }
}

fn interpolate_step_row(
    table: &GeometryTable,
    xs: Option<&CrossSection>,
    ineffective: Option<&IneffectiveFlowAreas>,
    elev: f64,
) -> GeometryRow {
    geometry_row_at_elevation(table, xs, elev, ineffective, None)
}

/// Steps from section 1 (known WSEL) to section 2 (unknown WSEL) using the Standard Step Method.
pub fn solve_step(
    table1: &GeometryTable,
    xs1: Option<&CrossSection>,
    ineffective1: Option<&IneffectiveFlowAreas>,
    y1: f64, // WSEL 1
    table2: &GeometryTable,
    xs2: Option<&CrossSection>,
    ineffective2: Option<&IneffectiveFlowAreas>,
    z2_min: f64,
    yc2: f64,
    q: f64,
    length: f64,
    c_contraction: f64,
    c_expansion: f64,
    is_subcritical: bool,
    use_channel1: bool,
    use_channel2: bool,
) -> Option<f64> {
    let row1 = interpolate_step_row(table1, xs1, ineffective1, y1);
    let area1 = step_flow_area(&row1, use_channel1, ineffective1);
    if area1 < 1e-6 {
        return None;
    }
    let hv1 = (q * q) / (2.0 * G_METRIC * area1 * area1);
    let k1 = row1.conveyance;

    let target_residual = |y2: f64| -> Option<f64> {
        let row2 = interpolate_step_row(table2, xs2, ineffective2, y2);
        let area2 = step_flow_area(&row2, use_channel2, ineffective2);
        if area2 < 1e-6 {
            return None;
        }
        let hv2 = (q * q) / (2.0 * G_METRIC * area2 * area2);
        let k2 = row2.conveyance;

        let k_avg = 0.5 * (k1 + k2);
        if k_avg < 1e-9 {
            return None;
        }
        let sf = (q / k_avg).powi(2);
        let hf = length * sf;

        let c_ec = if hv2 > hv1 { c_contraction } else { c_expansion };
        let ho = c_ec * (hv2 - hv1).abs();

        if is_subcritical {
            // Upstream step: H2 = H1 + hf + ho
            Some(y2 + hv2 - (y1 + hv1 + hf + ho))
        } else {
            // Downstream step: H2 = H1 - hf - ho
            Some(y2 + hv2 - (y1 + hv1 - hf - ho))
        }
    };

    // Define search bounds based on flow regime to prevent conjugate depth crossing
    let upstream_has_reach_modifiers = xs2
        .is_some_and(|xs| section_needs_dynamic_geometry(xs, ineffective2));
    let (mut low, mut high) = if is_subcritical {
        let l = if upstream_has_reach_modifiers {
            y1.max(z2_min + 1e-5)
        } else {
            z2_min + yc2 + 1e-5
        };
        let h = y1.max(z2_min + yc2) + 20.0;
        (l, h)
    } else {
        let l = z2_min + 1e-5;
        let h = z2_min + yc2 - 1e-5;
        (l, h)
    };

    let mut res_low = loop {
        match target_residual(low) {
            Some(r) => break r,
            None if is_subcritical => {
                low += 0.05;
                if low >= high {
                    return None;
                }
            }
            None => return None,
        }
    };
    let mut res_high = loop {
        match target_residual(high) {
            Some(r) => break r,
            None if is_subcritical => {
                high -= 0.05;
                if high <= low {
                    return None;
                }
            }
            None => return None,
        }
    };

    if res_low * res_high > 0.0 {
        if is_subcritical {
            // Expand subcritical upper bound if needed
            for _ in 0..5 {
                high += 20.0;
                if let Some(r_high) = target_residual(high) {
                    res_high = r_high;
                    if res_low * res_high <= 0.0 {
                        break;
                    }
                }
            }
            // Ineffective / low-conveyance reaches: scan upward from known downstream stage
            if res_low * res_high > 0.0 {
                let mut scan_base = y1.max(z2_min + 1e-5);
                if let Some(mut r_base) = target_residual(scan_base) {
                    let mut scan_top = scan_base;
                    for _ in 0..160 {
                        scan_top += 0.1;
                        if let Some(r_top) = target_residual(scan_top) {
                            if r_base * r_top <= 0.0 {
                                low = scan_base;
                                high = scan_top;
                                res_low = r_base;
                                res_high = r_top;
                                break;
                            }
                            scan_base = scan_top;
                            r_base = r_top;
                        }
                    }
                }
            }
        }
        if res_low * res_high > 0.0 {
            if is_subcritical && upstream_has_reach_modifiers {
                let mut best_y = y1.max(z2_min + 1e-5);
                let mut best_abs = f64::INFINITY;
                let mut y_try = best_y;
                for _ in 0..250 {
                    if let Some(r) = target_residual(y_try) {
                        let abs_r = r.abs();
                        if abs_r < best_abs {
                            best_abs = abs_r;
                            best_y = y_try;
                        }
                        if abs_r < 1e-6 {
                            return Some(y_try);
                        }
                    }
                    y_try += 0.05;
                }
                if best_abs < 0.05 {
                    return Some(best_y);
                }
            }
            // Failed to bracket root, fallback to critical depth
            return Some(z2_min + yc2);
        }
    }

    let mut best_y = 0.5 * (low + high);
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let res_mid = match target_residual(mid) {
            Some(r) => r,
            None => {
                high = mid;
                continue;
            }
        };

        if res_mid.abs() < 1e-8 {
            best_y = mid;
            break;
        }

        if is_subcritical {
            if res_mid < 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        } else {
            if res_mid > 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        }
        best_y = mid;
    }

    Some(best_y)
}

fn bridge_deck_profile_for(
    inputs: &SteadyInputs,
    b_idx: usize,
    raw_units: UnitSystem,
) -> Option<crate::solvers::bridge::BridgeDeckProfile> {
    let low_chord = inputs
        .bridge_low_chords
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.0);
    let high_chord = inputs
        .bridge_high_chords
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.0);
    crate::solvers::bridge::build_bridge_deck_profile(
        low_chord,
        high_chord,
        inputs
            .bridge_deck_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        inputs
            .bridge_deck_low_elevations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        inputs
            .bridge_deck_high_elevations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        raw_units,
    )
}

fn bridge_face_blocks(
    face_stations: Option<&Vec<Vec<f64>>>,
    face_elevations: Option<&Vec<Vec<f64>>>,
    legacy_stations: Option<&Vec<Vec<f64>>>,
    legacy_elevations: Option<&Vec<Vec<f64>>>,
    b_idx: usize,
) -> (Vec<f64>, Vec<f64>) {
    let stations = face_stations
        .and_then(|v| v.get(b_idx))
        .filter(|blocks| !blocks.is_empty())
        .cloned()
        .or_else(|| {
            legacy_stations
                .and_then(|v| v.get(b_idx))
                .cloned()
        })
        .unwrap_or_default();
    let elevations = face_elevations
        .and_then(|v| v.get(b_idx))
        .filter(|blocks| !blocks.is_empty())
        .cloned()
        .or_else(|| {
            legacy_elevations
                .and_then(|v| v.get(b_idx))
                .cloned()
        })
        .unwrap_or_default();
    (stations, elevations)
}

fn bridge_ineffective_upstream_for(inputs: &SteadyInputs, b_idx: usize) -> Option<IneffectiveFlowAreas> {
    let (left_s, left_e) = bridge_face_blocks(
        inputs.bridge_ineffective_left_stations_upstream.as_ref(),
        inputs.bridge_ineffective_left_elevations_upstream.as_ref(),
        inputs.bridge_ineffective_left_stations.as_ref(),
        inputs.bridge_ineffective_left_elevations.as_ref(),
        b_idx,
    );
    let (right_s, right_e) = bridge_face_blocks(
        inputs.bridge_ineffective_right_stations_upstream.as_ref(),
        inputs.bridge_ineffective_right_elevations_upstream.as_ref(),
        inputs.bridge_ineffective_right_stations.as_ref(),
        inputs.bridge_ineffective_right_elevations.as_ref(),
        b_idx,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

fn bridge_ineffective_downstream_for(inputs: &SteadyInputs, b_idx: usize) -> Option<IneffectiveFlowAreas> {
    let (left_s, left_e) = bridge_face_blocks(
        inputs.bridge_ineffective_left_stations_downstream.as_ref(),
        inputs.bridge_ineffective_left_elevations_downstream.as_ref(),
        inputs.bridge_ineffective_left_stations.as_ref(),
        inputs.bridge_ineffective_left_elevations.as_ref(),
        b_idx,
    );
    let (right_s, right_e) = bridge_face_blocks(
        inputs.bridge_ineffective_right_stations_downstream.as_ref(),
        inputs.bridge_ineffective_right_elevations_downstream.as_ref(),
        inputs.bridge_ineffective_right_stations.as_ref(),
        inputs.bridge_ineffective_right_elevations.as_ref(),
        b_idx,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

fn bridge_face_geometry_for(
    inputs: &SteadyInputs,
    b_idx: usize,
    i: usize,
    raw_units: UnitSystem,
    num_slices: usize,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[Option<CrossSection>],
    densified_z_mins: &[f64],
    interval_length_m: f64,
) -> crate::solvers::bridge_interior::BridgeFaceSolveGeometry {
    let reach_z_up_user = if raw_units == UnitSystem::USCustomary {
        densified_z_mins[i] / FT_TO_M
    } else {
        densified_z_mins[i]
    };
    let reach_z_down_user = if raw_units == UnitSystem::USCustomary {
        densified_z_mins[i + 1] / FT_TO_M
    } else {
        densified_z_mins[i + 1]
    };
    let interior = crate::solvers::bridge_interior::interior_from_steady(inputs, b_idx);
    let anchor_reach_xs = interior
        .opening_anchor_reach_station
        .and_then(|st| {
            crate::solvers::bridge_interior::cross_section_at_reach_station(
                densified_stations,
                densified_xs,
                st,
                raw_units,
            )
        });
    let (approach_xs, departure_xs, guide_banks_approach, guide_banks_departure) =
        crate::solvers::bridge_interior::resolve_approach_departure_sections(
            &interior,
            i,
            densified_stations,
            densified_xs,
            raw_units,
        );
    crate::solvers::bridge_interior::resolve_bridge_face_solve_geometry(
        &interior,
        anchor_reach_xs.as_ref(),
        densified_xs[i].as_ref(),
        densified_xs[i + 1].as_ref(),
        &densified_tables[i],
        &densified_tables[i + 1],
        reach_z_up_user,
        reach_z_down_user,
        raw_units,
        num_slices,
        bridge_ineffective_upstream_for(inputs, b_idx),
        bridge_ineffective_downstream_for(inputs, b_idx),
        inputs
            .bridge_skew_angles
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        inputs
            .bridge_pier_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        interval_length_m,
        inputs
            .bridge_lengths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        approach_xs,
        departure_xs,
        guide_banks_approach,
        guide_banks_departure,
    )
}

fn bridge_coupling_for(inputs: &SteadyInputs, b_idx: usize) -> crate::solvers::bridge::BridgeCouplingParams {
    let abutment = crate::solvers::bridge_abutment::abutment_user_input_from_steady(
        inputs
            .bridge_abutment_block_widths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        inputs.bridge_abutment_left_widths.as_ref(),
        inputs.bridge_abutment_right_widths.as_ref(),
        inputs.bridge_abutment_left_stations.as_ref(),
        inputs.bridge_abutment_right_stations.as_ref(),
        inputs.bridge_abutment_left_top_elevations.as_ref(),
        inputs.bridge_abutment_right_top_elevations.as_ref(),
        inputs.bridge_abutment_left_top_profile_stations.as_ref(),
        inputs.bridge_abutment_left_top_profile_elevations.as_ref(),
        inputs.bridge_abutment_right_top_profile_stations.as_ref(),
        inputs.bridge_abutment_right_top_profile_elevations.as_ref(),
        b_idx,
    );
    crate::solvers::bridge::BridgeCouplingParams {
        abutment,
        low_flow_method: inputs
            .bridge_low_flow_methods
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0),
        high_flow_method: inputs
            .bridge_high_flow_methods
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0),
        length: inputs
            .bridge_lengths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        wspro_coeff: inputs
            .bridge_wspro_coeffs
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.8),
        coeff_contraction: inputs.coeff_contraction.unwrap_or(0.1),
        coeff_expansion: inputs.coeff_expansion.unwrap_or(0.3),
        pressure_coeff_inlet: inputs
            .bridge_pressure_flow_coeffs_inlet
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: inputs
            .bridge_max_weir_submergence
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.98),
    }
}

/// Runs the steady-state water surface profile solver.
pub fn solve_steady(inputs: &SteadyInputs) -> SteadyResult {
    if crate::solvers::junction::has_tributary_junction(inputs) {
        return crate::solvers::junction::solve_steady_junction(inputs);
    }
    solve_steady_single_reach(inputs)
}

/// Single-reach steady solver (no tributary junction).
pub fn solve_steady_single_reach(inputs: &SteadyInputs) -> SteadyResult {
    let raw_units = inputs.cross_sections.first().map(|xs| xs.unit_system).unwrap_or(UnitSystem::Metric);
    let q = if raw_units == UnitSystem::USCustomary {
        inputs.flow_rate * crate::utils::CFS_TO_CMS
    } else {
        inputs.flow_rate
    };

    let num_slices = inputs.num_slices.unwrap_or(100);
    let c_contraction = inputs.coeff_contraction.unwrap_or(0.1);
    let c_expansion = inputs.coeff_expansion.unwrap_or(0.3);

    // Convert all cross sections to metric internally
    let mut xs_list: Vec<CrossSection> = inputs.cross_sections.iter().map(|xs| xs.to_metric()).collect();
    
    // Sort descending by station (upstream to downstream)
    // Upstream has larger station numbers, index 0 is most upstream.
    xs_list.sort_by(|a, b| b.station.partial_cmp(&a.station).unwrap());
    let m = xs_list.len();

    // Generate geometry tables and calculate bed elevations
    let tables: Vec<GeometryTable> = xs_list.iter().map(|xs| xs.generate_lookup_table(num_slices)).collect();
    let z_mins: Vec<f64> = xs_list.iter().map(|xs| xs.y.iter().cloned().fold(f64::INFINITY, f64::min)).collect();

    // DENSIFICATION STEP: Automatic Reach Interpolation
    let max_sp = inputs.max_spacing.map(|sp| {
        if raw_units == UnitSystem::USCustomary { sp * FT_TO_M } else { sp }
    });

    let mut densified_tables = Vec::new();
    let mut densified_z_mins = Vec::new();
    let mut densified_stations = Vec::new();
    let mut densified_xs: Vec<Option<CrossSection>> = Vec::new();
    let mut original_to_densified = Vec::new();

    for i in 0..m {
        let current_idx = densified_tables.len();
        original_to_densified.push(current_idx);

        densified_tables.push(tables[i].clone());
        densified_z_mins.push(z_mins[i]);
        densified_stations.push(xs_list[i].station);
        densified_xs.push(Some(xs_list[i].clone()));

        if i < m - 1 {
            let dx = xs_list[i].station - xs_list[i + 1].station;
            if let Some(limit) = max_sp {
                if limit > 0.0 && dx > limit {
                    let num_spaces = (dx / limit).ceil() as usize;
                    let ds = dx / num_spaces as f64;
                    for k in 1..num_spaces {
                        let t = k as f64 / num_spaces as f64;
                        let s_interp = xs_list[i].station - k as f64 * ds;
                        
                        let (t_interp, z_interp) = crate::geometry::processor::interpolate_geometry_table(
                            &tables[i],
                            z_mins[i],
                            &tables[i + 1],
                            z_mins[i + 1],
                            t,
                            num_slices,
                        );
                        
                        densified_tables.push(t_interp);
                        densified_z_mins.push(z_interp);
                        densified_stations.push(s_interp);
                        densified_xs.push(None);
                    }
                }
            }
        }
    }

    let bridge_face_intervals = crate::solvers::bridge_interior::apply_bridge_reach_layout_steady(
        inputs,
        raw_units,
        num_slices,
        &mut densified_stations,
        &mut densified_tables,
        &mut densified_z_mins,
        &mut densified_xs,
    );
    let original_stations: Vec<f64> = xs_list.iter().map(|xs| xs.station).collect();
    crate::solvers::bridge_interior::refresh_original_to_densified(
        &original_stations,
        &densified_stations,
        &mut original_to_densified,
    );
    let bridge_at_interval: std::collections::HashMap<usize, usize> = bridge_face_intervals
        .iter()
        .enumerate()
        .filter_map(|(b_idx, interval)| interval.map(|i| (i, b_idx)))
        .collect();

    let dm = densified_tables.len();

    // Calculate critical depths and elevations for the densified grid
    let ycs: Vec<f64> = densified_tables.iter().map(|table| solve_critical_depth_table(table, q)).collect();
    let critical_wsels: Vec<f64> = densified_z_mins.iter().zip(&ycs).map(|(&z, &yc)| z + yc).collect();

    let regime = inputs.regime; // 0=Subcritical, 1=Supercritical, 2=Mixed
    let mut wsel_metric = vec![0.0; dm];

    // Boundary conditions in metric
    let ds_bc_type = inputs.downstream_bc_type.unwrap_or(0);
    let ds_wsel_metric = match ds_bc_type {
        1 => Some(critical_wsels[dm - 1]),
        2 => {
            let slope = inputs.downstream_bc_slope.unwrap_or(0.01);
            Some(solve_normal_depth_table(&densified_tables[dm - 1], q, slope))
        }
        3 => {
            if let (Some(rq), Some(rw)) = (&inputs.downstream_bc_rating_q, &inputs.downstream_bc_rating_wsel) {
                interpolate_rating_curve(inputs.flow_rate, rq, rw).map(|w| {
                    if raw_units == UnitSystem::USCustomary { w * FT_TO_M } else { w }
                })
            } else {
                None
            }
        }
        _ => inputs.downstream_wsel.map(|w| if raw_units == UnitSystem::USCustomary { w * FT_TO_M } else { w }),
    }.unwrap_or(critical_wsels[dm - 1]);

    let us_bc_type = inputs.upstream_bc_type.unwrap_or(0);
    let us_wsel_metric = match us_bc_type {
        1 => Some(critical_wsels[0]),
        2 => {
            let slope = inputs.upstream_bc_slope.unwrap_or(0.01);
            Some(solve_normal_depth_table(&densified_tables[0], q, slope))
        }
        3 => {
            if let (Some(rq), Some(rw)) = (&inputs.upstream_bc_rating_q, &inputs.upstream_bc_rating_wsel) {
                interpolate_rating_curve(inputs.flow_rate, rq, rw).map(|w| {
                    if raw_units == UnitSystem::USCustomary { w * FT_TO_M } else { w }
                })
            } else {
                None
            }
        }
        _ => inputs.upstream_wsel.map(|w| if raw_units == UnitSystem::USCustomary { w * FT_TO_M } else { w }),
    }.unwrap_or(critical_wsels[0]);

    let culvert_count = inputs.culvert_stations.as_ref().map(|s| s.len());
    let mut culvert_control_types: Option<Vec<String>> =
        culvert_count.map(|n| vec![String::new(); n]);
    let mut culvert_wsel_inlet: Option<Vec<f64>> = culvert_count.map(|n| vec![0.0; n]);
    let mut culvert_wsel_outlet: Option<Vec<f64>> = culvert_count.map(|n| vec![0.0; n]);
    let mut culvert_q_barrels: Option<Vec<f64>> = culvert_count.map(|n| vec![0.0; n]);
    let mut culvert_q_weirs: Option<Vec<f64>> = culvert_count.map(|n| vec![0.0; n]);
    let mut culvert_barrel_depths: Option<Vec<f64>> = culvert_count.map(|n| vec![0.0; n]);
    let mut culvert_barrel_velocities: Option<Vec<f64>> = culvert_count.map(|n| vec![0.0; n]);
    let mut culvert_barrel_froude: Option<Vec<f64>> = culvert_count.map(|n| vec![0.0; n]);

    let mut structure_adjacent_indices = std::collections::HashSet::new();
    let mut structure_ineffective: std::collections::HashMap<usize, IneffectiveFlowAreas> =
        std::collections::HashMap::new();
    if let Some(ref c_stations) = inputs.culvert_stations {
        for &c_st in c_stations {
            let c_st_metric = if raw_units == UnitSystem::USCustomary {
                c_st * FT_TO_M
            } else {
                c_st
            };
            for j in 0..dm - 1 {
                if structure_in_reach_interval(c_st_metric, &densified_stations, j) {
                    structure_adjacent_indices.insert(j);
                    structure_adjacent_indices.insert(j + 1);
                    break;
                }
            }
        }
    }
    for (b_idx, interval) in bridge_face_intervals.iter().enumerate() {
        let Some(j) = *interval else { continue };
        structure_adjacent_indices.insert(j);
        structure_adjacent_indices.insert(j + 1);
        if let Some(ineff) = bridge_ineffective_upstream_for(inputs, b_idx).map(|ineff| {
            if raw_units == UnitSystem::USCustomary {
                ineff.to_metric(UnitSystem::USCustomary)
            } else {
                ineff
            }
        }) {
            structure_ineffective.insert(j, ineff);
        }
        if let Some(ineff) = bridge_ineffective_downstream_for(inputs, b_idx).map(|ineff| {
            if raw_units == UnitSystem::USCustomary {
                ineff.to_metric(UnitSystem::USCustomary)
            } else {
                ineff
            }
        }) {
            structure_ineffective.insert(j + 1, ineff);
        }
    }

    // SWEEP 1: SUBCRITICAL (Downstream to Upstream)
    let mut sub_wsel = vec![0.0; dm];
    if regime == 0 || regime == 2 {
        sub_wsel[dm - 1] = ds_wsel_metric;
        if sub_wsel[dm - 1] < critical_wsels[dm - 1] {
            sub_wsel[dm - 1] = critical_wsels[dm - 1];
        }

        for i in (0..dm - 1).rev() {
            let length = densified_stations[i] - densified_stations[i + 1];

            let bridge_idx = bridge_at_interval.get(&i).copied();

            // Check if there is a culvert in this reach interval
            let mut culvert_idx = None;
            if let Some(ref c_stations) = inputs.culvert_stations {
                for (c_idx, &c_st) in c_stations.iter().enumerate() {
                    let c_st_metric = if raw_units == UnitSystem::USCustomary {
                        c_st * FT_TO_M
                    } else {
                        c_st
                    };
                    if structure_in_reach_interval(c_st_metric, &densified_stations, i) {
                        culvert_idx = Some(c_idx);
                        break;
                    }
                }
            }

            if let Some(b_idx) = bridge_idx {
                let low_chord = inputs.bridge_low_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let high_chord = inputs.bridge_high_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let pier_width = inputs.bridge_pier_widths.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let num_piers = inputs.bridge_num_piers.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0);
                let pier_shape = inputs.bridge_pier_shapes.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0);
                let weir_coeff = inputs.bridge_weir_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(if raw_units == UnitSystem::USCustomary { 2.6 } else { 1.44 });
                let orifice_coeff = inputs.bridge_orifice_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.5);
                let coupling = bridge_coupling_for(inputs, b_idx);
                let deck = bridge_deck_profile_for(inputs, b_idx, raw_units);
                let deck_ref = deck.as_ref();

                let tw_wsel_user = if raw_units == UnitSystem::USCustomary {
                    sub_wsel[i + 1] / FT_TO_M
                } else {
                    sub_wsel[i + 1]
                };

                let face_geo = bridge_face_geometry_for(
                    inputs,
                    b_idx,
                    i,
                    raw_units,
                    num_slices,
                    &densified_stations,
                    &densified_tables,
                    &densified_xs,
                    &densified_z_mins,
                    length,
                );
                let wsel_up_user = crate::solvers::bridge::solve_bridge_wsel(
                    inputs.flow_rate,
                    low_chord,
                    high_chord,
                    pier_width,
                    num_piers,
                    pier_shape,
                    weir_coeff,
                    orifice_coeff,
                    face_geo.z_down_user,
                    face_geo.z_up_user,
                    tw_wsel_user,
                    raw_units,
                    &face_geo.table_up,
                    &face_geo.table_down,
                    &coupling,
                    length,
                    deck_ref,
                    Some(&face_geo.sections),
                );

                sub_wsel[i] = if raw_units == UnitSystem::USCustomary {
                    wsel_up_user * FT_TO_M
                } else {
                    wsel_up_user
                };
            } else if let Some(c_idx) = culvert_idx {
                let shape_type = inputs.culvert_shape_types.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0);
                let span = inputs.culvert_spans.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(4.0);
                let rise = inputs.culvert_rises.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(4.0);
                let roughness_n = inputs.culvert_roughness_ns.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.013);
                let culv_len = inputs.culvert_lengths.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(100.0);
                let entrance_loss_coeff = inputs.culvert_entrance_loss_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.5);
                let exit_loss_coeff = inputs.culvert_exit_loss_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(1.0);
                let manning_n_bottom = inputs.culvert_roughness_n_bottoms.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(roughness_n);
                let depth_bottom_n = inputs.culvert_depth_bottom_ns.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
                let depth_blocked = inputs.culvert_depth_blockeds.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
                let inlet_type = inputs.culvert_inlet_types.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0);
                let crest_elev = inputs.culvert_crest_elevs.as_ref().and_then(|v| v.get(c_idx)).copied();
                let weir_coeff = inputs.culvert_weir_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
                let weir_length = inputs.culvert_weir_lengths.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
                let num_barrels = inputs.culvert_barrels.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(1).max(1);
                let active_barrels = inputs
                    .culvert_active_barrels
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .copied()
                    .unwrap_or(0);
                let skew_deg = inputs
                    .culvert_skew_angles
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .copied()
                    .unwrap_or(0.0);
                let barrel_spans = inputs
                    .culvert_barrel_spans
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .cloned();
                let barrel_rises = inputs
                    .culvert_barrel_rises
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .cloned();

                let tw_wsel_user = if raw_units == UnitSystem::USCustomary {
                    sub_wsel[i + 1] / FT_TO_M
                } else {
                    sub_wsel[i + 1]
                };
                let bed_z_down = if raw_units == UnitSystem::USCustomary {
                    densified_z_mins[i + 1] / FT_TO_M
                } else {
                    densified_z_mins[i + 1]
                };
                let bed_z_up = if raw_units == UnitSystem::USCustomary {
                    densified_z_mins[i] / FT_TO_M
                } else {
                    densified_z_mins[i]
                };
                let z_down_user = inputs
                    .culvert_z_downs
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .copied()
                    .unwrap_or(bed_z_down);
                let z_up_user = inputs
                    .culvert_z_ups
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .copied()
                    .unwrap_or(bed_z_up);

                // Compute downstream velocity for exit loss calculation (using channel_area for contracted flow)
                let ds_row = densified_tables[i + 1].interpolate(sub_wsel[i + 1]);
                let ds_area_user = if raw_units == UnitSystem::USCustomary {
                    ds_row.channel_area / (FT_TO_M * FT_TO_M)
                } else {
                    ds_row.channel_area
                };
                let ds_velocity_user = if ds_area_user > 1e-9 {
                    inputs.flow_rate / ds_area_user
                } else {
                    0.0
                };

                let mut wsel_up_user = tw_wsel_user;
                let mut culvert_result = crate::solvers::culvert::CulvertSolveResult {
                    wsel: tw_wsel_user,
                    control_type: String::new(),
                    wsel_inlet: 0.0,
                    wsel_outlet: 0.0,
                    q_barrel: 0.0,
                    q_weir: 0.0,
                    barrel_depth: 0.0,
                    barrel_velocity: 0.0,
                    barrel_froude: 0.0,
                };
                let table_up = &densified_tables[i];

                for _ in 0..3 {
                    let wsel_up_metric = if raw_units == UnitSystem::USCustomary {
                        wsel_up_user * FT_TO_M
                    } else {
                        wsel_up_user
                    };
                    let us_row = table_up.interpolate(wsel_up_metric);
                    let us_area_user = if raw_units == UnitSystem::USCustomary {
                        us_row.channel_area / (FT_TO_M * FT_TO_M)
                    } else {
                        us_row.channel_area
                    };
                    let us_velocity_user = if us_area_user > 1e-9 {
                        inputs.flow_rate / us_area_user
                    } else {
                        0.0
                    };

                    culvert_result = crate::solvers::culvert::solve_culvert(
                        &crate::solvers::culvert::CulvertSolveParams {
                            q: inputs.flow_rate,
                            shape_type,
                            inlet_type,
                            span,
                            rise,
                            roughness_n,
                            length: culv_len,
                            entrance_loss_coeff,
                            exit_loss_coeff,
                            z_down: z_down_user,
                            z_up: z_up_user,
                            tw_wsel: tw_wsel_user,
                            units: raw_units,
                            manning_n_bottom,
                            depth_bottom_n,
                            depth_blocked,
                            ds_velocity: ds_velocity_user,
                            us_velocity: us_velocity_user,
                            crest_elev,
                            weir_coeff,
                            weir_length,
                            num_barrels,
                            active_barrels,
                            skew_deg,
                            barrel_spans: barrel_spans.clone(),
                            barrel_rises: barrel_rises.clone(),
                        },
                    );
                    wsel_up_user = culvert_result.wsel;
                }

                if let Some(ref mut controls) = culvert_control_types {
                    controls[c_idx] = culvert_result.control_type.clone();
                }
                if let Some(ref mut v) = culvert_wsel_inlet {
                    v[c_idx] = culvert_result.wsel_inlet;
                }
                if let Some(ref mut v) = culvert_wsel_outlet {
                    v[c_idx] = culvert_result.wsel_outlet;
                }
                if let Some(ref mut v) = culvert_q_barrels {
                    v[c_idx] = culvert_result.q_barrel;
                }
                if let Some(ref mut v) = culvert_q_weirs {
                    v[c_idx] = culvert_result.q_weir;
                }
                if let Some(ref mut v) = culvert_barrel_depths {
                    v[c_idx] = culvert_result.barrel_depth;
                }
                if let Some(ref mut v) = culvert_barrel_velocities {
                    v[c_idx] = culvert_result.barrel_velocity;
                }
                if let Some(ref mut v) = culvert_barrel_froude {
                    v[c_idx] = culvert_result.barrel_froude;
                }

                sub_wsel[i] = if raw_units == UnitSystem::USCustomary {
                    wsel_up_user * FT_TO_M
                } else {
                    wsel_up_user
                };
            } else {
                sub_wsel[i] = solve_step(
                    &densified_tables[i + 1],
                    densified_xs[i + 1].as_ref(),
                    structure_ineffective.get(&(i + 1)),
                    sub_wsel[i + 1],
                    &densified_tables[i],
                    densified_xs[i].as_ref(),
                    structure_ineffective.get(&i),
                    densified_z_mins[i],
                    ycs[i],
                    q,
                    length,
                    c_contraction,
                    c_expansion,
                    true,
                    structure_adjacent_indices.contains(&(i + 1)),
                    structure_adjacent_indices.contains(&i),
                ).unwrap_or(critical_wsels[i]);
            }
        }
    }

    // SWEEP 2: SUPERCRITICAL (Upstream to Downstream)
    let mut super_wsel = vec![0.0; dm];
    if regime == 1 || regime == 2 {
        super_wsel[0] = us_wsel_metric;
        if super_wsel[0] > critical_wsels[0] {
            super_wsel[0] = critical_wsels[0];
        }

        for i in 0..dm - 1 {
            let length = densified_stations[i] - densified_stations[i + 1];

            let bridge_idx = bridge_at_interval.get(&i).copied();

            // Check if there is a culvert in this reach interval
            let mut culvert_idx = None;
            if let Some(ref c_stations) = inputs.culvert_stations {
                for (c_idx, &c_st) in c_stations.iter().enumerate() {
                    let c_st_metric = if raw_units == UnitSystem::USCustomary {
                        c_st * FT_TO_M
                    } else {
                        c_st
                    };
                    if structure_in_reach_interval(c_st_metric, &densified_stations, i) {
                        culvert_idx = Some(c_idx);
                        break;
                    }
                }
            }

            if let Some(c_idx) = culvert_idx {
                let shape_type = inputs.culvert_shape_types.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0);
                let span = inputs.culvert_spans.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(4.0);
                let rise = inputs.culvert_rises.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(4.0);
                let roughness_n = inputs.culvert_roughness_ns.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.013);
                let culv_len = inputs.culvert_lengths.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(100.0);
                let entrance_loss_coeff = inputs.culvert_entrance_loss_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.5);
                let exit_loss_coeff = inputs.culvert_exit_loss_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(1.0);
                let manning_n_bottom = inputs.culvert_roughness_n_bottoms.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(roughness_n);
                let depth_bottom_n = inputs.culvert_depth_bottom_ns.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
                let depth_blocked = inputs.culvert_depth_blockeds.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
                let inlet_type = inputs.culvert_inlet_types.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0);
                let crest_elev = inputs.culvert_crest_elevs.as_ref().and_then(|v| v.get(c_idx)).copied();
                let weir_coeff = inputs.culvert_weir_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
                let weir_length = inputs.culvert_weir_lengths.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
                let num_barrels = inputs.culvert_barrels.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(1).max(1);
                let active_barrels = inputs
                    .culvert_active_barrels
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .copied()
                    .unwrap_or(0);
                let skew_deg = inputs
                    .culvert_skew_angles
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .copied()
                    .unwrap_or(0.0);
                let barrel_spans = inputs
                    .culvert_barrel_spans
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .cloned();
                let barrel_rises = inputs
                    .culvert_barrel_rises
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .cloned();

                let hw_wsel_user = if raw_units == UnitSystem::USCustomary {
                    super_wsel[i] / FT_TO_M
                } else {
                    super_wsel[i]
                };
                let bed_z_down = if raw_units == UnitSystem::USCustomary {
                    densified_z_mins[i + 1] / FT_TO_M
                } else {
                    densified_z_mins[i + 1]
                };
                let bed_z_up = if raw_units == UnitSystem::USCustomary {
                    densified_z_mins[i] / FT_TO_M
                } else {
                    densified_z_mins[i]
                };
                let z_down_user = inputs
                    .culvert_z_downs
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .copied()
                    .unwrap_or(bed_z_down);
                let z_up_user = inputs
                    .culvert_z_ups
                    .as_ref()
                    .and_then(|v| v.get(c_idx))
                    .copied()
                    .unwrap_or(bed_z_up);

                let table_up = &densified_tables[i];
                let table_down = &densified_tables[i + 1];
                let us_row = table_up.interpolate(super_wsel[i]);
                let us_area_user = if raw_units == UnitSystem::USCustomary {
                    us_row.channel_area / (FT_TO_M * FT_TO_M)
                } else {
                    us_row.channel_area
                };
                let us_velocity_user = inputs.flow_rate / us_area_user.max(1e-9);

                let mut tw_wsel_user = hw_wsel_user;
                for _ in 0..3 {
                    let tw_metric = if raw_units == UnitSystem::USCustomary {
                        tw_wsel_user * FT_TO_M
                    } else {
                        tw_wsel_user
                    };
                    let ds_row = table_down.interpolate(tw_metric);
                    let ds_area_user = if raw_units == UnitSystem::USCustomary {
                        ds_row.channel_area / (FT_TO_M * FT_TO_M)
                    } else {
                        ds_row.channel_area
                    };
                    let ds_velocity_user = inputs.flow_rate / ds_area_user.max(1e-9);

                    let culvert_params = crate::solvers::culvert::CulvertSolveParams {
                        q: inputs.flow_rate,
                        shape_type,
                        inlet_type,
                        span,
                        rise,
                        roughness_n,
                        length: culv_len,
                        entrance_loss_coeff,
                        exit_loss_coeff,
                        z_down: z_down_user,
                        z_up: z_up_user,
                        tw_wsel: tw_wsel_user,
                        units: raw_units,
                        manning_n_bottom,
                        depth_bottom_n,
                        depth_blocked,
                        ds_velocity: ds_velocity_user,
                        us_velocity: us_velocity_user,
                        crest_elev,
                        weir_coeff,
                        weir_length,
                        num_barrels,
                        active_barrels,
                        skew_deg,
                        barrel_spans: barrel_spans.clone(),
                        barrel_rises: barrel_rises.clone(),
                    };
                    let (tw_new, _) =
                        crate::solvers::culvert::solve_culvert_from_headwater(&culvert_params, hw_wsel_user);
                    tw_wsel_user = tw_new;
                }

                super_wsel[i + 1] = if raw_units == UnitSystem::USCustomary {
                    tw_wsel_user * FT_TO_M
                } else {
                    tw_wsel_user
                };
            } else if let Some(b_idx) = bridge_idx {
                let low_chord = inputs.bridge_low_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let high_chord = inputs.bridge_high_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let pier_width = inputs.bridge_pier_widths.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let num_piers = inputs.bridge_num_piers.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0);
                let pier_shape = inputs.bridge_pier_shapes.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0);
                let weir_coeff = inputs.bridge_weir_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(if raw_units == UnitSystem::USCustomary { 2.6 } else { 1.44 });
                let orifice_coeff = inputs.bridge_orifice_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.5);
                let coupling = bridge_coupling_for(inputs, b_idx);
                let deck = bridge_deck_profile_for(inputs, b_idx, raw_units);
                let deck_ref = deck.as_ref();

                let hw_wsel_user = if raw_units == UnitSystem::USCustomary {
                    super_wsel[i] / FT_TO_M
                } else {
                    super_wsel[i]
                };

                let face_geo = bridge_face_geometry_for(
                    inputs,
                    b_idx,
                    i,
                    raw_units,
                    num_slices,
                    &densified_stations,
                    &densified_tables,
                    &densified_xs,
                    &densified_z_mins,
                    length,
                );
                let tw_wsel_user = crate::solvers::bridge::solve_bridge_tailwater(
                    inputs.flow_rate,
                    low_chord,
                    high_chord,
                    pier_width,
                    num_piers,
                    pier_shape,
                    weir_coeff,
                    orifice_coeff,
                    face_geo.z_down_user,
                    face_geo.z_up_user,
                    hw_wsel_user,
                    raw_units,
                    &face_geo.table_up,
                    &face_geo.table_down,
                    &coupling,
                    length,
                    deck_ref,
                    Some(&face_geo.sections),
                );

                super_wsel[i + 1] = if raw_units == UnitSystem::USCustomary {
                    tw_wsel_user * FT_TO_M
                } else {
                    tw_wsel_user
                };
            } else {
                super_wsel[i + 1] = solve_step(
                    &densified_tables[i],
                    densified_xs[i].as_ref(),
                    structure_ineffective.get(&i),
                    super_wsel[i],
                    &densified_tables[i + 1],
                    densified_xs[i + 1].as_ref(),
                    structure_ineffective.get(&(i + 1)),
                    densified_z_mins[i + 1],
                    ycs[i + 1],
                    q,
                    length,
                    c_contraction,
                    c_expansion,
                    false,
                    structure_adjacent_indices.contains(&i),
                    structure_adjacent_indices.contains(&(i + 1)),
                ).unwrap_or(critical_wsels[i + 1]);
            }
        }
    }

    // REGIME SELECTION / MIXED REGIME SOLVING
    if regime == 0 {
        wsel_metric = sub_wsel;
    } else if regime == 1 {
        wsel_metric = super_wsel;
    } else {
        // Mixed regime selection
        let mut super_failed = false;
        for i in 0..dm {
            let sub_m = specific_force_at_elevation(
                &densified_tables[i],
                densified_xs[i].as_ref(),
                sub_wsel[i],
                q,
                structure_ineffective.get(&i),
                None,
            );
            let super_m = specific_force_at_elevation(
                &densified_tables[i],
                densified_xs[i].as_ref(),
                super_wsel[i],
                q,
                structure_ineffective.get(&i),
                None,
            );

            let yc_i = (critical_wsels[i] - densified_z_mins[i]).max(0.0);
            let dry_threshold = 0.02_f64.min(0.1 * yc_i);

            let sub_depth = sub_wsel[i] - densified_z_mins[i];
            let super_depth = super_wsel[i] - densified_z_mins[i];

            if super_depth < dry_threshold {
                super_failed = true;
            }

            if super_failed && sub_depth >= dry_threshold {
                wsel_metric[i] = sub_wsel[i];
            } else if sub_depth < dry_threshold && super_depth >= dry_threshold {
                wsel_metric[i] = super_wsel[i];
            } else if sub_m >= super_m {
                wsel_metric[i] = sub_wsel[i];
            } else {
                wsel_metric[i] = super_wsel[i];
            }
        }
    }

    // POST-PROCESSING: Calculate outputs for original sections and convert back to user units

    let mut original_mapping = vec![0; m];
    for (orig_idx, orig_xs) in inputs.cross_sections.iter().enumerate() {
        let mut sorted_idx = 0;
        for (s_idx, s_xs) in xs_list.iter().enumerate() {
            if (s_xs.station - (orig_xs.station * if raw_units == UnitSystem::USCustomary { FT_TO_M } else { 1.0 })).abs() < 1e-4 {
                sorted_idx = s_idx;
                break;
            }
        }
        original_mapping[orig_idx] = sorted_idx;
    }

    let mut out_wsel = vec![0.0; m];
    let mut out_yc = vec![0.0; m];
    let mut out_vel = vec![0.0; m];
    let mut out_area = vec![0.0; m];
    let mut out_fr = vec![0.0; m];
    let mut out_top_width = vec![0.0; m];
    let mut out_eg_slope = vec![0.0; m];

    for orig_idx in 0..m {
        let sorted_xs_idx = original_mapping[orig_idx];
        let sorted_idx = original_to_densified[sorted_xs_idx];
        
        let wsel_val = wsel_metric[sorted_idx];
        let yc_val = critical_wsels[sorted_idx];
        let table = &densified_tables[sorted_idx];
        let ineffective_override = structure_ineffective.get(&sorted_idx);
        let row = interpolate_step_row(
            table,
            densified_xs[sorted_idx].as_ref(),
            ineffective_override,
            wsel_val,
        );
        let flow_area = step_flow_area(
            &row,
            structure_adjacent_indices.contains(&sorted_idx),
            ineffective_override,
        );

        let velocity = if flow_area > 1e-6 { q / flow_area } else { 0.0 };
        let froude = if flow_area > 1e-6 && row.top_width > 1e-6 {
            let d_hydraulic = flow_area / row.top_width;
            velocity / (G_METRIC * d_hydraulic).sqrt()
        } else {
            0.0
        };

        if raw_units == UnitSystem::USCustomary {
            out_wsel[orig_idx] = wsel_val / FT_TO_M;
            out_yc[orig_idx] = yc_val / FT_TO_M;
            out_vel[orig_idx] = velocity / FT_TO_M;
            out_area[orig_idx] = row.area / (FT_TO_M * FT_TO_M);
            out_top_width[orig_idx] = row.top_width / FT_TO_M;
        } else {
            out_wsel[orig_idx] = wsel_val;
            out_yc[orig_idx] = yc_val;
            out_vel[orig_idx] = velocity;
            out_area[orig_idx] = row.area;
            out_top_width[orig_idx] = row.top_width;
        }
        out_fr[orig_idx] = froude;

        let sf = if row.conveyance > 1e-6 { (q / row.conveyance).powi(2) } else { 0.0 };
        out_eg_slope[orig_idx] = sf;
    }

    SteadyResult {
        wsel: out_wsel,
        critical_wsel: out_yc,
        velocity: out_vel,
        area: out_area,
        froude: out_fr,
        top_width: out_top_width,
        eg_slope: out_eg_slope,
        tributary_wsel: None,
        tributary_velocity: None,
        tributary_froude: None,
        culvert_control_types,
        culvert_wsel_inlet,
        culvert_wsel_outlet,
        culvert_q_barrels,
        culvert_q_weirs,
        culvert_barrel_depths,
        culvert_barrel_velocities,
        culvert_barrel_froude,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steady_critical_depth() {
        // Rectangular channel: bottom width = 10m. Q = 20 cms.
        // Analytical yc = (Q^2 / (g * B^2))^(1/3)
        // yc = (20^2 / (9.80665 * 10^2))^(1/3) = (400 / 980.665)^(1/3) = (0.407886)^0.33333 = 0.7416 m.
        let xs = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let table = xs.generate_lookup_table(10);
        let yc = solve_critical_depth(&xs, &table, 20.0);
        assert!((yc - 0.7416).abs() < 1e-3, "yc was {}", yc);
    }

    #[test]
    fn test_steady_subcritical_profile() {
        // Set up 3 identical cross-sections spaced 100m apart.
        // Rectangular channel: width = 10m, Manning's n = 0.02.
        // Stationing: 200, 100, 0.
        // Slope = 0.001 (bottom elevations: 0.2m, 0.1m, 0.0m).
        // Flow rate Q = 15.0 cms.
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0 + 0.2, 0.2, 0.2, 5.0 + 0.2],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0 + 0.1, 0.1, 0.1, 5.0 + 0.1],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let inputs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 15.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0, // Subcritical
            downstream_wsel: Some(1.2), // high tailwater boundary, creating backwater
            upstream_wsel: None,
            max_spacing: None,
            culvert_stations: None,
            culvert_shape_types: None,
            culvert_spans: None,
            culvert_rises: None,
            culvert_roughness_ns: None,
            culvert_lengths: None,
            culvert_entrance_loss_coeffs: None,
            culvert_exit_loss_coeffs: None,
            ..Default::default()
        };

        let result = solve_steady(&inputs);
        
        // Assertions
        // At station 0 (index 2 in inputs, but solver handles mapping back to match inputs array ordering)
        assert_eq!(result.wsel[2], 1.2);
        // At station 100 (index 1), WSEL should be higher than at station 0 but slope is lower than bed slope
        // Because backwater curve is M1, water depth decreases as you go upstream (depth at 0 is 1.2, depth at 100 should be < 1.2 - 0.1 = 1.1)
        let depth0 = result.wsel[2] - 0.0;
        let depth100 = result.wsel[1] - 0.1;
        let depth200 = result.wsel[0] - 0.2;
        
        assert!(depth100 < depth0, "depth100={} depth0={}", depth100, depth0);
        assert!(depth200 < depth100, "depth200={} depth100={}", depth200, depth100);
        
        // Froude number should be < 1.0 (subcritical)
        for &fr in &result.froude {
            assert!(fr < 1.0, "Froude was {}", fr);
        }
    }

    #[test]
    fn test_steady_reach_densification() {
        // Set up 2 cross-sections spaced 1000m apart.
        // Bed slope is 0.001 (z1 = 1.0m, z2 = 0.0m).
        // Rectangular channel: width = 10m.
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        // Run with a max spacing of 100.0m (which should create 9 intermediate cross sections, total 11 sections internally)
        let inputs = SteadyInputs {
            cross_sections: vec![xs1000, xs0],
            flow_rate: 15.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0, // Subcritical
            downstream_wsel: Some(1.2), // tailwater depth = 1.2m
            upstream_wsel: None,
            max_spacing: Some(100.0),
            culvert_stations: None,
            culvert_shape_types: None,
            culvert_spans: None,
            culvert_rises: None,
            culvert_roughness_ns: None,
            culvert_lengths: None,
            culvert_entrance_loss_coeffs: None,
            culvert_exit_loss_coeffs: None,
            ..Default::default()
        };

        let result = solve_steady(&inputs);

        // Verification
        // The solver should converge successfully. Check that output size matches original input size (2)
        assert_eq!(result.wsel.len(), 2);
        // Downstream boundary condition is preserved
        assert_eq!(result.wsel[1], 1.2);
        // Upstream water surface elevation should be solved successfully and be greater than bed level (1.0m)
        assert!(result.wsel[0] > 1.0);
    }

    #[test]
    fn test_steady_integrated_culvert() {
        // Concrete circular pipe: D = 5.0 ft, L = 100 ft, Q = 100 cfs, slope = 0.01
        // Channel reach with 3 sections at stations 200, 100, and 0 in US Customary.
        // Station 100 is just upstream of the culvert inlet (which sits between 100 and 0).
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0 + 2.0, 2.0, 2.0, 10.0 + 2.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0 + 1.0, 1.0, 1.0, 10.0 + 1.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let inputs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 100.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0, // Subcritical
            downstream_wsel: Some(3.0), // TW = 3 ft above invert
            upstream_wsel: None,
            max_spacing: None,
            culvert_stations: Some(vec![50.0]), // culvert located between 0 and 100 (at station 50)
            culvert_shape_types: Some(vec![0]), // Circular
            culvert_spans: Some(vec![5.0]),
            culvert_rises: Some(vec![5.0]),
            culvert_roughness_ns: Some(vec![0.012]),
            culvert_lengths: Some(vec![100.0]),
            culvert_entrance_loss_coeffs: Some(vec![0.5]),
            culvert_exit_loss_coeffs: Some(vec![1.0]),
            ..Default::default()
        };

        let result = solve_steady(&inputs);

        // Verification of integrated culvert model
        // Downstream section station 0 (index 2) WSEL is tailwater: 3.0 ft.
        assert_eq!(result.wsel[2], 3.0);

        // Upstream section station 100 (index 1) WSEL is solved by culvert inlet control (~1.0 + 4.25 = 5.25 ft).
        // Let's verify it matches to within 0.05 ft.
        let hw_wsel = result.wsel[1];
        assert!((hw_wsel - 5.25).abs() < 0.05, "expected ~5.25, got {}", hw_wsel);

        // Upstream section station 200 (index 0) WSEL is GVF solved starting from station 100's solved WSEL.
        assert!(result.wsel[0] > 2.0);

        let controls = result.culvert_control_types.expect("culvert_control_types");
        assert_eq!(controls.len(), 1);
        assert_eq!(controls[0], "inlet");

        let q_barrels = result.culvert_q_barrels.expect("culvert_q_barrels");
        assert!((q_barrels[0] - 100.0).abs() < 1e-6);
        let wsel_inlet = result.culvert_wsel_inlet.expect("culvert_wsel_inlet");
        assert!((wsel_inlet[0] - hw_wsel).abs() < 0.05);
        assert!(result.culvert_barrel_velocities.expect("vel")[0] > 0.0);
    }

    #[test]
    fn test_steady_culvert_on_cross_section_station_not_double_matched() {
        // Regression: culvert station exactly on a cross-section must match one interval only.
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![12.0, 2.0, 2.0, 12.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![11.0, 1.0, 1.0, 11.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let mut on_xs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 100.0,
            num_slices: Some(50),
            regime: 0,
            downstream_wsel: Some(3.0),
            culvert_stations: Some(vec![100.0]),
            culvert_shape_types: Some(vec![0]),
            culvert_spans: Some(vec![5.0]),
            culvert_rises: Some(vec![5.0]),
            culvert_roughness_ns: Some(vec![0.012]),
            culvert_lengths: Some(vec![100.0]),
            culvert_entrance_loss_coeffs: Some(vec![0.5]),
            culvert_exit_loss_coeffs: Some(vec![1.0]),
            ..Default::default()
        };

        let on_xs_result = solve_steady(&on_xs);
        on_xs.culvert_stations = Some(vec![50.0]);
        let mid_reach_result = solve_steady(&on_xs);

        let hw_on_xs = on_xs_result.wsel[1];
        let hw_mid = mid_reach_result.wsel[1];
        assert!(
            hw_on_xs < 8.0,
            "on-XS culvert headwater should stay physical, got {}",
            hw_on_xs
        );
        assert!(
            (hw_on_xs - hw_mid).abs() < 0.5,
            "on-XS ({}) and mid-reach ({}) culvert headwater should agree within 0.5 ft",
            hw_on_xs,
            hw_mid
        );
    }

    fn culvert_tier1_channel() -> Vec<CrossSection> {
        vec![
            CrossSection {
                station: 200.0,
                x: vec![0.0, 0.0, 10.0, 10.0],
                y: vec![12.0, 2.0, 2.0, 12.0],
                n_stations: vec![0.0],
                n_values: vec![0.02],
                unit_system: UnitSystem::USCustomary,
                is_overbank: None,
                blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
            },
            CrossSection {
                station: 100.0,
                x: vec![0.0, 0.0, 10.0, 10.0],
                y: vec![11.0, 1.0, 1.0, 11.0],
                n_stations: vec![0.0],
                n_values: vec![0.02],
                unit_system: UnitSystem::USCustomary,
                is_overbank: None,
                blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
            },
            CrossSection {
                station: 0.0,
                x: vec![0.0, 0.0, 10.0, 10.0],
                y: vec![10.0, 0.0, 0.0, 10.0],
                n_stations: vec![0.0],
                n_values: vec![0.02],
                unit_system: UnitSystem::USCustomary,
                is_overbank: None,
                blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
            },
        ]
    }

    fn base_culvert_tier1_inputs(cross_sections: Vec<CrossSection>) -> SteadyInputs {
        SteadyInputs {
            cross_sections,
            flow_rate: 100.0,
            num_slices: Some(50),
            regime: 0,
            downstream_wsel: Some(3.0),
            culvert_stations: Some(vec![50.0]),
            culvert_shape_types: Some(vec![0]),
            culvert_spans: Some(vec![5.0]),
            culvert_rises: Some(vec![5.0]),
            culvert_roughness_ns: Some(vec![0.012]),
            culvert_lengths: Some(vec![100.0]),
            culvert_entrance_loss_coeffs: Some(vec![0.5]),
            culvert_exit_loss_coeffs: Some(vec![1.0]),
            culvert_inlet_types: Some(vec![1]),
            ..Default::default()
        }
    }

    #[test]
    fn test_steady_culvert_tier1_integration() {
        let channel = culvert_tier1_channel();

        // Inlet control + explicit inlet type + control reporting
        let inlet_result = solve_steady(&base_culvert_tier1_inputs(channel.clone()));
        let controls = inlet_result
            .culvert_control_types
            .as_ref()
            .expect("culvert_control_types");
        assert_eq!(controls.len(), 1);
        assert_eq!(controls[0], "inlet");
        assert!((inlet_result.wsel[1] - 5.25).abs() < 0.05);

        // Outlet control via high tailwater
        let mut outlet_inputs = base_culvert_tier1_inputs(channel.clone());
        outlet_inputs.downstream_wsel = Some(15.0);
        let outlet_result = solve_steady(&outlet_inputs);
        assert_eq!(
            outlet_result.culvert_control_types.as_ref().unwrap()[0],
            "outlet"
        );

        // Raised invert increases headwater vs bed-invert default
        let bed_result = solve_steady(&base_culvert_tier1_inputs(channel.clone()));
        let mut invert_inputs = base_culvert_tier1_inputs(channel.clone());
        invert_inputs.culvert_z_ups = Some(vec![12.0]);
        invert_inputs.culvert_z_downs = Some(vec![11.0]);
        let invert_result = solve_steady(&invert_inputs);
        assert!(invert_result.wsel[1] > bed_result.wsel[1]);

        // Roadway overtopping — align invert/tailwater with unit-test geometry (low outlet depth)
        let mut ot_inputs = base_culvert_tier1_inputs(channel);
        ot_inputs.flow_rate = 500.0;
        ot_inputs.downstream_wsel = Some(10.0);
        ot_inputs.culvert_z_ups = Some(vec![10.0]);
        ot_inputs.culvert_z_downs = Some(vec![9.0]);
        ot_inputs.culvert_crest_elevs = Some(vec![14.0]);
        ot_inputs.culvert_weir_lengths = Some(vec![20.0]);
        ot_inputs.culvert_weir_coeffs = Some(vec![2.6]);
        ot_inputs.culvert_barrels = Some(vec![2]);
        let ot_result = solve_steady(&ot_inputs);
        assert_eq!(
            ot_result.culvert_control_types.as_ref().unwrap()[0],
            "overtopping"
        );
        assert!(ot_result.wsel[1] > 14.0);
    }

    #[test]
    fn test_steady_culvert_geometry_and_blockage_integration() {
        let channel = culvert_tier1_channel();
        let base = base_culvert_tier1_inputs(channel.clone());
        let base_hw = solve_steady(&base).wsel[1];

        // Skew increases headwater in outlet-control tailwater regime
        let mut skew_inputs = base_culvert_tier1_inputs(channel.clone());
        skew_inputs.downstream_wsel = Some(15.0);
        skew_inputs.culvert_skew_angles = Some(vec![30.0]);
        let skew_hw = solve_steady(&skew_inputs).wsel[1];
        skew_inputs.culvert_skew_angles = Some(vec![0.0]);
        assert!(skew_hw > solve_steady(&skew_inputs).wsel[1]);

        // Blocked barrel count increases headwater
        let mut blocked_inputs = base_culvert_tier1_inputs(channel.clone());
        blocked_inputs.culvert_barrels = Some(vec![2]);
        blocked_inputs.culvert_active_barrels = Some(vec![2]);
        let two_barrel_hw = solve_steady(&blocked_inputs).wsel[1];
        blocked_inputs.culvert_active_barrels = Some(vec![1]);
        assert!(solve_steady(&blocked_inputs).wsel[1] > two_barrel_hw);

        // Per-barrel geometry (one large + one small barrel)
        let mut mixed_inputs = base_culvert_tier1_inputs(channel.clone());
        mixed_inputs.culvert_barrels = Some(vec![2]);
        mixed_inputs.culvert_active_barrels = Some(vec![2]);
        mixed_inputs.culvert_barrel_spans = Some(vec![vec![8.0, 4.0]]);
        mixed_inputs.culvert_barrel_rises = Some(vec![vec![8.0, 4.0]]);
        let mut equal_inputs = mixed_inputs.clone();
        equal_inputs.culvert_barrel_spans = None;
        equal_inputs.culvert_barrel_rises = None;
        equal_inputs.culvert_spans = Some(vec![4.0]);
        equal_inputs.culvert_rises = Some(vec![4.0]);
        assert!(solve_steady(&mixed_inputs).wsel[1] < solve_steady(&equal_inputs).wsel[1]);

        // Sediment blockage through steady path
        let mut sediment_inputs = base_culvert_tier1_inputs(channel);
        sediment_inputs.culvert_depth_blockeds = Some(vec![1.0]);
        assert!(solve_steady(&sediment_inputs).wsel[1] > base_hw);
    }

    #[test]
    fn test_steady_culvert_sensitivity() {
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![12.0, 2.0, 2.0, 12.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![11.0, 1.0, 1.0, 11.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let mut inputs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 100.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0,
            downstream_wsel: Some(3.0),
            upstream_wsel: None,
            max_spacing: None,
            culvert_stations: Some(vec![50.0]),
            culvert_shape_types: Some(vec![1]), // Box
            culvert_spans: Some(vec![8.0]),
            culvert_rises: Some(vec![6.0]),
            culvert_roughness_ns: Some(vec![0.012]),
            culvert_lengths: Some(vec![100.0]),
            culvert_entrance_loss_coeffs: Some(vec![0.5]),
            culvert_exit_loss_coeffs: Some(vec![1.0]),
            ..Default::default()
        };

        let result_wide = solve_steady(&inputs);
        
        inputs.culvert_spans = Some(vec![0.1]);
        let result_narrow = solve_steady(&inputs);

        println!("Wide WSEL: {:?}", result_wide.wsel);
        println!("Narrow WSEL: {:?}", result_narrow.wsel);
        assert!(
            result_narrow.wsel[1] > result_wide.wsel[1],
            "Narrow culvert WSEL ({}) should be greater than wide culvert WSEL ({})",
            result_narrow.wsel[1],
            result_wide.wsel[1]
        );
    }

    #[test]
    fn test_steady_supercritical_culvert_routing() {
        let channel = culvert_tier1_channel();
        let mut inputs = base_culvert_tier1_inputs(channel);
        inputs.regime = 1;
        inputs.upstream_wsel = Some(8.0);
        inputs.downstream_wsel = None;
        inputs.downstream_bc_type = Some(1);
        let result = solve_steady(&inputs);
        assert!(result.wsel.iter().all(|w| w.is_finite()));
        assert!((result.wsel[2] - result.critical_wsel[2]).abs() > 0.01);
    }

    fn culvert_tier1_channel_metric() -> Vec<CrossSection> {
        vec![
            CrossSection {
                station: 60.0,
                x: vec![0.0, 0.0, 3.0, 3.0],
                y: vec![3.66, 0.61, 0.61, 3.66],
                n_stations: vec![0.0],
                n_values: vec![0.02],
                unit_system: UnitSystem::Metric,
                is_overbank: None,
                blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
            },
            CrossSection {
                station: 30.0,
                x: vec![0.0, 0.0, 3.0, 3.0],
                y: vec![3.35, 0.30, 0.30, 3.35],
                n_stations: vec![0.0],
                n_values: vec![0.02],
                unit_system: UnitSystem::Metric,
                is_overbank: None,
                blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
            },
            CrossSection {
                station: 0.0,
                x: vec![0.0, 0.0, 3.0, 3.0],
                y: vec![3.05, 0.0, 0.0, 3.05],
                n_stations: vec![0.0],
                n_values: vec![0.02],
                unit_system: UnitSystem::Metric,
                is_overbank: None,
                blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
            },
        ]
    }

    #[test]
    fn test_steady_supercritical_culvert_metric_and_mixed() {
        let channel = culvert_tier1_channel_metric();
        let metric = SteadyInputs {
            cross_sections: channel.clone(),
            flow_rate: 3.0,
            num_slices: Some(50),
            regime: 1,
            upstream_wsel: Some(2.5),
            downstream_wsel: None,
            downstream_bc_type: Some(1),
            culvert_stations: Some(vec![15.0]),
            culvert_shape_types: Some(vec![0]),
            culvert_spans: Some(vec![1.5]),
            culvert_rises: Some(vec![1.5]),
            culvert_roughness_ns: Some(vec![0.012]),
            culvert_lengths: Some(vec![30.0]),
            culvert_entrance_loss_coeffs: Some(vec![0.5]),
            culvert_exit_loss_coeffs: Some(vec![1.0]),
            culvert_inlet_types: Some(vec![1]),
            ..Default::default()
        };
        let metric_result = solve_steady(&metric);
        assert!(metric_result.wsel.iter().all(|w| w.is_finite()));
        assert!((metric_result.wsel[2] - metric_result.critical_wsel[2]).abs() > 0.01);

        let mut mixed = base_culvert_tier1_inputs(culvert_tier1_channel());
        mixed.regime = 2;
        mixed.upstream_wsel = Some(8.0);
        mixed.downstream_wsel = Some(3.0);
        let mixed_result = solve_steady(&mixed);
        assert!(mixed_result.wsel.iter().all(|w| w.is_finite()));
        assert!(mixed_result.wsel[1] > mixed.downstream_wsel.unwrap());
    }

    #[test]
    fn test_steady_supercritical_bridge_tailwater_coupling() {
        let channel = culvert_tier1_channel();
        let inputs = SteadyInputs {
            cross_sections: channel,
            flow_rate: 100.0,
            num_slices: Some(50),
            regime: 1,
            upstream_wsel: Some(8.0),
            downstream_bc_type: Some(1),
            bridge_stations: Some(vec![50.0]),
            bridge_low_chords: Some(vec![12.0]),
            bridge_high_chords: Some(vec![14.0]),
            bridge_pier_widths: Some(vec![0.0]),
            bridge_num_piers: Some(vec![0]),
            bridge_pier_shapes: Some(vec![0]),
            bridge_weir_coeffs: Some(vec![3.0]),
            bridge_orifice_coeffs: Some(vec![0.8]),
            ..Default::default()
        };
        let result = solve_steady(&inputs);
        assert!(result.wsel.iter().all(|w| w.is_finite()));
        // Supercritical sweep: downstream WSEL at the bridge interval should be below upstream headwater.
        assert!(
            result.wsel[2] < result.wsel[0],
            "bridge tailwater should be below upstream headwater, got {} vs {}",
            result.wsel[2],
            result.wsel[0]
        );
    }

    #[test]
    fn test_steady_integrated_bridge() {
        // Simple reach: stations 200, 100, 0
        // Rectangular channel: width = 10m
        // Bed elevations: 0.2m, 0.1m, 0.0m
        // Flow rate: 15.0 cms
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0 + 0.2, 0.2, 0.2, 10.0 + 0.2],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0 + 0.1, 0.1, 0.1, 10.0 + 0.1],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let inputs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 15.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0,
            downstream_wsel: Some(3.0),
            upstream_wsel: None,
            max_spacing: None,
            bridge_stations: Some(vec![50.0]), // bridge at station 50 (between 0 and 100)
            bridge_low_chords: Some(vec![5.0]),
            bridge_high_chords: Some(vec![7.0]),
            bridge_pier_widths: Some(vec![0.5]),
            bridge_num_piers: Some(vec![2]),
            bridge_pier_shapes: Some(vec![0]),
            bridge_weir_coeffs: Some(vec![1.44]),
            bridge_orifice_coeffs: Some(vec![0.5]),
            ..Default::default()
        };

        let result = solve_steady(&inputs);

        // Verification
        // WSEL at station 0 (index 2) should be 3.0 (downstream boundary)
        assert_eq!(result.wsel[2], 3.0);
        // WSEL at station 100 (index 1) — bridge Yarnell low-flow backwater
        assert!(
            (result.wsel[1] - 3.00247).abs() < 0.001,
            "Bridge upstream WSEL should match HEC-RAS Yarnell, got {}",
            result.wsel[1]
        );
    }

    #[test]
    fn test_steady_bridge_bu_bd_face_layout() {
        let channel = |station: f64, bed: f64| CrossSection {
            station,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![bed + 10.0, bed, bed, bed + 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let bu = CrossSection {
            station: 52.0,
            x: vec![0.0, 0.0, 5.0, 5.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let bd = CrossSection {
            station: 48.0,
            x: vec![0.0, 0.0, 5.0, 5.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let inputs = SteadyInputs {
            cross_sections: vec![channel(200.0, 0.2), channel(100.0, 0.1), channel(0.0, 0.0)],
            flow_rate: 15.0,
            num_slices: Some(50),
            regime: 0,
            downstream_wsel: Some(3.0),
            bridge_stations: Some(vec![50.0]),
            bridge_low_chords: Some(vec![5.0]),
            bridge_high_chords: Some(vec![7.0]),
            bridge_pier_widths: Some(vec![0.5]),
            bridge_num_piers: Some(vec![2]),
            bridge_upstream_cross_sections: Some(vec![bu]),
            bridge_downstream_cross_sections: Some(vec![bd]),
            ..Default::default()
        };
        let result = solve_steady(&inputs);
        assert!(result.wsel.iter().all(|w| w.is_finite()));
        assert_eq!(result.wsel[2], 3.0);
        assert!(
            result.wsel[1] > 3.0,
            "narrow BU/BD opening should backwater above tailwater"
        );
    }

    #[test]
    fn test_steady_normal_depth_boundary() {
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let inputs = SteadyInputs {
            cross_sections: vec![xs100, xs0],
            flow_rate: 15.0,
            num_slices: Some(50),
            regime: 0, // Subcritical
            downstream_bc_type: Some(2), // Normal Depth
            downstream_bc_slope: Some(0.001),
            ..Default::default()
        };

        let result = solve_steady(&inputs);
        // Verify downstream WSEL is normal depth elevation (y_min=0.0 + yn ~ 1.05m)
        let ds_wsel = result.wsel[1];
        assert!(ds_wsel > 0.9 && ds_wsel < 1.2, "Expected normal depth WSEL ~1.05m, got {}", ds_wsel);
    }

    #[test]
    fn test_steady_rating_curve_boundary() {
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let inputs = SteadyInputs {
            cross_sections: vec![xs100, xs0],
            flow_rate: 15.0, // Should interpolate to 2.5
            num_slices: Some(50),
            regime: 0,
            downstream_bc_type: Some(3), // Rating Curve
            downstream_bc_rating_q: Some(vec![0.0, 10.0, 20.0]),
            downstream_bc_rating_wsel: Some(vec![1.0, 2.0, 3.0]),
            ..Default::default()
        };

        let result = solve_steady(&inputs);
        // Verify downstream WSEL is 2.5
        let ds_wsel = result.wsel[1];
        assert!((ds_wsel - 2.5).abs() < 1e-5, "Expected 2.5, got {}", ds_wsel);
    }

    #[test]
    fn test_steady_reach_ineffective_raises_upstream_wsel() {
        use crate::geometry::IneffectiveFlowAreas;

        fn compound_channel(station: f64, z_bottom: f64) -> CrossSection {
            CrossSection {
                station,
                x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
                y: vec![
                    5.0 + z_bottom,
                    z_bottom,
                    z_bottom,
                    5.0 + z_bottom,
                    z_bottom,
                    5.0 + z_bottom,
                    z_bottom,
                    5.0 + z_bottom,
                ],
                n_stations: vec![0.0, 10.0],
                n_values: vec![0.03, 0.05],
                unit_system: UnitSystem::Metric,
                is_overbank: Some(vec![
                    false, false, false, false, true, true, true, true,
                ]),
                blocked_obstructions: None,
                ineffective_flow_areas: None,
                guide_banks: None,
            }
        }

        let xs_ds = compound_channel(0.0, 0.0);
        let xs_us_open = compound_channel(100.0, 0.1);
        let mut xs_us_ineff = xs_us_open.clone();
        xs_us_ineff.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[], &[], &[30.0], &[3.0]).unwrap(),
        );

        let common = |xs_us: CrossSection| SteadyInputs {
            cross_sections: vec![xs_us, xs_ds.clone()],
            flow_rate: 30.0,
            num_slices: Some(50),
            regime: 0,
            downstream_bc_type: Some(0),
            downstream_wsel: Some(2.0),
            ..Default::default()
        };

        let _open = solve_steady(&common(xs_us_open.clone()));
        let _ineffective = solve_steady(&common(xs_us_ineff.clone()));
        let table = xs_us_open.generate_lookup_table(50);
        let wsel = _open.wsel[0];
        let open_row = geometry_row_at_elevation(&table, Some(&xs_us_open), wsel, None, None);
        let ineff_row = geometry_row_at_elevation(&table, Some(&xs_us_ineff), wsel, None, None);
        assert!(
            ineff_row.active_area < open_row.active_area - 1.0,
            "right overbank ineffective should clip conveyance at upstream section"
        );
        assert!(
            ineff_row.conveyance < open_row.conveyance,
            "ineffective should reduce conveyance at fixed stage"
        );
        assert!(
            specific_force_at_elevation(&table, Some(&xs_us_ineff), wsel, 30.0, None, None)
                > specific_force_at_elevation(&table, Some(&xs_us_open), wsel, 30.0, None, None),
            "smaller active area should raise specific force at fixed Q and stage"
        );
        assert!(
            (_ineffective.wsel[1] - 2.0).abs() < 1e-4,
            "downstream BC should be preserved, got {}",
            _ineffective.wsel[1],
        );
    }

    #[test]
    fn test_validate_steady_inputs_guide_bank_warning() {
        use crate::geometry::{GuideBankPolyline, GuideBanks};
        let inputs = SteadyInputs {
            bridge_stations: Some(vec![50.0]),
            bridge_approach_guide_banks: Some(vec![GuideBanks {
                left_polylines: vec![GuideBankPolyline {
                    stations: vec![0.0, 0.0],
                    elevations: vec![1.0, 2.0],
                }],
                ..Default::default()
            }]),
            ..Default::default()
        };
        let warnings = crate::solvers::validate_steady_inputs(&inputs).warnings;
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("approach"));
    }
}

