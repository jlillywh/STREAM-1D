use crate::geometry::{
    row_at_elevation, CrossSection, GeometryRow, GeometryTable, IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::{
    resolve_abutments, BridgeAbutmentUserInput, BridgeAbutments,
};
use crate::solvers::culvert::apply_barrel_skew;
use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS, G_METRIC};

/// Cross-section context for bridge-adjacent ineffective flow areas.
#[derive(Debug, Clone, Default)]
pub struct BridgeSectionContext {
    /// Ineffective blocks at the upstream bridge face.
    pub ineffective_up: Option<IneffectiveFlowAreas>,
    /// Ineffective blocks at the downstream bridge face.
    pub ineffective_down: Option<IneffectiveFlowAreas>,
    pub xs_up: Option<CrossSection>,
    pub xs_down: Option<CrossSection>,
    /// Skew from normal to flow, degrees (0–59°; same convention as culverts).
    pub skew_deg: f64,
    /// Pier centerline stations across the opening (user units; same frame as deck stations).
    pub pier_stations: Option<Vec<f64>>,
}

/// HEC-RAS-style bridge skew: projected opening width × cos(θ), friction length ÷ cos(θ).
pub fn apply_bridge_skew(skew_deg: f64, width_m: f64, length_m: f64) -> (f64, f64) {
    apply_barrel_skew(skew_deg, width_m, length_m)
}

/// Supported pier shape types (Yarnell K and momentum drag coefficients per HEC-RAS).
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

    /// Yarnell pier shape coefficient K (HEC-RAS).
    pub fn yarnell_coefficient(&self) -> f64 {
        match self {
            PierShape::Square => 1.25,
            PierShape::Semicircular => 0.90,
            PierShape::TwinCylinder => 0.95,
            PierShape::Triangular => 1.05,
        }
    }

    /// Momentum pier drag coefficient C_D (HEC-RAS pier drag table).
    pub fn drag_coefficient(&self) -> f64 {
        match self {
            PierShape::Semicircular => 1.20,
            PierShape::TwinCylinder => 1.33,
            PierShape::Triangular => 1.60,
            PierShape::Square => 2.00,
        }
    }
}

/// HEC-RAS low-flow classification through a bridge opening.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LowFlowClass {
    /// Completely subcritical through the bridge (Yarnell / momentum / energy).
    A,
    /// Passes through critical depth in the constriction (momentum balance).
    B,
    /// Completely supercritical through the bridge (momentum balance).
    C,
}

/// Low-flow method selection for Class A profile through the bridge opening.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LowFlowMethod {
    /// Classify A/B/C; Class A uses Yarnell (piers), WSPRO (abutments), or energy.
    Auto = 0,
    Yarnell = 1,
    Momentum = 2,
    /// HEC-RAS energy (standard step) through the obstructed opening.
    Energy = 3,
    /// FHWA WSPRO contracted-opening energy with discharge coefficient C.
    Wspro = 4,
}

impl LowFlowMethod {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => LowFlowMethod::Yarnell,
            2 => LowFlowMethod::Momentum,
            3 => LowFlowMethod::Energy,
            4 => LowFlowMethod::Wspro,
            _ => LowFlowMethod::Auto,
        }
    }
}

/// High-flow method when downstream tailwater is at or above the low chord.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HighFlowMethod {
    /// Sluice-gate / submerged-orifice pressure flow and Bradley weir overtopping (HEC-RAS default).
    PressureWeir = 0,
    /// Standard-step energy through the obstructed opening (also used as submergence fallback).
    Energy = 1,
}

impl HighFlowMethod {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => HighFlowMethod::Energy,
            _ => HighFlowMethod::PressureWeir,
        }
    }
}

/// Per-bridge coupling parameters (steady and unsteady).
#[derive(Debug, Clone)]
pub struct BridgeCouplingParams {
    pub abutment: BridgeAbutmentUserInput,
    pub low_flow_method: i32,
    /// High-flow method: 0 = pressure/weir, 1 = energy.
    pub high_flow_method: i32,
    /// Reach length through the bridge for friction (user units). 0 uses interval length or 10 m.
    pub length: f64,
    /// WSPRO contracted-opening discharge coefficient C (typical 0.7–0.9).
    pub wspro_coeff: f64,
    pub coeff_contraction: f64,
    pub coeff_expansion: f64,
    /// Sluice-gate pressure coefficient when only upstream is submerged. 0 = auto from Y3/Z (HEC-RAS).
    pub pressure_coeff_inlet: f64,
    /// Fully submerged pressure coefficient when both sides are under the deck (typical 0.8).
    pub pressure_coeff_submerged: f64,
    /// Switch to energy method when weir submergence ratio exceeds this (HEC-RAS default 0.98).
    pub max_weir_submergence: f64,
}

impl Default for BridgeCouplingParams {
    fn default() -> Self {
        Self {
            abutment: BridgeAbutmentUserInput::default(),
            low_flow_method: 0,
            high_flow_method: 0,
            length: 0.0,
            wspro_coeff: 0.8,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3,
            pressure_coeff_inlet: 0.0,
            pressure_coeff_submerged: 0.8,
            max_weir_submergence: 0.98,
        }
    }
}

/// Parameters for a standalone bridge headwater solve (rating curve or direct API).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeSolveParams {
    /// Discharge (user units). Ignored when sampling a rating curve.
    #[serde(default)]
    pub q: f64,
    pub low_chord: f64,
    pub high_chord: f64,
    pub z_down: f64,
    pub z_up: f64,
    /// Fixed downstream tailwater for subcritical rating (user units).
    pub tw_wsel: f64,
    pub units: UnitSystem,
    #[serde(default)]
    pub pier_width: f64,
    #[serde(default)]
    pub num_piers: i32,
    #[serde(default)]
    pub pier_shape_type: i32,
    /// Weir coefficient (0 = default 2.6 US / 1.44 metric).
    #[serde(default)]
    pub weir_coeff: f64,
    /// Orifice/pressure coefficient (0 = default 0.5 submerged / auto sluice).
    #[serde(default)]
    pub orifice_coeff: f64,
    /// Legacy total abutment block width (left + right, perpendicular to flow).
    #[serde(default)]
    pub abutment_block_width: f64,
    /// Left abutment width (perpendicular to flow). With `abutment_right_width`, overrides legacy total.
    #[serde(default)]
    pub abutment_left_width: Option<f64>,
    #[serde(default)]
    pub abutment_right_width: Option<f64>,
    /// Left abutment outer-face station in opening coordinates (default: opening left edge).
    #[serde(default)]
    pub abutment_left_station: Option<f64>,
    /// Right abutment outer-face station in opening coordinates (default: opening right edge).
    #[serde(default)]
    pub abutment_right_station: Option<f64>,
    /// Constant top elevation for left abutment (omit for full-height blockage).
    #[serde(default)]
    pub abutment_left_top_elevation: Option<f64>,
    #[serde(default)]
    pub abutment_right_top_elevation: Option<f64>,
    /// Piecewise top profile for left abutment (`stations` + `elevations`, ≥ 2 points).
    #[serde(default)]
    pub abutment_left_top_profile_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub abutment_left_top_profile_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub abutment_right_top_profile_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub abutment_right_top_profile_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub low_flow_method: i32,
    /// High-flow method: 0 = pressure/weir, 1 = energy.
    #[serde(default)]
    pub high_flow_method: i32,
    /// Reach length through bridge for friction (user units). 0 uses 50 ft or 15 m.
    #[serde(default)]
    pub length: f64,
    #[serde(default = "default_wspro_coeff")]
    pub wspro_coeff: f64,
    #[serde(default = "default_coeff_contraction")]
    pub coeff_contraction: f64,
    #[serde(default = "default_coeff_expansion")]
    pub coeff_expansion: f64,
    #[serde(default)]
    pub pressure_coeff_inlet: f64,
    #[serde(default = "default_max_weir_submergence")]
    pub max_weir_submergence: f64,
    #[serde(default)]
    pub skew_deg: f64,
    #[serde(default)]
    pub pier_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_low_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_high_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_left_station: Option<f64>,
    #[serde(default)]
    pub ineffective_left_elevation: Option<f64>,
    #[serde(default)]
    pub ineffective_right_station: Option<f64>,
    #[serde(default)]
    pub ineffective_right_elevation: Option<f64>,
    /// Multi-block left ineffective stations (falls back to scalar `ineffective_left_station`).
    #[serde(default)]
    pub ineffective_left_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_left_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_right_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_right_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_left_station_upstream: Option<f64>,
    #[serde(default)]
    pub ineffective_left_elevation_upstream: Option<f64>,
    #[serde(default)]
    pub ineffective_right_station_upstream: Option<f64>,
    #[serde(default)]
    pub ineffective_right_elevation_upstream: Option<f64>,
    #[serde(default)]
    pub ineffective_left_stations_upstream: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_left_elevations_upstream: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_right_stations_upstream: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_right_elevations_upstream: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_left_station_downstream: Option<f64>,
    #[serde(default)]
    pub ineffective_left_elevation_downstream: Option<f64>,
    #[serde(default)]
    pub ineffective_right_station_downstream: Option<f64>,
    #[serde(default)]
    pub ineffective_right_elevation_downstream: Option<f64>,
    #[serde(default)]
    pub ineffective_left_stations_downstream: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_left_elevations_downstream: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_right_stations_downstream: Option<Vec<f64>>,
    #[serde(default)]
    pub ineffective_right_elevations_downstream: Option<Vec<f64>>,
    /// Approach/departure channel width when `xs_up` / `xs_down` are omitted.
    #[serde(default = "default_channel_width")]
    pub channel_width: f64,
    #[serde(default)]
    pub manning_n: f64,
    #[serde(default = "default_num_slices")]
    pub num_slices: usize,
    /// Optional explicit cross sections (override `channel_width` rectangular default).
    #[serde(default)]
    pub xs_up: Option<CrossSection>,
    #[serde(default)]
    pub xs_down: Option<CrossSection>,
}

fn default_wspro_coeff() -> f64 {
    0.8
}

fn default_coeff_contraction() -> f64 {
    0.1
}

fn default_coeff_expansion() -> f64 {
    0.3
}

fn default_max_weir_submergence() -> f64 {
    0.98
}

fn default_channel_width() -> f64 {
    10.0
}

fn default_num_slices() -> usize {
    50
}

impl Default for BridgeSolveParams {
    fn default() -> Self {
        Self {
            q: 0.0,
            low_chord: 0.0,
            high_chord: 0.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 0.0,
            units: UnitSystem::Metric,
            pier_width: 0.0,
            num_piers: 0,
            pier_shape_type: 0,
            weir_coeff: 0.0,
            orifice_coeff: 0.0,
            abutment_block_width: 0.0,
            abutment_left_width: None,
            abutment_right_width: None,
            abutment_left_station: None,
            abutment_right_station: None,
            abutment_left_top_elevation: None,
            abutment_right_top_elevation: None,
            abutment_left_top_profile_stations: None,
            abutment_left_top_profile_elevations: None,
            abutment_right_top_profile_stations: None,
            abutment_right_top_profile_elevations: None,
            low_flow_method: 0,
            high_flow_method: 0,
            length: 0.0,
            wspro_coeff: default_wspro_coeff(),
            coeff_contraction: default_coeff_contraction(),
            coeff_expansion: default_coeff_expansion(),
            pressure_coeff_inlet: 0.0,
            max_weir_submergence: default_max_weir_submergence(),
            skew_deg: 0.0,
            pier_stations: None,
            deck_stations: None,
            deck_low_elevations: None,
            deck_high_elevations: None,
            ineffective_left_station: None,
            ineffective_left_elevation: None,
            ineffective_right_station: None,
            ineffective_right_elevation: None,
            ineffective_left_stations: None,
            ineffective_left_elevations: None,
            ineffective_right_stations: None,
            ineffective_right_elevations: None,
            ineffective_left_station_upstream: None,
            ineffective_left_elevation_upstream: None,
            ineffective_right_station_upstream: None,
            ineffective_right_elevation_upstream: None,
            ineffective_left_stations_upstream: None,
            ineffective_left_elevations_upstream: None,
            ineffective_right_stations_upstream: None,
            ineffective_right_elevations_upstream: None,
            ineffective_left_station_downstream: None,
            ineffective_left_elevation_downstream: None,
            ineffective_right_station_downstream: None,
            ineffective_right_elevation_downstream: None,
            ineffective_left_stations_downstream: None,
            ineffective_left_elevations_downstream: None,
            ineffective_right_stations_downstream: None,
            ineffective_right_elevations_downstream: None,
            channel_width: default_channel_width(),
            manning_n: 0.0,
            num_slices: default_num_slices(),
            xs_up: None,
            xs_down: None,
        }
    }
}

/// Headwater rating curve for a bridge at fixed tailwater.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeRatingCurveInputs {
    pub q_values: Vec<f64>,
    /// Bridge geometry and tailwater; field `q` is ignored.
    #[serde(flatten)]
    pub bridge: BridgeSolveParams,
}

/// Headwater vs discharge samples for one bridge opening.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeRatingCurveResult {
    pub q: Vec<f64>,
    /// Upstream headwater (same role as culvert rating `wsel`).
    pub wsel: Vec<f64>,
    pub wsel_down: Vec<f64>,
    pub flow_regimes: Vec<String>,
    pub head_losses: Vec<f64>,
}

/// Piecewise-linear deck geometry across the bridge opening (HEC-RAS deck/roadway profiles).
#[derive(Debug, Clone, Default)]
pub struct BridgeDeckProfile {
    /// Horizontal stations across the opening (metric, monotonic increasing).
    pub stations_m: Vec<f64>,
    /// Low chord elevation at each station.
    pub low_elevations_m: Vec<f64>,
    /// High chord (roadway crest) elevation at each station.
    pub high_elevations_m: Vec<f64>,
}

impl BridgeDeckProfile {
    pub fn is_valid(&self) -> bool {
        let n = self.stations_m.len();
        n >= 2
            && n == self.low_elevations_m.len()
            && n == self.high_elevations_m.len()
            && self.stations_m.windows(2).all(|w| w[1] > w[0])
    }
}

/// Build a deck profile from optional per-point arrays; returns `None` when fewer than two points.
pub fn build_bridge_deck_profile(
    _scalar_low: f64,
    _scalar_high: f64,
    stations: Option<&[f64]>,
    low_elevs: Option<&[f64]>,
    high_elevs: Option<&[f64]>,
    units: UnitSystem,
) -> Option<BridgeDeckProfile> {
    let st = stations?;
    let lo = low_elevs?;
    let hi = high_elevs?;
    if st.len() < 2 || st.len() != lo.len() || st.len() != hi.len() {
        return None;
    }
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let profile = BridgeDeckProfile {
        stations_m: st.iter().map(|s| to_m(*s)).collect(),
        low_elevations_m: lo.iter().map(|e| to_m(*e)).collect(),
        high_elevations_m: hi.iter().map(|e| to_m(*e)).collect(),
    };
    if profile.is_valid() {
        Some(profile)
    } else {
        None
    }
}

fn interpolate_profile(stations: &[f64], elevations: &[f64], station: f64) -> f64 {
    if stations.is_empty() {
        return 0.0;
    }
    if station <= stations[0] {
        return elevations[0];
    }
    if station >= stations[stations.len() - 1] {
        return elevations[elevations.len() - 1];
    }
    for i in 0..stations.len() - 1 {
        if station <= stations[i + 1] {
            let t = (station - stations[i]) / (stations[i + 1] - stations[i]);
            return elevations[i] + t * (elevations[i + 1] - elevations[i]);
        }
    }
    elevations[elevations.len() - 1]
}

fn deck_extrema(
    scalar_low: f64,
    scalar_high: f64,
    deck: Option<&BridgeDeckProfile>,
    units: UnitSystem,
) -> (f64, f64, f64, f64) {
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let low_m = to_m(scalar_low);
    let high_m = to_m(scalar_high);
    if let Some(d) = deck.filter(|p| p.is_valid()) {
        let low_min = d
            .low_elevations_m
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let low_max = d
            .low_elevations_m
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let high_min = d
            .high_elevations_m
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let high_max = d
            .high_elevations_m
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        (low_min, low_max, high_min, high_max)
    } else {
        (low_m, low_m, high_m, high_m)
    }
}

fn profile_opening_area_factor(geom: &BridgeGeometry) -> f64 {
    let deck = match &geom.deck {
        Some(d) if d.is_valid() => d,
        _ => return 1.0,
    };
    let z = geom.z_up_m;
    let mut trap_area = 0.0;
    let mut width = 0.0;
    for i in 0..deck.stations_m.len() - 1 {
        let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
        if w <= 0.0 {
            continue;
        }
        let h0 = (deck.low_elevations_m[i] - z).max(0.0);
        let h1 = (deck.low_elevations_m[i + 1] - z).max(0.0);
        trap_area += 0.5 * (h0 + h1) * w;
        width += w;
    }
    let h_min = (geom.low_chord_m - z).max(0.0);
    let rect_area = h_min * width.max(1e-6);
    if rect_area > 1e-6 {
        (trap_area / rect_area).clamp(0.05, 2.0)
    } else {
        1.0
    }
}

fn effective_weir_length_m(geom: &BridgeGeometry, e_upstream: f64, fallback: f64) -> f64 {
    let deck = match &geom.deck {
        Some(d) if d.is_valid() => d,
        _ => return fallback,
    };
    let mut len = 0.0;
    for i in 0..deck.stations_m.len() - 1 {
        let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
        if w <= 0.0 {
            continue;
        }
        let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
        let high_mid = interpolate_profile(
            &deck.stations_m,
            &deck.high_elevations_m,
            s_mid,
        );
        if e_upstream > high_mid {
            len += w;
        }
    }
    if len > 1e-3 {
        len
    } else {
        fallback.max(1e-3)
    }
}

/// Bridge geometry and coefficients in metric units.
#[derive(Debug, Clone)]
pub struct BridgeGeometry {
    /// Minimum low chord (deck-soffit) elevation — free-flow limit.
    pub low_chord_m: f64,
    /// Maximum low chord elevation — HEC-RAS pressure-flow EGL trigger.
    pub low_chord_max_m: f64,
    /// Minimum high chord (roadway crest) — weir overtopping begins.
    pub high_chord_m: f64,
    /// Maximum high chord elevation across the deck profile.
    pub high_chord_max_m: f64,
    pub pier_width_m: f64,
    pub num_piers: i32,
    /// Explicit pier centerline stations (metric); empty → evenly spaced across opening.
    pub pier_stations_m: Vec<f64>,
    pub skew_deg: f64,
    pub skew_cos: f64,
    pub pier_shape: PierShape,
    pub abutments: BridgeAbutments,
    pub weir_coeff_m: f64,
    pub orifice_coeff: f64,
    pub z_up_m: f64,
    pub z_down_m: f64,
    pub low_flow_method: LowFlowMethod,
    pub high_flow_method: HighFlowMethod,
    pub length_m: f64,
    pub wspro_coeff_c: f64,
    pub coeff_contraction: f64,
    pub coeff_expansion: f64,
    pub pressure_coeff_inlet: f64,
    pub pressure_coeff_submerged: f64,
    pub max_weir_submergence: f64,
    pub deck: Option<BridgeDeckProfile>,
    pub ineffective_up: Option<IneffectiveFlowAreas>,
    pub ineffective_down: Option<IneffectiveFlowAreas>,
    pub xs_up: Option<CrossSection>,
    pub xs_down: Option<CrossSection>,
}

fn ineffective_for_side(geom: &BridgeGeometry, is_upstream: bool) -> Option<&IneffectiveFlowAreas> {
    if is_upstream {
        geom.ineffective_up.as_ref()
    } else {
        geom.ineffective_down.as_ref()
    }
}

/// Bradley (1978) trapezoidal-weir submergence curve (HEC-RAS Fig. 5-8): percent submergence → flow factor.
const BRADLEY_SUBMERGENCE_PCT: [f64; 12] =
    [0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 95.0, 98.0];
const BRADLEY_FLOW_FACTOR: [f64; 12] =
    [1.0, 1.0, 0.99, 0.97, 0.94, 0.90, 0.84, 0.75, 0.62, 0.40, 0.22, 0.08];

#[derive(Debug, Copy, Clone)]
struct ObstructedHydraulics {
    a_eff: f64,
    area_moment: f64,
    top_width: f64,
}

fn lookup_row(
    table: &GeometryTable,
    xs: Option<&CrossSection>,
    ineffective: Option<&IneffectiveFlowAreas>,
    wsel: f64,
) -> GeometryRow {
    if let Some(xs) = xs {
        row_at_elevation(table, xs, wsel, ineffective)
    } else {
        let row = table.interpolate(wsel);
        GeometryRow {
            active_area: row.area,
            active_channel_area: row.channel_area,
            ..row
        }
    }
}

fn base_flow_area(row: &GeometryRow, ineffective: Option<&IneffectiveFlowAreas>) -> f64 {
    let has_ineffective = ineffective.filter(|i| i.is_configured()).is_some();
    if row.channel_area > 1e-6 {
        if has_ineffective {
            row.active_channel_area
        } else {
            row.channel_area
        }
    } else if has_ineffective {
        row.active_area
    } else {
        row.area
    }
}

fn section_xs<'a>(geom: &'a BridgeGeometry, is_upstream: bool) -> Option<&'a CrossSection> {
    if is_upstream {
        geom.xs_up.as_ref()
    } else {
        geom.xs_down.as_ref()
    }
}

fn opening_station_bounds_from_deck(deck: Option<&BridgeDeckProfile>) -> (f64, f64) {
    if let Some(deck) = deck.filter(|d| d.is_valid()) {
        return (
            deck.stations_m.first().copied().unwrap_or(0.0),
            deck.stations_m.last().copied().unwrap_or(0.0),
        );
    }
    (0.0, 10.0)
}

fn opening_station_bounds_m(geom: &BridgeGeometry) -> (f64, f64) {
    opening_station_bounds_from_deck(geom.deck.as_ref())
}

fn gross_projected_opening_width_m(geom: &BridgeGeometry) -> f64 {
    let (s0, s1) = opening_station_bounds_m(geom);
    (s1 - s0).max(0.0) * geom.skew_cos
}

fn effective_pier_width_m(geom: &BridgeGeometry) -> f64 {
    geom.pier_width_m / geom.skew_cos
}

fn resolved_pier_stations_m(geom: &BridgeGeometry) -> Vec<f64> {
    if !geom.pier_stations_m.is_empty() {
        return geom.pier_stations_m.clone();
    }
    let n = geom.num_piers.max(0);
    if n == 0 {
        return vec![];
    }
    let (s_min, s_max) = opening_station_bounds_m(geom);
    let span = (s_max - s_min).max(1e-3);
    let w = effective_pier_width_m(geom);
    let inset = w * 0.5;
    let usable = (span - 2.0 * inset).max(w);
    (0..n)
        .map(|i| s_min + inset + usable * (i as f64 + 1.0) / (n as f64 + 1.0))
        .collect()
}

fn pier_in_opening_span(geom: &BridgeGeometry, station: f64) -> bool {
    let (s_min, s_max) = opening_station_bounds_m(geom);
    let half = effective_pier_width_m(geom) * 0.5;
    station + half > s_min && station - half < s_max
}

fn active_pier_count_in_opening(geom: &BridgeGeometry) -> usize {
    resolved_pier_stations_m(geom)
        .iter()
        .filter(|&&s| pier_in_opening_span(geom, s))
        .count()
}

fn total_pier_flow_width_m(geom: &BridgeGeometry) -> f64 {
    active_pier_count_in_opening(geom) as f64 * effective_pier_width_m(geom)
}

fn pier_submerged_area_geom(geom: &BridgeGeometry, depth: f64) -> f64 {
    total_pier_flow_width_m(geom) * depth.max(0.0)
}

/// Downstream flow area for Yarnell: base area minus per-side abutments, before pier blockage.
fn yarnell_downstream_flow_area_m2(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
) -> f64 {
    let depth = (wsel - z_bed).max(0.0);
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, false);
    (props.a_eff + pier_submerged_area_geom(geom, depth)).max(1e-5)
}

fn obstructed_hydraulics(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> ObstructedHydraulics {
    let ineffective = ineffective_for_side(geom, is_upstream);
    let row = lookup_row(
        table,
        section_xs(geom, is_upstream),
        ineffective,
        wsel,
    );
    let a_base = base_flow_area(&row, ineffective);
    let depth = (wsel - z_bed).max(0.0);
    let a_piers = pier_submerged_area_geom(geom, depth);
    let a_abut = geom.abutments.submerged_area_m2(wsel, z_bed);
    let a_eff = (a_base - a_piers - a_abut).max(1e-5);

    let full_moment = table.calculate_area_moment(wsel);
    let area_moment = if a_base > 1e-5 {
        full_moment * (a_eff / a_base)
    } else {
        a_eff * depth * 0.5
    };

    let t_base = if row.channel_area > 1e-6 {
        row.top_width.min(a_base / depth.max(1e-3))
    } else {
        row.top_width
    };
    let abut_width_at_wsel = geom.abutments.submerged_width_at_wsel_m(wsel, z_bed);
    let top_width = (t_base - total_pier_flow_width_m(geom) - abut_width_at_wsel).max(1e-3);

    ObstructedHydraulics {
        a_eff,
        area_moment,
        top_width,
    }
}

fn specific_force(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    q: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> f64 {
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, is_upstream);
    if props.a_eff < 1e-6 {
        return f64::INFINITY;
    }
    (q * q) / (G_METRIC * props.a_eff) + props.area_moment
}

fn obstructed_conveyance(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> f64 {
    let ineffective = ineffective_for_side(geom, is_upstream);
    let row = lookup_row(table, section_xs(geom, is_upstream), ineffective, wsel);
    let a_base = base_flow_area(&row, ineffective);
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, is_upstream);
    if a_base > 1e-6 {
        row.conveyance * (props.a_eff / a_base)
    } else {
        0.0
    }
}

fn velocity_head(q: f64, area: f64) -> f64 {
    if area < 1e-6 {
        return 0.0;
    }
    (q * q) / (2.0 * G_METRIC * area * area)
}

fn friction_loss(q: f64, k1: f64, k2: f64, length: f64) -> f64 {
    let k_avg = 0.5 * (k1 + k2).max(1e-6);
    length * (q / k_avg).powi(2)
}

/// WSPRO idealized contraction loss (HEC-RAS eq. 10) for approach area A1 and bridge opening A2.
fn wspro_contraction_loss(q: f64, a_approach: f64, a_bridge: f64, c: f64) -> f64 {
    if a_approach < 1e-6 || a_bridge < 1e-6 || c < 1e-6 {
        return 0.0;
    }
    let ratio = a_approach / a_bridge;
    let alpha_2 = 1.0 / (c * c);
    let beta_2 = 1.0 / c;
    let alpha_1 = 1.0;
    let beta_1 = 1.0;
    (q * q) / (2.0 * G_METRIC * a_approach.powi(2))
        * (2.0 * beta_1 - alpha_1 - 2.0 * beta_2 * ratio + alpha_2 * ratio * ratio)
}

fn solve_low_flow_energy_or_wspro(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
    use_wspro: bool,
) -> f64 {
    let length = if geom.length_m > 1e-3 {
        geom.length_m
    } else {
        10.0
    };
    let props_down = obstructed_hydraulics(table_down, tw_m, geom.z_down_m, geom, false);
    if props_down.a_eff < 1e-6 {
        return tw_m;
    }
    let k_down = obstructed_conveyance(table_down, tw_m, geom.z_down_m, geom, false);
    let e_down = tw_m + velocity_head(q_metric, props_down.a_eff);

    let residual = |wsel_up: f64| -> f64 {
        let props_up = obstructed_hydraulics(table_up, wsel_up, geom.z_up_m, geom, true);
        if props_up.a_eff < 1e-6 {
            return 1e6;
        }
        let k_up = obstructed_conveyance(table_up, wsel_up, geom.z_up_m, geom, true);
        let e_up = wsel_up + velocity_head(q_metric, props_up.a_eff);
        let hf = friction_loss(q_metric, k_down, k_up, length);
        let h_other = if use_wspro {
            let opening_wsel = wsel_up.min(tw_m).min(geom.low_chord_m);
            let props_opening =
                obstructed_hydraulics(table_up, opening_wsel, geom.z_up_m, geom, true);
            wspro_contraction_loss(
                q_metric,
                props_up.a_eff,
                props_opening.a_eff.max(1e-5),
                geom.wspro_coeff_c,
            )
            .max(0.0)
        } else {
            let hv_up = velocity_head(q_metric, props_up.a_eff);
            let hv_down = velocity_head(q_metric, props_down.a_eff);
            if hv_up > hv_down {
                geom.coeff_contraction * (hv_up - hv_down)
            } else {
                geom.coeff_expansion * (hv_down - hv_up)
            }
        };
        e_up - e_down - hf - h_other
    };

    let mut low = tw_m;
    let mut high = geom.low_chord_m;
    let mut best = low;
    let res_low = residual(low);
    let mut res_high = residual(high);
    if res_low * res_high > 0.0 {
        high = geom.low_chord_m + 20.0;
        res_high = residual(high);
    }
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let res = residual(mid);
        if res.abs() < 1e-6 {
            return mid;
        }
        if res_low * res_high <= 0.0 {
            if res < 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        } else if res < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

fn auto_class_a_method(geom: &BridgeGeometry) -> LowFlowMethod {
    if geom.num_piers > 0 {
        LowFlowMethod::Yarnell
    } else if geom.abutments.is_configured() {
        LowFlowMethod::Wspro
    } else {
        LowFlowMethod::Energy
    }
}

fn pier_drag_momentum_with_table(
    table: &GeometryTable,
    q: f64,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> f64 {
    let depth = (wsel - z_bed).max(0.0);
    let a_pier = pier_submerged_area_geom(geom, depth);
    if a_pier <= 1e-6 {
        return 0.0;
    }
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, is_upstream);
    let v = q / props.a_eff.max(1e-5);
    let y_pier = depth * 0.5;
    let cd = geom.pier_shape.drag_coefficient();
    a_pier * y_pier + 0.5 * cd * a_pier * (v * v) / G_METRIC
}

fn solve_critical_depth_obstructed(
    table: &GeometryTable,
    z_bed: f64,
    q: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> f64 {
    if table.rows.is_empty() || q <= 1e-5 {
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
        let props = obstructed_hydraulics(table, elev, z_bed, geom, is_upstream);
        if props.a_eff < 1e-6 {
            low = mid;
            continue;
        }
        let fr_sq = (q * q * props.top_width) / (G_METRIC * props.a_eff.powi(3));
        let f_val = 1.0 - fr_sq;
        if f_val.abs() < 1e-6 {
            best_yc = mid;
            break;
        }
        if f_val < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best_yc = mid;
    }
    best_yc
}

fn critical_specific_force(
    table: &GeometryTable,
    z_bed: f64,
    q: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> (f64, f64) {
    let yc = solve_critical_depth_obstructed(table, z_bed, q, geom, is_upstream);
    let wsel_crit = z_bed + yc;
    (wsel_crit, specific_force(table, wsel_crit, z_bed, q, geom, is_upstream))
}

/// Classify low flow per HEC-RAS: compare downstream momentum to critical momentum in the bridge.
pub fn classify_low_flow(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> LowFlowClass {
    let (_, m_crit_up) = critical_specific_force(table_up, geom.z_up_m, q_metric, geom, true);
    let (_, m_crit_down) = critical_specific_force(table_down, geom.z_down_m, q_metric, geom, false);
    let (wsel_crit, m_crit) = if m_crit_up >= m_crit_down {
        (
            geom.z_up_m + solve_critical_depth_obstructed(table_up, geom.z_up_m, q_metric, geom, true),
            m_crit_up,
        )
    } else {
        (
            geom.z_down_m
                + solve_critical_depth_obstructed(table_down, geom.z_down_m, q_metric, geom, false),
            m_crit_down,
        )
    };

    let m_down = specific_force(table_down, tw_m, geom.z_down_m, q_metric, geom, false);
    let props = obstructed_hydraulics(table_down, tw_m, geom.z_down_m, geom, false);
    let v_down = if props.a_eff > 1e-5 {
        q_metric / props.a_eff
    } else {
        0.0
    };
    let fr_down = if props.a_eff > 1e-5 {
        v_down / (G_METRIC * props.a_eff / props.top_width.max(1e-3)).sqrt()
    } else {
        0.0
    };

    let _ = wsel_crit;

    if fr_down >= 1.0 && tw_m < geom.low_chord_m {
        LowFlowClass::C
    } else if m_down < m_crit {
        LowFlowClass::B
    } else {
        LowFlowClass::A
    }
}

/// HEC-RAS Yarnell low-flow pier head loss (Class A): drop from section 3 to section 2.
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
    let k = pier_shape.yarnell_coefficient();

    2.0 * k * (k + 10.0 * omega - 0.6) * (alpha + 15.0 * alpha.powi(4)) * velocity_head
}

fn net_opening_area_at_low_chord(geom: &BridgeGeometry, table: &GeometryTable) -> f64 {
    let factor = profile_opening_area_factor(geom);
    // Cross-section openings: same per-side abutment/pier obstruction as low-flow solvers.
    if geom.xs_up.is_some() {
        return obstructed_hydraulics(table, geom.low_chord_m, geom.z_up_m, geom, true)
            .a_eff
            .max(1e-4)
            * factor;
    }
    let height_under_deck = (geom.low_chord_m - geom.z_up_m).max(0.0);
    let deck_width = gross_projected_opening_width_m(geom);
    let a_gross = if deck_width > 1e-6 {
        deck_width * height_under_deck
    } else {
        let row = lookup_row(
            table,
            geom.xs_up.as_ref(),
            ineffective_for_side(geom, true),
            geom.low_chord_m,
        );
        base_flow_area(&row, ineffective_for_side(geom, true))
    };
    let a_piers = pier_submerged_area_geom(geom, height_under_deck);
    let a_abut = geom.abutments.submerged_area_m2(geom.low_chord_m, geom.z_up_m);
    (a_gross - a_piers - a_abut).max(1e-4) * factor
}

fn upstream_energy_grade(
    wsel: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table: &GeometryTable,
    z_bed: f64,
    is_upstream: bool,
) -> f64 {
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, is_upstream);
    wsel + velocity_head(q_metric, props.a_eff)
}

/// HEC-RAS sluice-gate Cd vs Y3/Z (Fig. 5-5); user coefficient overrides when > 0.
pub(crate) fn sluice_gate_discharge_coeff(y3_over_z: f64, user_coeff: f64) -> f64 {
    if user_coeff > 1e-6 {
        return user_coeff;
    }
    let r = y3_over_z.clamp(0.0, 1.0);
    0.27 + 0.23 * r
}

pub(crate) fn bradley_weir_submergence_factor(submergence_ratio: f64) -> f64 {
    if submergence_ratio <= 0.0 {
        return 1.0;
    }
    let pct = (submergence_ratio * 100.0).clamp(0.0, 98.0);
    for i in 1..BRADLEY_SUBMERGENCE_PCT.len() {
        if pct <= BRADLEY_SUBMERGENCE_PCT[i] {
            let t = (pct - BRADLEY_SUBMERGENCE_PCT[i - 1])
                / (BRADLEY_SUBMERGENCE_PCT[i] - BRADLEY_SUBMERGENCE_PCT[i - 1]);
            return BRADLEY_FLOW_FACTOR[i - 1]
                + t * (BRADLEY_FLOW_FACTOR[i] - BRADLEY_FLOW_FACTOR[i - 1]);
        }
    }
    BRADLEY_FLOW_FACTOR[BRADLEY_FLOW_FACTOR.len() - 1]
}

fn weir_submergence_ratio(tw_m: f64, e_upstream: f64, crest_m: f64) -> f64 {
    let tail_above = (tw_m - crest_m).max(0.0);
    let head_above = (e_upstream - crest_m).max(1e-6);
    (tail_above / head_above).clamp(0.0, 1.5)
}

fn pressure_flow_discharge(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    a_net: f64,
) -> f64 {
    if wsel_up <= geom.low_chord_m + 1e-6 {
        return 0.0;
    }

    if tw_m >= geom.low_chord_m {
        let e_up = upstream_energy_grade(wsel_up, q_metric, geom, table_up, geom.z_up_m, true);
        let head = (e_up - tw_m).max(0.0);
        geom.pressure_coeff_submerged * a_net * (2.0 * G_METRIC * head).sqrt()
    } else {
        let z = (geom.low_chord_m - geom.z_up_m).max(1e-3);
        let y3 = (wsel_up - geom.z_up_m).max(1e-3);
        let cd = sluice_gate_discharge_coeff(y3 / z, geom.pressure_coeff_inlet);
        let props = obstructed_hydraulics(table_up, wsel_up, geom.z_up_m, geom, true);
        let v_head = velocity_head(q_metric, props.a_eff);
        let drive = (y3 - 0.5 * z + v_head).max(0.0);
        cd * a_net * (2.0 * G_METRIC * drive).sqrt()
    }
}

fn weir_flow_discharge(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    l_weir: f64,
) -> f64 {
    let e_up = upstream_energy_grade(wsel_up, q_metric, geom, table_up, geom.z_up_m, true);
    let h_weir = (e_up - geom.high_chord_m).max(0.0);
    if h_weir <= 1e-6 {
        return 0.0;
    }
    let sub_ratio = weir_submergence_ratio(tw_m, e_up, geom.high_chord_m);
    let factor = bradley_weir_submergence_factor(sub_ratio);
    geom.weir_coeff_m * factor * l_weir * h_weir.powf(1.5)
}

fn solve_pressure_headwater(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
) -> f64 {
    let a_net = net_opening_area_at_low_chord(geom, table_up);
    let mut low = tw_m.max(geom.z_up_m + 1e-4);
    let mut high = geom.low_chord_m + 30.0;
    let mut best = geom.low_chord_m;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let q_calc = pressure_flow_discharge(mid, tw_m, q_metric, geom, table_up, a_net);
        if (q_calc - q_metric).abs() < 1e-6 {
            return mid;
        }
        if q_calc < q_metric {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

fn apply_low_flow_pressure_check(
    q_metric: f64,
    tw_m: f64,
    wsel_low: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let egl = upstream_energy_grade(wsel_low, q_metric, geom, table_up, geom.z_up_m, true);
    if egl <= geom.low_chord_max_m {
        return wsel_low;
    }
    if geom.high_flow_method == HighFlowMethod::Energy {
        return solve_high_flow_energy(q_metric, tw_m, geom, table_up, table_down);
    }
    let pressure_hw = solve_pressure_headwater(q_metric, tw_m, geom, table_up);
    if pressure_hw > wsel_low {
        pressure_hw
    } else {
        wsel_low
    }
}

fn high_flow_energy_uses_wspro(geom: &BridgeGeometry) -> bool {
    matches!(geom.low_flow_method, LowFlowMethod::Wspro)
        || (geom.low_flow_method == LowFlowMethod::Auto && geom.abutments.is_configured())
}

/// Energy balance through the obstructed opening (HEC-RAS high-flow energy method).
fn solve_high_flow_energy(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    solve_low_flow_energy_or_wspro(
        q_metric,
        tw_m,
        geom,
        table_up,
        table_down,
        high_flow_energy_uses_wspro(geom),
    )
}

fn solve_high_flow_energy_fallback(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    solve_high_flow_energy(q_metric, tw_m, geom, table_up, table_down)
}

fn solve_low_flow_class_a(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let method = match geom.low_flow_method {
        LowFlowMethod::Auto => auto_class_a_method(geom),
        other => other,
    };

    match method {
        LowFlowMethod::Energy => {
            return solve_low_flow_energy_or_wspro(
                q_metric, tw_m, geom, table_up, table_down, false,
            );
        }
        LowFlowMethod::Wspro => {
            return solve_low_flow_energy_or_wspro(
                q_metric, tw_m, geom, table_up, table_down, true,
            );
        }
        LowFlowMethod::Momentum => {}
        LowFlowMethod::Yarnell | LowFlowMethod::Auto => {}
    }

    let use_yarnell = matches!(method, LowFlowMethod::Yarnell) && geom.num_piers > 0;

    if use_yarnell {
        let flow_area_net = yarnell_downstream_flow_area_m2(table_down, tw_m, geom.z_down_m, geom);

        if flow_area_net > 1e-5 && q_metric > 1e-5 {
            let hl = yarnell_pier_head_loss(
                q_metric,
                tw_m,
                geom.z_down_m,
                geom.pier_width_m,
                geom.num_piers,
                geom.pier_shape,
                flow_area_net,
            );
            return tw_m + hl;
        }
    }

    // Momentum (general) Class A: upstream specific force = downstream + pier drag
    let m_down = specific_force(table_down, tw_m, geom.z_down_m, q_metric, geom, false);
    let drag = pier_drag_momentum_with_table(table_up, q_metric, tw_m, geom.z_up_m, geom, true);
    let target = m_down + drag;

    let mut low = tw_m;
    let mut high = geom.low_chord_m;
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let m_up = specific_force(table_up, mid, geom.z_up_m, q_metric, geom, true);
        if (m_up - target).abs() < 1e-6 {
            return mid;
        }
        if m_up < target {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

fn solve_low_flow_class_b(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let use_energy = matches!(
        geom.low_flow_method,
        LowFlowMethod::Energy | LowFlowMethod::Wspro
    );

    if !use_energy {
        let (_, m_crit_up) = critical_specific_force(table_up, geom.z_up_m, q_metric, geom, true);
        let (_, m_crit_down) = critical_specific_force(table_down, geom.z_down_m, q_metric, geom, false);
        let m_crit = m_crit_up.max(m_crit_down);
        let z_control = if m_crit_up >= m_crit_down {
            geom.z_up_m
        } else {
            geom.z_down_m
        };
        let table_control = if m_crit_up >= m_crit_down {
            table_up
        } else {
            table_down
        };
        let is_upstream = m_crit_up >= m_crit_down;
        let yc = solve_critical_depth_obstructed(table_control, z_control, q_metric, geom, is_upstream);
        let wsel_min = z_control + yc;

        let mut low = wsel_min;
        let mut high = geom.low_chord_m;
        let mut best = low;
        for _ in 0..50 {
            let mid = 0.5 * (low + high);
            let drag = pier_drag_momentum_with_table(table_up, q_metric, mid, geom.z_up_m, geom, true);
            let m_up = specific_force(table_up, mid, geom.z_up_m, q_metric, geom, true);
            let residual = m_up - drag - m_crit;
            if residual.abs() < 1e-5 {
                return mid;
            }
            if residual < 0.0 {
                low = mid;
            } else {
                high = mid;
            }
            best = mid;
        }
        if best > tw_m {
            return best;
        }
    }

    // HEC-RAS Class B energy fallback when momentum fails or energy/WSPRO is selected.
    let use_wspro = matches!(geom.low_flow_method, LowFlowMethod::Wspro)
        || (geom.low_flow_method == LowFlowMethod::Auto && geom.abutments.is_configured());
    solve_low_flow_energy_or_wspro(q_metric, tw_m, geom, table_up, table_down, use_wspro)
}

fn solve_low_flow_class_c(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let m_down = specific_force(table_down, tw_m, geom.z_down_m, q_metric, geom, false);
    let mut low = tw_m;
    let mut high = geom.low_chord_m;
    let mut best = high;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let drag = pier_drag_momentum_with_table(table_up, q_metric, mid, geom.z_up_m, geom, true);
        let m_up = specific_force(table_up, mid, geom.z_up_m, q_metric, geom, true);
        let residual = (m_up - drag) - m_down;
        if residual.abs() < 1e-5 {
            return mid;
        }
        if residual < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

fn solve_low_flow(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let class = classify_low_flow(q_metric, tw_m, geom, table_up, table_down);
    let wsel_up = match class {
        LowFlowClass::A => solve_low_flow_class_a(q_metric, tw_m, geom, table_up, table_down),
        LowFlowClass::B => solve_low_flow_class_b(q_metric, tw_m, geom, table_up, table_down),
        LowFlowClass::C => solve_low_flow_class_c(q_metric, tw_m, geom, table_up, table_down),
    };
    if wsel_up < geom.low_chord_m {
        apply_low_flow_pressure_check(q_metric, tw_m, wsel_up, geom, table_up, table_down)
    } else {
        solve_high_flow(q_metric, geom, tw_m, table_up, table_down)
    }
}

/// HEC-RAS high-flow headwater: pressure/weir (default) or explicit energy method.
fn solve_high_flow(
    q_metric: f64,
    geom: &BridgeGeometry,
    tw_clamped: f64,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    if geom.high_flow_method == HighFlowMethod::Energy {
        return solve_high_flow_energy(q_metric, tw_clamped, geom, table_up, table_down);
    }

    let a_net = net_opening_area_at_low_chord(geom, table_up);
    let pressure_only = solve_pressure_headwater(q_metric, tw_clamped, geom, table_up);

    if pressure_only < geom.high_chord_m {
        let e_up = upstream_energy_grade(pressure_only, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_ratio(tw_clamped, e_up, geom.high_chord_m) >= geom.max_weir_submergence {
            return solve_high_flow_energy_fallback(
                q_metric,
                tw_clamped,
                geom,
                table_up,
                table_down,
            );
        }
        return pressure_only;
    }

    let fallback_weir_width = table_up.interpolate(geom.high_chord_m).top_width.max(1.0);

    let residual = |h_up: f64| -> f64 {
        let e_up = upstream_energy_grade(h_up, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_ratio(tw_clamped, e_up, geom.high_chord_m) >= geom.max_weir_submergence
        {
            return -1.0;
        }
        let l_weir = effective_weir_length_m(geom, e_up, fallback_weir_width);
        let q_pressure = pressure_flow_discharge(h_up, tw_clamped, q_metric, geom, table_up, a_net);
        let q_weir = weir_flow_discharge(h_up, tw_clamped, q_metric, geom, table_up, l_weir);
        (q_pressure + q_weir) - q_metric
    };

    let mut low = geom.high_chord_m;
    let mut high = geom.high_chord_m + 50.0;
    let mut best_h = low;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let e_up = upstream_energy_grade(mid, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_ratio(tw_clamped, e_up, geom.high_chord_m) >= geom.max_weir_submergence
        {
            return solve_high_flow_energy_fallback(
                q_metric,
                tw_clamped,
                geom,
                table_up,
                table_down,
            );
        }
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

/// Result of a bridge headwater–tailwater coupling solve (steady or unsteady post-step).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeSolveResult {
    pub wsel_up: f64,
    pub wsel_down: f64,
    pub head_loss: f64,
    /// `low_a`, `low_b`, `low_c`, `pressure`, `weir`, or `energy`
    pub flow_regime: String,
}

fn bridge_flow_regime_label(
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

/// Couples upstream WSEL to downstream tailwater for inline bridge routing.
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
    let wsel_up = solve_bridge_wsel(
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
    );
    let flow_regime = bridge_flow_regime_label(
        tw_wsel,
        wsel_up,
        low_chord,
        high_chord,
        units,
        q,
        table_up,
        table_down,
        coupling,
        interval_length_m,
        pier_width,
        num_piers,
        pier_shape_type,
        weir_coeff,
        orifice_coeff,
        z_down,
        z_up,
    );
    BridgeSolveResult {
        wsel_up,
        wsel_down: tw_wsel,
        head_loss: (wsel_up - tw_wsel).max(0.0),
        flow_regime,
    }
}

fn default_weir_coeff_for_units(units: UnitSystem) -> f64 {
    if units == UnitSystem::USCustomary {
        2.6
    } else {
        1.44
    }
}

fn rectangular_channel_cross_section(
    width: f64,
    z_bed: f64,
    manning_n: f64,
    units: UnitSystem,
) -> CrossSection {
    CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, width, width],
        y: vec![z_bed + 10.0, z_bed, z_bed, z_bed + 10.0],
        n_stations: vec![0.0],
        n_values: vec![manning_n.max(0.01)],
        unit_system: units,
        is_overbank: None,
        blocked_obstructions: None,
    }
}

fn abutment_input_from_params(params: &BridgeSolveParams) -> BridgeAbutmentUserInput {
    BridgeAbutmentUserInput {
        legacy_total_width: params.abutment_block_width,
        left_width: params.abutment_left_width,
        right_width: params.abutment_right_width,
        left_station: params.abutment_left_station,
        right_station: params.abutment_right_station,
        left_top_elevation: params.abutment_left_top_elevation,
        right_top_elevation: params.abutment_right_top_elevation,
        left_top_profile_stations: params.abutment_left_top_profile_stations.clone(),
        left_top_profile_elevations: params.abutment_left_top_profile_elevations.clone(),
        right_top_profile_stations: params.abutment_right_top_profile_stations.clone(),
        right_top_profile_elevations: params.abutment_right_top_profile_elevations.clone(),
    }
}

fn coupling_from_params(params: &BridgeSolveParams) -> BridgeCouplingParams {
    BridgeCouplingParams {
        abutment: abutment_input_from_params(params),
        low_flow_method: params.low_flow_method,
        high_flow_method: params.high_flow_method,
        length: params.length,
        wspro_coeff: params.wspro_coeff,
        coeff_contraction: params.coeff_contraction,
        coeff_expansion: params.coeff_expansion,
        pressure_coeff_inlet: params.pressure_coeff_inlet,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: params.max_weir_submergence,
    }
}

fn vec_or_scalar(values: Option<&Vec<f64>>, scalar: Option<f64>) -> Vec<f64> {
    if let Some(v) = values {
        if !v.is_empty() {
            return v.clone();
        }
    }
    scalar.into_iter().collect()
}

fn ineffective_face_blocks(
    face_stations: Option<&Vec<f64>>,
    face_elevations: Option<&Vec<f64>>,
    face_station: Option<f64>,
    face_elevation: Option<f64>,
    legacy_stations: Option<&Vec<f64>>,
    legacy_elevations: Option<&Vec<f64>>,
    legacy_station: Option<f64>,
    legacy_elevation: Option<f64>,
) -> (Vec<f64>, Vec<f64>) {
    let stations = {
        let us = vec_or_scalar(face_stations, face_station);
        if !us.is_empty() {
            us
        } else {
            vec_or_scalar(legacy_stations, legacy_station)
        }
    };
    let elevations = {
        let ue = vec_or_scalar(face_elevations, face_elevation);
        if !ue.is_empty() {
            ue
        } else {
            vec_or_scalar(legacy_elevations, legacy_elevation)
        }
    };
    (stations, elevations)
}

fn ineffective_upstream_from_params(params: &BridgeSolveParams) -> Option<IneffectiveFlowAreas> {
    let (left_s, left_e) = ineffective_face_blocks(
        params.ineffective_left_stations_upstream.as_ref(),
        params.ineffective_left_elevations_upstream.as_ref(),
        params.ineffective_left_station_upstream,
        params.ineffective_left_elevation_upstream,
        params.ineffective_left_stations.as_ref(),
        params.ineffective_left_elevations.as_ref(),
        params.ineffective_left_station,
        params.ineffective_left_elevation,
    );
    let (right_s, right_e) = ineffective_face_blocks(
        params.ineffective_right_stations_upstream.as_ref(),
        params.ineffective_right_elevations_upstream.as_ref(),
        params.ineffective_right_station_upstream,
        params.ineffective_right_elevation_upstream,
        params.ineffective_right_stations.as_ref(),
        params.ineffective_right_elevations.as_ref(),
        params.ineffective_right_station,
        params.ineffective_right_elevation,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

fn ineffective_downstream_from_params(params: &BridgeSolveParams) -> Option<IneffectiveFlowAreas> {
    let (left_s, left_e) = ineffective_face_blocks(
        params.ineffective_left_stations_downstream.as_ref(),
        params.ineffective_left_elevations_downstream.as_ref(),
        params.ineffective_left_station_downstream,
        params.ineffective_left_elevation_downstream,
        params.ineffective_left_stations.as_ref(),
        params.ineffective_left_elevations.as_ref(),
        params.ineffective_left_station,
        params.ineffective_left_elevation,
    );
    let (right_s, right_e) = ineffective_face_blocks(
        params.ineffective_right_stations_downstream.as_ref(),
        params.ineffective_right_elevations_downstream.as_ref(),
        params.ineffective_right_station_downstream,
        params.ineffective_right_elevation_downstream,
        params.ineffective_right_stations.as_ref(),
        params.ineffective_right_elevations.as_ref(),
        params.ineffective_right_station,
        params.ineffective_right_elevation,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

fn interval_length_metric(params: &BridgeSolveParams) -> f64 {
    let len = if params.length > 1e-6 {
        params.length
    } else if params.units == UnitSystem::USCustomary {
        50.0
    } else {
        15.0
    };
    if params.units == UnitSystem::USCustomary {
        len * FT_TO_M
    } else {
        len
    }
}

fn geometry_tables_from_params(
    params: &BridgeSolveParams,
) -> (GeometryTable, GeometryTable, CrossSection, CrossSection) {
    let num_slices = params.num_slices.max(2);
    let manning_n = if params.manning_n > 1e-6 {
        params.manning_n
    } else {
        0.03
    };

    let (xs_up, xs_down) = if params.xs_up.is_some() || params.xs_down.is_some() {
        let up = params
            .xs_up
            .clone()
            .unwrap_or_else(|| {
                rectangular_channel_cross_section(
                    params.channel_width.max(1e-3),
                    params.z_up,
                    manning_n,
                    params.units,
                )
            });
        let down = params
            .xs_down
            .clone()
            .unwrap_or_else(|| {
                rectangular_channel_cross_section(
                    params.channel_width.max(1e-3),
                    params.z_down,
                    manning_n,
                    params.units,
                )
            });
        (up, down)
    } else {
        (
            rectangular_channel_cross_section(
                params.channel_width.max(1e-3),
                params.z_up,
                manning_n,
                params.units,
            ),
            rectangular_channel_cross_section(
                params.channel_width.max(1e-3),
                params.z_down,
                manning_n,
                params.units,
            ),
        )
    };

    let up_metric = xs_up.to_metric();
    let down_metric = xs_down.to_metric();
    (
        up_metric.generate_lookup_table(num_slices),
        down_metric.generate_lookup_table(num_slices),
        xs_up,
        xs_down,
    )
}

/// Solves upstream headwater from fixed tailwater using [`BridgeSolveParams`].
pub fn solve_bridge_from_params(params: &BridgeSolveParams) -> BridgeSolveResult {
    let (table_up, table_down, xs_up, xs_down) = geometry_tables_from_params(params);
    let coupling = coupling_from_params(params);
    let deck = build_bridge_deck_profile(
        params.low_chord,
        params.high_chord,
        params.deck_stations.as_deref(),
        params.deck_low_elevations.as_deref(),
        params.deck_high_elevations.as_deref(),
        params.units,
    );
    let sections = BridgeSectionContext {
        ineffective_up: ineffective_upstream_from_params(params),
        ineffective_down: ineffective_downstream_from_params(params),
        xs_up: Some(xs_up),
        xs_down: Some(xs_down),
        skew_deg: params.skew_deg,
        pier_stations: params.pier_stations.clone(),
    };
    let weir_coeff = if params.weir_coeff > 1e-6 {
        params.weir_coeff
    } else {
        default_weir_coeff_for_units(params.units)
    };
    let orifice_coeff = if params.orifice_coeff > 1e-6 {
        params.orifice_coeff
    } else {
        0.5
    };

    solve_bridge_coupled(
        params.q,
        params.low_chord,
        params.high_chord,
        params.pier_width,
        params.num_piers,
        params.pier_shape_type,
        weir_coeff,
        orifice_coeff,
        params.z_down,
        params.z_up,
        params.tw_wsel,
        params.units,
        &table_up,
        &table_down,
        &coupling,
        interval_length_metric(params),
        deck.as_ref(),
        Some(&sections),
    )
}

/// Compute upstream headwater vs discharge at fixed tailwater (bridge rating curve).
pub fn compute_bridge_rating_curve(inputs: &BridgeRatingCurveInputs) -> BridgeRatingCurveResult {
    let mut q = Vec::with_capacity(inputs.q_values.len());
    let mut wsel = Vec::with_capacity(inputs.q_values.len());
    let mut wsel_down = Vec::with_capacity(inputs.q_values.len());
    let mut flow_regimes = Vec::with_capacity(inputs.q_values.len());
    let mut head_losses = Vec::with_capacity(inputs.q_values.len());

    for &q_sample in &inputs.q_values {
        let mut params = inputs.bridge.clone();
        params.q = q_sample;
        let result = solve_bridge_from_params(&params);
        q.push(q_sample);
        wsel.push(result.wsel_up);
        wsel_down.push(result.wsel_down);
        flow_regimes.push(result.flow_regime);
        head_losses.push(result.head_loss);
    }

    BridgeRatingCurveResult {
        q,
        wsel,
        wsel_down,
        flow_regimes,
        head_losses,
    }
}

/// Solves upstream WSEL from a known downstream tailwater (subcritical sweep).
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
        deck,
        sections,
    );

    let tw_m = if units == UnitSystem::USCustomary {
        tw_wsel * FT_TO_M
    } else {
        tw_wsel
    };
    let tw_clamped = tw_m.max(geom.z_down_m + 1e-4);
    let q_metric = if units == UnitSystem::USCustomary {
        q * CFS_TO_CMS
    } else {
        q
    };

    let wsel_up_metric = if tw_clamped < geom.low_chord_m {
        solve_low_flow(q_metric, tw_clamped, &geom, table_up, table_down)
    } else {
        solve_high_flow(q_metric, &geom, tw_clamped, table_up, table_down)
    };

    if units == UnitSystem::USCustomary {
        wsel_up_metric / FT_TO_M
    } else {
        wsel_up_metric
    }
}

/// Solves downstream tailwater from a known upstream headwater (supercritical sweep).
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
        deck,
        sections,
    );

    let hw_m = if units == UnitSystem::USCustomary {
        hw_wsel * FT_TO_M
    } else {
        hw_wsel
    };
    let q_metric = if units == UnitSystem::USCustomary {
        q * CFS_TO_CMS
    } else {
        q
    };

    let tw_metric = if hw_m >= geom.low_chord_m {
        solve_high_flow_tailwater(q_metric, &geom, hw_m, table_up, table_down)
    } else {
        solve_low_flow_tailwater(q_metric, hw_m, &geom, table_up, table_down)
    };

    if units == UnitSystem::USCustomary {
        tw_metric / FT_TO_M
    } else {
        tw_metric
    }
}

fn build_bridge_geometry(
    low_chord: f64,
    high_chord: f64,
    pier_width: f64,
    num_piers: i32,
    pier_shape_type: i32,
    weir_coeff: f64,
    orifice_coeff: f64,
    z_down: f64,
    z_up: f64,
    units: UnitSystem,
    coupling: &BridgeCouplingParams,
    interval_length: f64,
    deck: Option<&BridgeDeckProfile>,
    sections: Option<&BridgeSectionContext>,
) -> BridgeGeometry {
    let (low_min, low_max, high_min, high_max) = deck_extrema(low_chord, high_chord, deck, units);
    let length_base_m = if coupling.length > 1e-6 {
        if units == UnitSystem::USCustomary {
            coupling.length * FT_TO_M
        } else {
            coupling.length
        }
    } else if interval_length > 1e-6 {
        interval_length
    } else {
        10.0
    };
    let skew_deg = sections.map(|s| s.skew_deg).unwrap_or(0.0);
    let (_, length_m) = apply_bridge_skew(skew_deg, 1.0, length_base_m);
    let skew_cos = {
        let deg = skew_deg.clamp(0.0, 59.0);
        deg.to_radians().cos().max(0.52)
    };

    let pier_stations_m = sections
        .and_then(|s| s.pier_stations.as_ref())
        .map(|st| {
            st.iter()
                .map(|x| {
                    if units == UnitSystem::USCustomary {
                        x * FT_TO_M
                    } else {
                        *x
                    }
                })
                .collect::<Vec<f64>>()
        })
        .unwrap_or_default();
    let num_piers = if !pier_stations_m.is_empty() {
        pier_stations_m.len() as i32
    } else {
        num_piers
    };

    let submerged_c = if orifice_coeff > 1e-6 {
        orifice_coeff
    } else {
        coupling.pressure_coeff_submerged
    };

    let deck_owned = deck.cloned();
    let to_metric_ineffective = |i: &IneffectiveFlowAreas| {
        if units == UnitSystem::USCustomary {
            i.to_metric(UnitSystem::USCustomary)
        } else {
            i.clone()
        }
    };
    let ineffective_up = sections
        .and_then(|s| s.ineffective_up.as_ref())
        .filter(|i| i.is_configured())
        .map(to_metric_ineffective);
    let ineffective_down = sections
        .and_then(|s| s.ineffective_down.as_ref())
        .filter(|i| i.is_configured())
        .map(to_metric_ineffective);
    let xs_up = sections.and_then(|s| s.xs_up.clone()).map(|xs| {
        if units == UnitSystem::USCustomary {
            xs.to_metric()
        } else {
            xs
        }
    });
    let xs_down = sections.and_then(|s| s.xs_down.clone()).map(|xs| {
        if units == UnitSystem::USCustomary {
            xs.to_metric()
        } else {
            xs
        }
    });

    let (opening_s_min, opening_s_max) = opening_station_bounds_from_deck(deck);
    let abutments = resolve_abutments(&coupling.abutment, opening_s_min, opening_s_max, skew_cos, units);

    if units == UnitSystem::USCustomary {
        BridgeGeometry {
            low_chord_m: low_min,
            low_chord_max_m: low_max,
            high_chord_m: high_min,
            high_chord_max_m: high_max,
            pier_width_m: pier_width * FT_TO_M,
            num_piers,
            pier_stations_m: pier_stations_m.clone(),
            skew_deg,
            skew_cos,
            pier_shape: PierShape::from_i32(pier_shape_type),
            abutments: abutments.clone(),
            weir_coeff_m: weir_coeff / 1.8113,
            orifice_coeff: submerged_c,
            z_up_m: z_up * FT_TO_M,
            z_down_m: z_down * FT_TO_M,
            low_flow_method: LowFlowMethod::from_i32(coupling.low_flow_method),
            high_flow_method: HighFlowMethod::from_i32(coupling.high_flow_method),
            length_m,
            wspro_coeff_c: coupling.wspro_coeff,
            coeff_contraction: coupling.coeff_contraction,
            coeff_expansion: coupling.coeff_expansion,
            pressure_coeff_inlet: coupling.pressure_coeff_inlet,
            pressure_coeff_submerged: submerged_c,
            max_weir_submergence: coupling.max_weir_submergence,
            deck: deck_owned,
            ineffective_up: ineffective_up.clone(),
            ineffective_down: ineffective_down.clone(),
            xs_up: xs_up.clone(),
            xs_down: xs_down.clone(),
        }
    } else {
        BridgeGeometry {
            low_chord_m: low_min,
            low_chord_max_m: low_max,
            high_chord_m: high_min,
            high_chord_max_m: high_max,
            pier_width_m: pier_width,
            num_piers,
            pier_stations_m,
            skew_deg,
            skew_cos,
            pier_shape: PierShape::from_i32(pier_shape_type),
            abutments,
            weir_coeff_m: weir_coeff,
            orifice_coeff: submerged_c,
            z_up_m: z_up,
            z_down_m: z_down,
            low_flow_method: LowFlowMethod::from_i32(coupling.low_flow_method),
            high_flow_method: HighFlowMethod::from_i32(coupling.high_flow_method),
            length_m,
            wspro_coeff_c: coupling.wspro_coeff,
            coeff_contraction: coupling.coeff_contraction,
            coeff_expansion: coupling.coeff_expansion,
            pressure_coeff_inlet: coupling.pressure_coeff_inlet,
            pressure_coeff_submerged: submerged_c,
            max_weir_submergence: coupling.max_weir_submergence,
            deck: deck_owned,
            ineffective_up,
            ineffective_down,
            xs_up,
            xs_down,
        }
    }
}

fn solve_low_flow_tailwater(
    q_metric: f64,
    hw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    // Invert subcritical low-flow solvers via bisection on tailwater.
    let mut low = geom.z_down_m + 1e-4;
    let mut high = hw_m.min(geom.low_chord_m);
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let hw_calc = solve_low_flow(q_metric, mid, geom, table_up, table_down);
        if (hw_calc - hw_m).abs() < 1e-4 {
            return mid;
        }
        if hw_calc < hw_m {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

fn solve_high_flow_energy_tailwater(
    q_metric: f64,
    hw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let mut low = geom.z_down_m + 1e-4;
    let mut high = hw_m.max(geom.low_chord_m);
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let hw_calc = solve_high_flow_energy(q_metric, mid, geom, table_up, table_down);
        if (hw_calc - hw_m).abs() < 1e-4 {
            return mid;
        }
        if hw_calc < hw_m {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

fn solve_high_flow_tailwater(
    q_metric: f64,
    geom: &BridgeGeometry,
    hw_m: f64,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    if geom.high_flow_method == HighFlowMethod::Energy {
        return solve_high_flow_energy_tailwater(q_metric, hw_m, geom, table_up, table_down);
    }

    let a_net = net_opening_area_at_low_chord(geom, table_down);

    if hw_m < geom.high_chord_m {
        let mut low = geom.z_down_m + 1e-4;
        let mut high = hw_m;
        let mut best = low;
        for _ in 0..50 {
            let mid = 0.5 * (low + high);
            let q_calc = pressure_flow_discharge(hw_m, mid, q_metric, geom, table_up, a_net);
            if (q_calc - q_metric).abs() < 1e-6 {
                return mid;
            }
            if q_calc < q_metric {
                high = mid;
            } else {
                low = mid;
            }
            best = mid;
        }
        return best;
    }

    let fallback_weir_width = table_down.interpolate(geom.high_chord_m).top_width.max(1.0);
    let residual = |tw: f64| -> f64 {
        let e_up = upstream_energy_grade(hw_m, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_ratio(tw, e_up, geom.high_chord_m) >= geom.max_weir_submergence {
            return -1.0;
        }
        let l_weir = effective_weir_length_m(geom, e_up, fallback_weir_width);
        let q_pressure = pressure_flow_discharge(hw_m, tw, q_metric, geom, table_up, a_net);
        let q_weir = weir_flow_discharge(hw_m, tw, q_metric, geom, table_up, l_weir);
        (q_pressure + q_weir) - q_metric
    };

    let mut low = geom.z_down_m + 1e-4;
    let mut high = hw_m;
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let e_up = upstream_energy_grade(hw_m, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_ratio(mid, e_up, geom.high_chord_m) >= geom.max_weir_submergence {
            return solve_high_flow_energy_fallback(q_metric, mid, geom, table_up, table_down);
        }
        let res = residual(mid);
        if res.abs() < 1e-8 {
            return mid;
        }
        if res > 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::CrossSection;

    fn rectangular_table(width: f64, z_bed: f64, num_slices: usize) -> GeometryTable {
        let xs = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, width, width],
            y: vec![z_bed + 10.0, z_bed, z_bed, z_bed + 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        };
        xs.generate_lookup_table(num_slices)
    }

    #[test]
    fn test_yarnell_pier_head_loss_hec_ras() {
        let hl = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::Square, 30.0);
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
        assert!(square > semi);
    }

    #[test]
    fn test_classify_low_flow_subcritical_is_class_a() {
        let table = rectangular_table(10.0, 0.0, 50);
        let geom = BridgeGeometry {
            low_chord_m: 5.0,
            low_chord_max_m: 5.0,
            high_chord_m: 7.0,
            high_chord_max_m: 7.0,
            pier_width_m: 0.5,
            num_piers: 2,
            pier_shape: PierShape::Square,
            abutments: BridgeAbutments::default(),
            weir_coeff_m: 1.44,
            orifice_coeff: 0.5,
            z_up_m: 0.1,
            z_down_m: 0.0,
            low_flow_method: LowFlowMethod::Auto,
            high_flow_method: HighFlowMethod::PressureWeir,
            length_m: 100.0,
            wspro_coeff_c: 0.8,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3,
            pressure_coeff_inlet: 0.0,
            pressure_coeff_submerged: 0.8,
            max_weir_submergence: 0.98,
            deck: None,
            pier_stations_m: vec![],
            skew_deg: 0.0,
            skew_cos: 1.0,
            ineffective_up: None,
            ineffective_down: None,
            xs_up: None,
            xs_down: None,
        };
        assert_eq!(
            classify_low_flow(15.0, 3.0, &geom, &table, &table),
            LowFlowClass::A
        );
    }

    #[test]
    fn test_asymmetric_abutments_reduce_area_more_on_wide_side() {
        let table = rectangular_table(10.0, 0.0, 50);
        let base = BridgeGeometry {
            low_chord_m: 5.0,
            low_chord_max_m: 5.0,
            high_chord_m: 7.0,
            high_chord_max_m: 7.0,
            pier_width_m: 0.0,
            num_piers: 0,
            pier_shape: PierShape::Square,
            abutments: BridgeAbutments::default(),
            weir_coeff_m: 1.44,
            orifice_coeff: 0.5,
            z_up_m: 0.0,
            z_down_m: 0.0,
            low_flow_method: LowFlowMethod::Momentum,
            high_flow_method: HighFlowMethod::PressureWeir,
            length_m: 100.0,
            wspro_coeff_c: 0.8,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3,
            pressure_coeff_inlet: 0.0,
            pressure_coeff_submerged: 0.8,
            max_weir_submergence: 0.98,
            deck: None,
            pier_stations_m: vec![],
            skew_deg: 0.0,
            skew_cos: 1.0,
            ineffective_up: None,
            ineffective_down: None,
            xs_up: None,
            xs_down: None,
        };
        let narrow_left = BridgeGeometry {
            abutments: resolve_abutments(
                &BridgeAbutmentUserInput {
                    left_width: Some(1.0),
                    right_width: Some(3.0),
                    ..Default::default()
                },
                0.0,
                10.0,
                1.0,
                UnitSystem::Metric,
            ),
            ..base.clone()
        };
        let symmetric = BridgeGeometry {
            abutments: BridgeAbutments::symmetric_total_width_m(4.0, 0.0, 10.0),
            ..base
        };
        let props_asym = obstructed_hydraulics(&table, 3.0, 0.0, &narrow_left, false);
        let props_sym = obstructed_hydraulics(&table, 3.0, 0.0, &symmetric, false);
        assert!(
            (props_asym.a_eff - props_sym.a_eff).abs() < 1e-6,
            "same total width should yield same effective area"
        );
        assert!((narrow_left.abutments.left_width_m() - 1.0).abs() < 1e-9);
        assert!((narrow_left.abutments.right_width_m() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_per_side_abutment_tops_affect_obstructed_hydraulics() {
        let table = rectangular_table(10.0, 0.0, 50);
        let base = BridgeGeometry {
            low_chord_m: 5.0,
            low_chord_max_m: 5.0,
            high_chord_m: 7.0,
            high_chord_max_m: 7.0,
            pier_width_m: 0.0,
            num_piers: 0,
            pier_shape: PierShape::Square,
            abutments: BridgeAbutments::default(),
            weir_coeff_m: 1.44,
            orifice_coeff: 0.5,
            z_up_m: 0.0,
            z_down_m: 0.0,
            low_flow_method: LowFlowMethod::Momentum,
            high_flow_method: HighFlowMethod::PressureWeir,
            length_m: 100.0,
            wspro_coeff_c: 0.8,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3,
            pressure_coeff_inlet: 0.0,
            pressure_coeff_submerged: 0.8,
            max_weir_submergence: 0.98,
            deck: None,
            pier_stations_m: vec![],
            skew_deg: 0.0,
            skew_cos: 1.0,
            ineffective_up: None,
            ineffective_down: None,
            xs_up: None,
            xs_down: None,
        };
        let geom_both = BridgeGeometry {
            abutments: resolve_abutments(
                &BridgeAbutmentUserInput {
                    left_width: Some(2.0),
                    right_width: Some(2.0),
                    left_top_elevation: Some(0.0),
                    right_top_elevation: Some(0.0),
                    ..Default::default()
                },
                0.0,
                10.0,
                1.0,
                UnitSystem::Metric,
            ),
            ..base.clone()
        };
        let geom_right_only = BridgeGeometry {
            abutments: resolve_abutments(
                &BridgeAbutmentUserInput {
                    left_width: Some(2.0),
                    right_width: Some(2.0),
                    left_top_elevation: Some(3.5),
                    right_top_elevation: Some(0.0),
                    ..Default::default()
                },
                0.0,
                10.0,
                1.0,
                UnitSystem::Metric,
            ),
            ..base
        };
        let props_both = obstructed_hydraulics(&table, 3.0, 0.0, &geom_both, false);
        let props_right = obstructed_hydraulics(&table, 3.0, 0.0, &geom_right_only, false);
        assert!(props_right.a_eff > props_both.a_eff);
        assert!(props_right.top_width > props_both.top_width);
    }

    #[test]
    fn test_abutment_reduces_opening_area() {
        let table = rectangular_table(10.0, 0.0, 50);
        let geom_no_abut = BridgeGeometry {
            low_chord_m: 5.0,
            low_chord_max_m: 5.0,
            high_chord_m: 7.0,
            high_chord_max_m: 7.0,
            pier_width_m: 0.0,
            num_piers: 0,
            pier_shape: PierShape::Square,
            abutments: BridgeAbutments::default(),
            weir_coeff_m: 1.44,
            orifice_coeff: 0.5,
            z_up_m: 0.0,
            z_down_m: 0.0,
            low_flow_method: LowFlowMethod::Momentum,
            high_flow_method: HighFlowMethod::PressureWeir,
            length_m: 100.0,
            wspro_coeff_c: 0.8,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3,
            pressure_coeff_inlet: 0.0,
            pressure_coeff_submerged: 0.8,
            max_weir_submergence: 0.98,
            deck: None,
            pier_stations_m: vec![],
            skew_deg: 0.0,
            skew_cos: 1.0,
            ineffective_up: None,
            ineffective_down: None,
            xs_up: None,
            xs_down: None,
        };
        let props_no = obstructed_hydraulics(&table, 3.0, 0.0, &geom_no_abut, false);
        let geom_abut = BridgeGeometry {
            abutments: BridgeAbutments::symmetric_total_width_m(2.0, 0.0, 10.0),
            ..geom_no_abut.clone()
        };
        let props_abut = obstructed_hydraulics(&table, 3.0, 0.0, &geom_abut, false);
        assert!(props_abut.a_eff < props_no.a_eff);
    }

    #[test]
    fn test_solve_bridge_wsel_yarnell_integration() {
        let table_up = rectangular_table(10.0, 0.1, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams {
            low_flow_method: 1,
            ..Default::default()
        };
        let wsel_up = solve_bridge_wsel(
            15.0,
            5.0,
            7.0,
            0.5,
            2,
            0,
            1.44,
            0.5,
            0.0,
            0.1,
            3.0,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            100.0,
            None,
            None,
        );
        assert!((wsel_up - 3.00247).abs() < 0.001);
    }

    #[test]
    fn test_solve_bridge_wsel_energy_no_obstructions() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams {
            low_flow_method: 3,
            ..Default::default()
        };
        let wsel_up = solve_bridge_wsel(
            20.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            None,
        );
        assert!(
            wsel_up > 2.5,
            "energy method should raise upstream WSEL above tailwater, got {wsel_up}"
        );
    }

    #[test]
    fn test_wspro_higher_c_lowers_head_loss() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let base = BridgeCouplingParams {
            low_flow_method: 4,
            abutment: BridgeAbutmentUserInput {
                legacy_total_width: 2.0,
                ..Default::default()
            },
            length: 50.0,
            ..Default::default()
        };
        let mut coupling_low_c = base.clone();
        coupling_low_c.wspro_coeff = 0.6;
        let mut coupling_high_c = base;
        coupling_high_c.wspro_coeff = 0.95;
        let hw_low = solve_bridge_wsel(
            15.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling_low_c,
            50.0,
            None,
            None,
        );
        let hw_high = solve_bridge_wsel(
            15.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling_high_c,
            50.0,
            None,
            None,
        );
        assert!(
            hw_high < hw_low,
            "higher WSPRO C should reduce upstream head, low_c={hw_low}, high_c={hw_high}"
        );
    }

    #[test]
    fn test_auto_low_flow_uses_wspro_with_abutments() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let auto_coupling = BridgeCouplingParams {
            low_flow_method: 0,
            abutment: BridgeAbutmentUserInput {
                legacy_total_width: 1.5,
                ..Default::default()
            },
            length: 50.0,
            ..Default::default()
        };
        let wspro_coupling = BridgeCouplingParams {
            low_flow_method: 4,
            abutment: BridgeAbutmentUserInput {
                legacy_total_width: 1.5,
                ..Default::default()
            },
            length: 50.0,
            ..Default::default()
        };
        let hw_auto = solve_bridge_wsel(
            15.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &auto_coupling,
            50.0,
            None,
            None,
        );
        let hw_wspro = solve_bridge_wsel(
            15.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &wspro_coupling,
            50.0,
            None,
            None,
        );
        assert!(
            (hw_auto - hw_wspro).abs() < 0.01,
            "auto with abutments should match explicit WSPRO, auto={hw_auto}, wspro={hw_wspro}"
        );
    }

    #[test]
    fn test_sluice_gate_cd_increases_with_submergence() {
        let cd_min = sluice_gate_discharge_coeff(0.0, 0.0);
        let cd_mid = sluice_gate_discharge_coeff(0.5, 0.0);
        let cd_deep = sluice_gate_discharge_coeff(1.0, 0.0);
        assert!(cd_deep > cd_mid);
        assert!(cd_mid > cd_min);
        assert!((cd_min - 0.27).abs() < 0.01);
        assert!((cd_deep - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_bradley_submergence_reduces_weir_factor() {
        assert!((bradley_weir_submergence_factor(0.0) - 1.0).abs() < 1e-6);
        assert!(bradley_weir_submergence_factor(0.9) < bradley_weir_submergence_factor(0.5));
        assert!(bradley_weir_submergence_factor(0.95) < 0.3);
    }

    #[test]
    fn test_submerged_orifice_constant_driving_head() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams::default();
        let q = 35.0;
        let hw_mild = solve_bridge_wsel(
            q,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.8,
            0.0,
            0.0,
            5.05,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            None,
        );
        let hw_deep = solve_bridge_wsel(
            q,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.8,
            0.0,
            0.0,
            5.8,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            None,
        );
        // Fully submerged orifice: E_up - TW ≈ constant for a given Q, so WSEL rises with tailwater.
        assert!(
            hw_deep > hw_mild,
            "deeper submergence should raise upstream WSEL, mild={hw_mild}, deep={hw_deep}"
        );
        let drive_mild = hw_mild - 5.05;
        let drive_deep = hw_deep - 5.8;
        assert!(
            (drive_mild - drive_deep).abs() < 0.05,
            "driving head should be similar, mild={drive_mild}, deep={drive_deep}"
        );
    }

    #[test]
    fn test_flat_deck_profile_matches_scalar_chords() {
        let deck = build_bridge_deck_profile(
            5.0,
            7.0,
            Some(&[0.0, 10.0, 20.0]),
            Some(&[5.0, 5.0, 5.0]),
            Some(&[7.0, 7.0, 7.0]),
            UnitSystem::Metric,
        )
        .expect("flat profile");
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams::default();
        let hw_scalar = solve_bridge_wsel(
            15.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.8,
            0.0,
            0.0,
            3.0,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            None,
        );
        let hw_profile = solve_bridge_wsel(
            15.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.8,
            0.0,
            0.0,
            3.0,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            Some(&deck),
            None,
        );
        assert!(
            (hw_scalar - hw_profile).abs() < 0.01,
            "flat profile should match scalar chords, scalar={hw_scalar}, profile={hw_profile}"
        );
    }

    #[test]
    fn test_deck_profile_hump_raises_headwater_at_low_flow() {
        let deck = build_bridge_deck_profile(
            5.0,
            7.0,
            Some(&[0.0, 10.0, 20.0]),
            Some(&[5.0, 6.5, 5.0]),
            Some(&[7.0, 7.5, 7.0]),
            UnitSystem::Metric,
        )
        .expect("humped profile");
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams::default();
        let hw_flat = solve_bridge_wsel(
            25.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.8,
            0.0,
            0.0,
            3.0,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            None,
        );
        let hw_profile = solve_bridge_wsel(
            25.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.8,
            0.0,
            0.0,
            3.0,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            Some(&deck),
            None,
        );
        assert!(
            hw_profile >= hw_flat,
            "center deck hump should not reduce headwater, flat={hw_flat}, profile={hw_profile}"
        );
        assert_eq!(deck.low_elevations_m.iter().cloned().fold(f64::INFINITY, f64::min), 5.0);
        assert_eq!(deck.low_elevations_m.iter().cloned().fold(f64::NEG_INFINITY, f64::max), 6.5);
    }

    #[test]
    fn test_ineffective_flow_raises_bridge_headwater() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams {
            low_flow_method: 3,
            ..Default::default()
        };
        let xs = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
            y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: Some(vec![false, false, false, false, true, true, true, true]),
            blocked_obstructions: None,
        };
        let sections_none = BridgeSectionContext::default();
        let ineff =
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap();
        let sections_ineff = BridgeSectionContext {
            ineffective_up: Some(ineff.clone()),
            ineffective_down: Some(ineff),
            xs_up: Some(xs.clone()),
            xs_down: Some(xs),
            ..Default::default()
        };

        let hw_none = solve_bridge_wsel(
            20.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            Some(&sections_none),
        );
        let hw_ineff = solve_bridge_wsel(
            20.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            Some(&sections_ineff),
        );
        assert!(
            hw_ineff >= hw_none,
            "ineffective left overbank should raise or maintain headwater, none={hw_none}, ineff={hw_ineff}"
        );
    }

    #[test]
    fn test_multi_block_ineffective_raises_bridge_headwater() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams {
            low_flow_method: 3,
            ..Default::default()
        };
        let xs = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0, 20.0, 20.0, 30.0, 30.0, 40.0, 40.0],
            y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: Some(vec![
                false, false, false, false, true, true, true, true, true, true,
            ]),
            blocked_obstructions: None,
        };
        let single = BridgeSectionContext {
            ineffective_up: Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
            ),
            ineffective_down: Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
            ),
            xs_up: Some(xs.clone()),
            xs_down: Some(xs.clone()),
            ..Default::default()
        };
        let multi = BridgeSectionContext {
            ineffective_up: Some(
                IneffectiveFlowAreas::from_block_pairs(&[20.0, 30.0], &[2.0, 3.5], &[], &[])
                    .unwrap(),
            ),
            ineffective_down: Some(
                IneffectiveFlowAreas::from_block_pairs(&[20.0, 30.0], &[2.0, 3.5], &[], &[])
                    .unwrap(),
            ),
            xs_up: Some(xs.clone()),
            xs_down: Some(xs),
            ..Default::default()
        };

        let hw_single = solve_bridge_wsel(
            20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&single),
        );
        let hw_multi = solve_bridge_wsel(
            20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&multi),
        );
        assert!(
            hw_multi >= hw_single,
            "inner ineffective block should raise headwater, single={hw_single}, multi={hw_multi}"
        );
    }

    #[test]
    fn test_separate_us_ds_ineffective_elevations() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams {
            low_flow_method: 3,
            ..Default::default()
        };
        let xs = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
            y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: Some(vec![false, false, false, false, true, true, true, true]),
            blocked_obstructions: None,
        };
        let sections_split = BridgeSectionContext {
            ineffective_up: Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[2.0], &[], &[]).unwrap(),
            ),
            ineffective_down: Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[5.0], &[], &[]).unwrap(),
            ),
            xs_up: Some(xs.clone()),
            xs_down: Some(xs.clone()),
            ..Default::default()
        };
        let sections_shared = BridgeSectionContext {
            ineffective_up: Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
            ),
            ineffective_down: Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
            ),
            xs_up: Some(xs.clone()),
            xs_down: Some(xs),
            ..Default::default()
        };

        let hw_split = solve_bridge_wsel(
            20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&sections_split),
        );
        let hw_shared = solve_bridge_wsel(
            20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&sections_shared),
        );
        assert!(
            hw_split >= hw_shared,
            "lower upstream ineffective activation should raise headwater, split={hw_split}, shared={hw_shared}"
        );
    }

    #[test]
    fn test_apply_bridge_skew_geometry() {
        let (w, l) = apply_bridge_skew(0.0, 20.0, 50.0);
        assert!((w - 20.0).abs() < 1e-6);
        assert!((l - 50.0).abs() < 1e-6);
        let (w30, l30) = apply_bridge_skew(30.0, 20.0, 50.0);
        assert!(w30 < w);
        assert!(l30 > l);
    }

    #[test]
    fn test_bridge_skew_increases_low_flow_headwater() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams {
            low_flow_method: 3,
            ..Default::default()
        };
        let plain = BridgeSectionContext::default();
        let skewed = BridgeSectionContext {
            skew_deg: 25.0,
            ..Default::default()
        };
        let hw_plain = solve_bridge_wsel(
            20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&plain),
        );
        let hw_skew = solve_bridge_wsel(
            20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&skewed),
        );
        assert!(
            hw_skew >= hw_plain,
            "skew should raise headwater via longer friction path, plain={hw_plain}, skew={hw_skew}"
        );
    }

    #[test]
    fn test_explicit_pier_stations_increase_headwater_vs_even_spacing() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams {
            low_flow_method: 1,
            ..Default::default()
        };
        let deck = build_bridge_deck_profile(
            5.0,
            7.0,
            Some(&[0.0, 20.0]),
            Some(&[5.0, 5.0]),
            Some(&[7.0, 7.0]),
            UnitSystem::Metric,
        )
        .unwrap();
        let two_piers = BridgeSectionContext {
            pier_stations: Some(vec![6.0, 14.0]),
            ..Default::default()
        };
        let three_piers = BridgeSectionContext {
            pier_stations: Some(vec![4.0, 10.0, 16.0]),
            ..Default::default()
        };
        let hw_two = solve_bridge_wsel(
            15.0, 5.0, 7.0, 0.5, 2, 0, 1.44, 0.5, 0.0, 0.0, 3.0,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, Some(&deck), Some(&two_piers),
        );
        let hw_three = solve_bridge_wsel(
            15.0, 5.0, 7.0, 0.5, 3, 0, 1.44, 0.5, 0.0, 0.0, 3.0,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, Some(&deck), Some(&three_piers),
        );
        assert!(
            hw_three > hw_two,
            "more pier stations should increase Yarnell headwater, two={hw_two}, three={hw_three}"
        );
    }

    fn abutment_coupling(left_w: f64, right_w: f64, left_top: f64, right_top: f64) -> BridgeCouplingParams {
        BridgeCouplingParams {
            abutment: BridgeAbutmentUserInput {
                left_width: Some(left_w),
                right_width: Some(right_w),
                left_top_elevation: Some(left_top),
                right_top_elevation: Some(right_top),
                ..Default::default()
            },
            length: 50.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_per_side_abutments_affect_energy_wspro_momentum_pressure() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let symmetric = abutment_coupling(2.5, 2.5, 0.0, 0.0);
        let asymmetric = abutment_coupling(1.0, 4.0, 0.0, 2.5);
        let q = 15.0;
        let tw = 2.5;

        let mut energy_sym = symmetric.clone();
        energy_sym.low_flow_method = 3;
        let mut energy_asym = asymmetric.clone();
        energy_asym.low_flow_method = 3;
        let hw_energy_sym = solve_bridge_wsel(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &energy_sym, 50.0, None, None,
        );
        let hw_energy_asym = solve_bridge_wsel(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &energy_asym, 50.0, None, None,
        );
        assert!(
            (hw_energy_sym - hw_energy_asym).abs() > 0.01,
            "energy method should reflect per-side abutment tops"
        );

        let mut wspro_sym = symmetric.clone();
        wspro_sym.low_flow_method = 4;
        let mut wspro_asym = asymmetric.clone();
        wspro_asym.low_flow_method = 4;
        let hw_wspro_sym = solve_bridge_wsel(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &wspro_sym, 50.0, None, None,
        );
        let hw_wspro_asym = solve_bridge_wsel(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &wspro_asym, 50.0, None, None,
        );
        assert!(
            (hw_wspro_sym - hw_wspro_asym).abs() > 0.01,
            "WSPRO should reflect per-side abutment tops"
        );

        let mut momentum = asymmetric.clone();
        momentum.low_flow_method = 2;
        let hw_momentum = solve_bridge_wsel(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &momentum, 50.0, None, None,
        );
        assert!(hw_momentum > tw);

        let mut pressure = asymmetric.clone();
        pressure.low_flow_method = 3;
        let hw_pressure = solve_bridge_wsel(
            35.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, 5.8,
            UnitSystem::Metric, &table_up, &table_down, &pressure, 50.0, None, None,
        );
        let mut pressure_sym = symmetric.clone();
        pressure_sym.low_flow_method = 3;
        let hw_pressure_sym = solve_bridge_wsel(
            35.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, 5.8,
            UnitSystem::Metric, &table_up, &table_down, &pressure_sym, 50.0, None, None,
        );
        assert!(
            (hw_pressure - hw_pressure_sym).abs() > 0.01,
            "pressure flow should reflect per-side abutment obstruction"
        );

        let mut yarnell_sym = symmetric.clone();
        yarnell_sym.low_flow_method = 1;
        let mut yarnell_asym = asymmetric.clone();
        yarnell_asym.low_flow_method = 1;
        let hw_yarnell_sym = solve_bridge_wsel(
            q, 5.0, 7.0, 0.5, 2, 0, 1.44, 0.5, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &yarnell_sym, 50.0, None, None,
        );
        let hw_yarnell_asym = solve_bridge_wsel(
            q, 5.0, 7.0, 0.5, 2, 0, 1.44, 0.5, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &yarnell_asym, 50.0, None, None,
        );
        assert!(
            (hw_yarnell_sym - hw_yarnell_asym).abs() > 0.001,
            "Yarnell should use per-side abutment area in pier alpha"
        );

        let q_weir = 50.0;
        let tw_weir = 5.5;
        let weir_sym = solve_bridge_coupled(
            q_weir, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw_weir,
            UnitSystem::Metric, &table_up, &table_down, &symmetric, 50.0, None, None,
        );
        let weir_asym = solve_bridge_coupled(
            q_weir, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw_weir,
            UnitSystem::Metric, &table_up, &table_down, &asymmetric, 50.0, None, None,
        );
        assert!(
            weir_sym.flow_regime == "weir" || weir_sym.flow_regime == "pressure",
            "expected high-flow regime, got {}",
            weir_sym.flow_regime
        );
        assert!(
            (weir_sym.wsel_up - weir_asym.wsel_up).abs() > 0.01,
            "weir/pressure EGL should reflect per-side abutment obstruction"
        );
    }

    #[test]
    fn test_bridge_rating_curve() {
        let inputs = BridgeRatingCurveInputs {
            q_values: vec![10.0, 20.0, 30.0],
            bridge: BridgeSolveParams {
                low_chord: 5.0,
                high_chord: 7.0,
                z_down: 0.0,
                z_up: 0.0,
                tw_wsel: 2.5,
                low_flow_method: 3,
                channel_width: 10.0,
                manning_n: 0.03,
                ..Default::default()
            },
        };
        let curve = compute_bridge_rating_curve(&inputs);
        assert_eq!(curve.q.len(), 3);
        assert!(curve.wsel[1] > curve.wsel[0]);
        assert!(curve.wsel[2] > curve.wsel[1]);
        assert_eq!(curve.wsel_down.len(), 3);
        assert_eq!(curve.flow_regimes.len(), 3);
        assert!(!curve.flow_regimes[0].is_empty());
    }

    fn hand_rectangular_a_eff(
        channel_width_m: f64,
        wsel_m: f64,
        z_bed_m: f64,
        geom: &BridgeGeometry,
    ) -> f64 {
        let a_base = channel_width_m * (wsel_m - z_bed_m).max(0.0);
        (a_base - geom.abutments.submerged_area_m2(wsel_m, z_bed_m)).max(1e-5)
    }

    #[test]
    fn test_obstructed_area_hand_calc_asymmetric_and_one_sided() {
        let table = rectangular_table(10.0, 0.0, 50);
        let base = BridgeGeometry {
            low_chord_m: 5.0,
            low_chord_max_m: 5.0,
            high_chord_m: 7.0,
            high_chord_max_m: 7.0,
            pier_width_m: 0.0,
            num_piers: 0,
            pier_shape: PierShape::Square,
            abutments: BridgeAbutments::default(),
            weir_coeff_m: 1.44,
            orifice_coeff: 0.5,
            z_up_m: 0.0,
            z_down_m: 0.0,
            low_flow_method: LowFlowMethod::Wspro,
            high_flow_method: HighFlowMethod::PressureWeir,
            length_m: 50.0,
            wspro_coeff_c: 0.8,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3,
            pressure_coeff_inlet: 0.0,
            pressure_coeff_submerged: 0.8,
            max_weir_submergence: 0.98,
            deck: None,
            pier_stations_m: vec![],
            skew_deg: 0.0,
            skew_cos: 1.0,
            ineffective_up: None,
            ineffective_down: None,
            xs_up: None,
            xs_down: None,
        };

        let asymmetric = BridgeGeometry {
            abutments: resolve_abutments(
                &BridgeAbutmentUserInput {
                    left_width: Some(1.0),
                    right_width: Some(4.0),
                    left_top_elevation: Some(0.0),
                    right_top_elevation: Some(2.5),
                    ..Default::default()
                },
                0.0,
                10.0,
                1.0,
                UnitSystem::Metric,
            ),
            ..base.clone()
        };
        let props_asym = obstructed_hydraulics(&table, 2.5, 0.0, &asymmetric, false);
        let hand_asym = hand_rectangular_a_eff(10.0, 2.5, 0.0, &asymmetric);
        assert!((hand_asym - 22.5).abs() < 1e-6, "hand A_eff@2.5 = {hand_asym}");
        assert!(
            (props_asym.a_eff - hand_asym).abs() < 1e-3,
            "obstructed_hydraulics {:.4} vs hand {:.4}",
            props_asym.a_eff,
            hand_asym
        );

        let left_only = BridgeGeometry {
            abutments: resolve_abutments(
                &BridgeAbutmentUserInput {
                    left_width: Some(3.0),
                    left_top_elevation: Some(0.0),
                    ..Default::default()
                },
                0.0,
                10.0,
                1.0,
                UnitSystem::Metric,
            ),
            ..base.clone()
        };
        let props_left = obstructed_hydraulics(&table, 2.5, 0.0, &left_only, false);
        assert!((hand_rectangular_a_eff(10.0, 2.5, 0.0, &left_only) - 17.5).abs() < 1e-6);
        assert!((props_left.a_eff - 17.5).abs() < 1e-3);

        let right_only = BridgeGeometry {
            abutments: resolve_abutments(
                &BridgeAbutmentUserInput {
                    right_width: Some(3.0),
                    right_top_elevation: Some(2.0),
                    ..Default::default()
                },
                0.0,
                10.0,
                1.0,
                UnitSystem::Metric,
            ),
            ..base
        };
        let props_right = obstructed_hydraulics(&table, 2.5, 0.0, &right_only, false);
        assert!((hand_rectangular_a_eff(10.0, 2.5, 0.0, &right_only) - 23.5).abs() < 1e-6);
        assert!((props_right.a_eff - 23.5).abs() < 1e-3);
    }

    #[test]
    fn test_wspro_headwater_hand_calc_reference_cases() {
        let table = rectangular_table(10.0, 0.0, 50);
        let q = 15.0;
        let tw = 2.5;

        let cases: [(&str, BridgeCouplingParams, f64, f64); 3] = [
            (
                "asymmetric_per_side",
                abutment_coupling(1.0, 4.0, 0.0, 2.5),
                22.5,
                2.511_630_058_288_574,
            ),
            (
                "one_sided_left",
                BridgeCouplingParams {
                    abutment: BridgeAbutmentUserInput {
                        left_width: Some(3.0),
                        left_top_elevation: Some(0.0),
                        ..Default::default()
                    },
                    length: 50.0,
                    low_flow_method: 4,
                    ..Default::default()
                },
                17.5,
                2.519_601_583_480_835,
            ),
            (
                "symmetric_full_height",
                abutment_coupling(2.5, 2.5, 0.0, 0.0),
                12.5,
                2.539_474_964_141_846,
            ),
        ];

        let base = BridgeGeometry {
            low_chord_m: 5.0,
            low_chord_max_m: 5.0,
            high_chord_m: 7.0,
            high_chord_max_m: 7.0,
            pier_width_m: 0.0,
            num_piers: 0,
            pier_shape: PierShape::Square,
            abutments: BridgeAbutments::default(),
            weir_coeff_m: 1.44,
            orifice_coeff: 0.5,
            z_up_m: 0.0,
            z_down_m: 0.0,
            low_flow_method: LowFlowMethod::Wspro,
            high_flow_method: HighFlowMethod::PressureWeir,
            length_m: 50.0,
            wspro_coeff_c: 0.8,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3,
            pressure_coeff_inlet: 0.0,
            pressure_coeff_submerged: 0.8,
            max_weir_submergence: 0.98,
            deck: None,
            pier_stations_m: vec![],
            skew_deg: 0.0,
            skew_cos: 1.0,
            ineffective_up: None,
            ineffective_down: None,
            xs_up: None,
            xs_down: None,
        };

        for (name, mut coupling, a_eff_tw, expected_hw) in cases {
            coupling.low_flow_method = 4;
            let geom = BridgeGeometry {
                abutments: resolve_abutments(
                    &coupling.abutment,
                    0.0,
                    10.0,
                    1.0,
                    UnitSystem::Metric,
                ),
                length_m: coupling.length,
                wspro_coeff_c: coupling.wspro_coeff,
                coeff_contraction: coupling.coeff_contraction,
                coeff_expansion: coupling.coeff_expansion,
                ..base.clone()
            };
            assert!(
                (hand_rectangular_a_eff(10.0, tw, 0.0, &geom) - a_eff_tw).abs() < 1e-3,
                "{name}: hand A_eff@TW"
            );

            let hw = solve_bridge_wsel(
                q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
                UnitSystem::Metric, &table, &table, &coupling, 50.0, None, None,
            );
            assert!(
                (hw - expected_hw).abs() < 0.002,
                "{name}: WSPRO hw {hw:.4} vs reference {expected_hw:.4}"
            );

            let props_down = obstructed_hydraulics(&table, tw, 0.0, &geom, false);
            let props_up = obstructed_hydraulics(&table, hw, 0.0, &geom, true);
            let opening_wsel = hw.min(tw).min(geom.low_chord_m);
            let props_open = obstructed_hydraulics(&table, opening_wsel, 0.0, &geom, true);
            let e_down = tw + velocity_head(q, props_down.a_eff);
            let e_up = hw + velocity_head(q, props_up.a_eff);
            let hf = friction_loss(
                q,
                obstructed_conveyance(&table, tw, 0.0, &geom, false),
                obstructed_conveyance(&table, hw, 0.0, &geom, true),
                geom.length_m,
            );
            let h_wspro = wspro_contraction_loss(
                q,
                props_up.a_eff,
                props_open.a_eff.max(1e-5),
                geom.wspro_coeff_c,
            );
            assert!(
                (e_up - e_down - hf - h_wspro).abs() < 1e-4,
                "{name}: WSPRO energy balance residual"
            );
        }
    }

    #[test]
    fn test_bridge_rating_curve_per_side_abutments() {
        let base = BridgeSolveParams {
            low_chord: 5.0,
            high_chord: 7.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 2.5,
            low_flow_method: 4,
            channel_width: 10.0,
            manning_n: 0.03,
            ..Default::default()
        };
        let asymmetric = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
            q_values: vec![15.0, 25.0],
            bridge: BridgeSolveParams {
                abutment_left_width: Some(1.0),
                abutment_right_width: Some(4.0),
                abutment_right_top_elevation: Some(2.5),
                ..base.clone()
            },
        });
        let legacy_symmetric = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
            q_values: vec![15.0, 25.0],
            bridge: BridgeSolveParams {
                abutment_block_width: 5.0,
                ..base
            },
        });
        assert!(
            (asymmetric.wsel[0] - legacy_symmetric.wsel[0]).abs() > 0.01,
            "rating curve should honor per-side abutment geometry"
        );
        assert!(
            asymmetric.wsel[1] > asymmetric.wsel[0],
            "headwater should increase with discharge"
        );
    }

    #[test]
    fn test_explicit_high_flow_energy_method() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let pressure_coupling = BridgeCouplingParams::default();
        let energy_coupling = BridgeCouplingParams {
            high_flow_method: 1,
            low_flow_method: 3,
            ..Default::default()
        };
        let q = 35.0;
        let tw = 5.8;

        let pressure = solve_bridge_coupled(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &pressure_coupling, 50.0, None, None,
        );
        let energy = solve_bridge_coupled(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &energy_coupling, 50.0, None, None,
        );

        assert_eq!(pressure.flow_regime, "pressure");
        assert_eq!(energy.flow_regime, "energy");
        assert!(energy.wsel_up > tw);
        assert!(
            (pressure.wsel_up - energy.wsel_up).abs() > 0.01,
            "explicit energy should differ from pressure/weir, pressure={}, energy={}",
            pressure.wsel_up,
            energy.wsel_up
        );
    }

    #[test]
    fn test_high_flow_energy_supercritical_roundtrip() {
        let table_up = rectangular_table(10.0, 0.0, 50);
        let table_down = rectangular_table(10.0, 0.0, 50);
        let coupling = BridgeCouplingParams {
            high_flow_method: 1,
            low_flow_method: 3,
            ..Default::default()
        };
        let q = 30.0;
        let hw = solve_bridge_wsel(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, 5.5,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, None,
        );
        let tw = solve_bridge_tailwater(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, hw,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, None,
        );
        let hw_back = solve_bridge_wsel(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw,
            UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, None,
        );
        assert!((hw_back - hw).abs() < 0.05, "roundtrip hw={hw}, hw_back={hw_back}, tw={tw}");
    }
}
