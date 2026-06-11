use crate::geometry::{
    flow_area_for_row, row_at_elevation, CrossSection, GeometryRow, GeometryTable, GuideBanks,
    IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::{
    resolve_abutments, BridgeAbutmentUserInput, BridgeAbutments,
};
use crate::solvers::deck_vent_geometry::{
    resolve_deck_vents, total_deck_vent_discharge_m3s, DeckVentUserInput, ResolvedDeckVent,
};
use crate::solvers::pier_geometry::{
    evenly_spaced_pier_stations, resolve_pier_width_specs, PierAttachmentsUserInput,
    PierWidthUserInput, ResolvedPier,
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
    /// BU (bridge upstream face) cross section when explicit or from reach fallback.
    pub xs_up: Option<CrossSection>,
    /// BD (bridge downstream face) cross section when explicit or from reach fallback.
    pub xs_down: Option<CrossSection>,
    /// Optional interior bridge cuts (US → DS). Stored for future multi-segment hydraulics.
    pub internal_xs: Vec<CrossSection>,
    /// Reach XS lateral `x` at bridge opening station 0 (left deck edge).
    pub opening_reach_station_origin: Option<f64>,
    /// Skew from normal to flow, degrees (0–59°; same convention as culverts).
    pub skew_deg: f64,
    /// Pier centerline stations across the opening (user units; same frame as deck stations).
    pub pier_stations: Option<Vec<f64>>,
    /// Reach friction length BU → BD (metric), including interior cut spacing when provided.
    pub friction_length_m: f64,
    /// Approach cross section (HEC-RAS section 4 equivalent) when resolved.
    pub xs_approach: Option<CrossSection>,
    /// Departure / exit cross section when resolved.
    pub xs_departure: Option<CrossSection>,
    /// Guide banks on the approach cut for bridge contraction / WSPRO approach area.
    pub guide_banks_approach: Option<GuideBanks>,
    /// Guide banks on the departure cut for bridge expansion area.
    pub guide_banks_departure: Option<GuideBanks>,
    /// Optional per-pier tapered width overrides (user units; converted in `build_bridge_geometry`).
    pub pier_widths: Option<PierWidthUserInput>,
    /// Optional per-pier footing and nosing (user units; converted in `build_bridge_geometry`).
    pub pier_attachments: Option<PierAttachmentsUserInput>,
    /// Optional deck vent / slotted-opening segments (user units; converted in `build_bridge_geometry`).
    pub deck_vents: Option<DeckVentUserInput>,
}

/// HEC-RAS-style bridge skew: projected opening width × cos(θ), friction length ÷ cos(θ).
pub fn apply_bridge_skew(skew_deg: f64, width_m: f64, length_m: f64) -> (f64, f64) {
    apply_barrel_skew(skew_deg, width_m, length_m)
}

/// Supported pier shape types (Yarnell $K$ and momentum $C_D$ per HEC-RAS 6.x low-flow tables).
///
/// Values `0`–`3` are unchanged from API v1. Values `4`–`11` added in API v29
/// (`docs/development/extended_pier_shape_catalog.md`).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PierShape {
    Square = 0,
    Semicircular = 1,
    /// Twin-cylinder piers **with** connecting diaphragm.
    TwinCylinder = 2,
    /// 90° triangular nose and tail.
    Triangular = 3,
    /// Twin-cylinder piers **without** diaphragm ($K$ matches 90° triangular; $C_D$ elongated).
    TwinCylinderNoDiaphragm = 4,
    TenPileTrestle = 5,
    Elliptical2to1 = 6,
    Elliptical4to1 = 7,
    Elliptical8to1 = 8,
    Triangular30 = 9,
    Triangular60 = 10,
    Triangular120 = 11,
}

impl PierShape {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => PierShape::Semicircular,
            2 => PierShape::TwinCylinder,
            3 => PierShape::Triangular,
            4 => PierShape::TwinCylinderNoDiaphragm,
            5 => PierShape::TenPileTrestle,
            6 => PierShape::Elliptical2to1,
            7 => PierShape::Elliptical4to1,
            8 => PierShape::Elliptical8to1,
            9 => PierShape::Triangular30,
            10 => PierShape::Triangular60,
            11 => PierShape::Triangular120,
            _ => PierShape::Square,
        }
    }

    /// Yarnell pier shape coefficient $K$ (HEC-RAS Yarnell table).
    ///
    /// Elliptical variants have no HEC Yarnell row — uses semicircular $K=0.90$ when Yarnell
    /// low flow is selected. Ten-pile trestle uses the documented $K=2.50$ row.
    pub fn yarnell_coefficient(&self) -> f64 {
        match self {
            PierShape::Square => 1.25,
            PierShape::Semicircular => 0.90,
            PierShape::TwinCylinder => 0.95,
            PierShape::Triangular | PierShape::TwinCylinderNoDiaphragm => 1.05,
            PierShape::Triangular30 | PierShape::Triangular60 | PierShape::Triangular120 => 1.05,
            PierShape::TenPileTrestle => 2.50,
            PierShape::Elliptical2to1 | PierShape::Elliptical4to1 | PierShape::Elliptical8to1 => {
                0.90
            }
        }
    }

    /// Momentum pier drag coefficient $C_D$ (HEC-RAS pier drag table).
    ///
    /// Ten-pile trestle has no HEC momentum row — uses square-nose $C_D=2.00$.
    pub fn drag_coefficient(&self) -> f64 {
        match self {
            PierShape::Semicircular => 1.20,
            PierShape::TwinCylinder | PierShape::TwinCylinderNoDiaphragm => 1.33,
            PierShape::Triangular => 1.60,
            PierShape::Triangular30 => 1.00,
            PierShape::Triangular60 => 1.39,
            PierShape::Triangular120 => 1.72,
            PierShape::Elliptical2to1 => 0.60,
            PierShape::Elliptical4to1 => 0.32,
            PierShape::Elliptical8to1 => 0.29,
            PierShape::Square | PierShape::TenPileTrestle => 2.00,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum BridgeFlowRegimeKind {
    LowA,
    LowB,
    LowC,
    Pressure,
    Weir,
    Energy,
}

impl BridgeFlowRegimeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LowA => "low_a",
            Self::LowB => "low_b",
            Self::LowC => "low_c",
            Self::Pressure => "pressure",
            Self::Weir => "weir",
            Self::Energy => "energy",
        }
    }
}

impl LowFlowClass {
    fn flow_regime(self) -> BridgeFlowRegimeKind {
        match self {
            Self::A => BridgeFlowRegimeKind::LowA,
            Self::B => BridgeFlowRegimeKind::LowB,
            Self::C => BridgeFlowRegimeKind::LowC,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BridgeHeadwaterSolve {
    pub wsel_m: f64,
    pub regime: BridgeFlowRegimeKind,
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
    /// Tapered pier top width per pier (perpendicular to flow). With `pier_bottom_widths`, linear taper.
    #[serde(default)]
    pub pier_top_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub pier_bottom_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub pier_width_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub pier_width_values: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub pier_top_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub pier_base_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub pier_footing_top_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub pier_footing_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub pier_footing_bottom_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub pier_nosing_lengths: Option<Vec<f64>>,
    #[serde(default)]
    pub pier_nosing_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_vent_left_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_vent_right_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_vent_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_vent_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_vent_invert_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_vent_soffit_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_vent_discharge_coefficients: Option<Vec<f64>>,
    #[serde(default)]
    pub deck_vent_types: Option<Vec<i32>>,
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
    /// Reach XS lateral `x` at bridge opening station 0 (aligns opening ↔ reach frames).
    #[serde(default)]
    pub opening_reach_station_origin: Option<f64>,
    /// Optional interior bridge cuts (US → DS). Stored; hydraulics use BU/BD only today.
    #[serde(default)]
    pub xs_internal: Option<Vec<CrossSection>>,
    /// Unified roadway embankment (API v26). Composes flat deck/abutment/ineffective/blocked fields.
    #[serde(default)]
    pub roadway_embankment: Option<crate::solvers::bridge_roadway_compose::BridgeRoadwayEmbankment>,
    #[serde(default, skip_serializing)]
    pub(crate) composed_embankment_blocked:
        Option<crate::solvers::bridge_roadway_compose::ComposedEmbankmentBlocked>,
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
            pier_top_widths: None,
            pier_bottom_widths: None,
            pier_width_elevations: None,
            pier_width_values: None,
            pier_top_elevations: None,
            pier_base_elevations: None,
            pier_footing_top_elevations: None,
            pier_footing_widths: None,
            pier_footing_bottom_elevations: None,
            pier_nosing_lengths: None,
            pier_nosing_widths: None,
            deck_vent_left_stations: None,
            deck_vent_right_stations: None,
            deck_vent_stations: None,
            deck_vent_widths: None,
            deck_vent_invert_elevations: None,
            deck_vent_soffit_elevations: None,
            deck_vent_discharge_coefficients: None,
            deck_vent_types: None,
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
            opening_reach_station_origin: None,
            xs_internal: None,
            roadway_embankment: None,
            composed_embankment_blocked: None,
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

    /// Shift opening-frame deck stations to reach XS coordinates (metric).
    pub fn remap_stations_to_reach(&mut self, origin_user: f64, units: UnitSystem) {
        let origin_m = if units == UnitSystem::USCustomary {
            origin_user * FT_TO_M
        } else {
            origin_user
        };
        for s in &mut self.stations_m {
            *s += origin_m;
        }
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
    /// Resolved per-pier width specs (metric). Empty → synthesize legacy constant prisms at solve time.
    pub pier_specs: Vec<ResolvedPier>,
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
    /// Approach (section 4) cut for contraction / WSPRO approach area.
    pub xs_approach: Option<CrossSection>,
    /// Departure cut for expansion area.
    pub xs_departure: Option<CrossSection>,
    pub guide_banks_approach: Option<GuideBanks>,
    pub guide_banks_departure: Option<GuideBanks>,
    pub table_approach: Option<GeometryTable>,
    pub table_departure: Option<GeometryTable>,
    /// Supplemental pressure-flow paths through deck vents / slots (metric).
    pub deck_vents: Vec<ResolvedDeckVent>,
}

const APPROACH_DEPARTURE_TABLE_SLICES: usize = 50;

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
    guide_banks: Option<&GuideBanks>,
    wsel: f64,
) -> GeometryRow {
    if let Some(xs) = xs {
        row_at_elevation(table, xs, wsel, ineffective, guide_banks)
    } else {
        let row = table.interpolate(wsel);
        GeometryRow {
            active_area: row.area,
            active_channel_area: row.channel_area,
            ..row
        }
    }
}

fn base_flow_area(
    row: &GeometryRow,
    ineffective: Option<&IneffectiveFlowAreas>,
    guide_banks: Option<&GuideBanks>,
) -> f64 {
    let has_ineffective = ineffective.filter(|i| i.is_configured()).is_some()
        || row.active_area + 1e-6 < row.area;
    let has_guide = guide_banks.filter(|g| g.is_configured()).is_some();
    if has_ineffective && !has_guide {
        return flow_area_for_row(row);
    }
    let clip_active = has_ineffective || has_guide;
    if row.channel_area > 1e-6 {
        if clip_active {
            row.active_channel_area
        } else {
            row.channel_area
        }
    } else if clip_active {
        row.active_area
    } else {
        row.area
    }
}

fn ineffective_on_cut(xs: Option<&CrossSection>) -> Option<&IneffectiveFlowAreas> {
    xs.and_then(|x| x.ineffective_flow_areas.as_ref())
        .filter(|i| i.is_configured())
}

fn guide_banks_configured_on_side(geom: &BridgeGeometry, is_approach: bool) -> bool {
    if is_approach {
        geom.guide_banks_approach
            .as_ref()
            .is_some_and(|g| g.is_configured())
    } else {
        geom.guide_banks_departure
            .as_ref()
            .is_some_and(|g| g.is_configured())
    }
}

fn approach_departure_cut_modifiers_active(geom: &BridgeGeometry, is_approach: bool) -> bool {
    if guide_banks_configured_on_side(geom, is_approach) {
        return true;
    }
    let xs = if is_approach {
        geom.xs_approach.as_ref()
    } else {
        geom.xs_departure.as_ref()
    };
    ineffective_on_cut(xs).is_some()
}

/// Active flow area on approach or departure cut (guide banks and/or ineffective on that cut).
fn reach_cut_flow_area(geom: &BridgeGeometry, is_approach: bool, wsel: f64) -> Option<f64> {
    if !approach_departure_cut_modifiers_active(geom, is_approach) {
        return None;
    }
    let (xs, table, guide_banks) = if is_approach {
        (
            geom.xs_approach.as_ref(),
            geom.table_approach.as_ref(),
            geom.guide_banks_approach.as_ref(),
        )
    } else {
        (
            geom.xs_departure.as_ref(),
            geom.table_departure.as_ref(),
            geom.guide_banks_departure.as_ref(),
        )
    };
    let xs = xs?;
    let table = table?;
    let ineffective = ineffective_on_cut(Some(xs));
    let guide_banks = guide_banks.filter(|g| g.is_configured());
    let row = lookup_row(table, Some(xs), ineffective, guide_banks, wsel);
    Some(base_flow_area(&row, ineffective, guide_banks))
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

fn legacy_resolved_piers(geom: &BridgeGeometry) -> Vec<ResolvedPier> {
    let (s_min, s_max) = opening_station_bounds_m(geom);
    let inset = geom.pier_width_m.max(0.0) * 0.5;
    let stations = if !geom.pier_stations_m.is_empty() {
        geom.pier_stations_m.clone()
    } else {
        evenly_spaced_pier_stations(geom.num_piers, s_min, s_max, inset)
    };
    let z_bed = geom.z_up_m.min(geom.z_down_m);
    let z_tops: Vec<f64> = stations
        .iter()
        .map(|&s| {
            geom.deck
                .as_ref()
                .map(|d| interpolate_profile(&d.stations_m, &d.low_elevations_m, s))
                .unwrap_or(geom.low_chord_m)
        })
        .collect();
    resolve_pier_width_specs(
        geom.pier_width_m,
        &stations,
        z_bed,
        &z_tops,
        None,
        None,
    )
}

fn active_resolved_piers(geom: &BridgeGeometry) -> Vec<ResolvedPier> {
    let piers = if geom.pier_specs.is_empty() {
        legacy_resolved_piers(geom)
    } else {
        geom.pier_specs.clone()
    };
    piers
        .into_iter()
        .filter(|p| pier_in_opening_span(geom, p))
        .collect()
}

fn pier_half_width_opening_m(geom: &BridgeGeometry, pier: &ResolvedPier) -> f64 {
    pier.spec.width_perp_at(geom.low_chord_m).max(0.0) / geom.skew_cos * 0.5
}

fn pier_in_opening_span(geom: &BridgeGeometry, pier: &ResolvedPier) -> bool {
    let (s_min, s_max) = opening_station_bounds_m(geom);
    let half = pier_half_width_opening_m(geom, pier);
    pier.station_m + half > s_min && pier.station_m - half < s_max
}

fn total_pier_flow_width_at_wsel_m(geom: &BridgeGeometry, wsel: f64, z_bed: f64) -> f64 {
    let piers = active_resolved_piers(geom);
    crate::solvers::pier_geometry::total_pier_flow_width_at_wsel_m(
        &piers,
        wsel,
        z_bed,
        geom.skew_cos,
    )
}

fn pier_submerged_area_at_wsel(geom: &BridgeGeometry, wsel: f64, z_bed: f64) -> f64 {
    let piers = active_resolved_piers(geom);
    crate::solvers::pier_geometry::total_submerged_pier_area_m2(
        &piers,
        wsel,
        z_bed,
        geom.skew_cos,
    )
}

/// Downstream flow area for Yarnell: base area minus per-side abutments, before pier blockage.
fn yarnell_downstream_flow_area_m2(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
) -> f64 {
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, false);
    (props.a_eff + pier_submerged_area_at_wsel(geom, wsel, z_bed)).max(1e-5)
}

/// HEC-RAS weighting: use the more constricted of BU and BD at a common water-surface elevation.
fn obstructed_opening_at_wsel(
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
    wsel: f64,
) -> (ObstructedHydraulics, bool) {
    let up = obstructed_hydraulics(table_up, wsel, geom.z_up_m, geom, true);
    let down = obstructed_hydraulics(table_down, wsel, geom.z_down_m, geom, false);
    if up.a_eff <= down.a_eff {
        (up, true)
    } else {
        (down, false)
    }
}

/// Vertical opening below the low chord (minimum of BU and BD invert depths).
fn opening_height_below_deck_m(geom: &BridgeGeometry) -> f64 {
    let h_up = (geom.low_chord_m - geom.z_up_m).max(0.0);
    let h_down = (geom.low_chord_m - geom.z_down_m).max(0.0);
    h_up.min(h_down).max(1e-3)
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
        None,
        wsel,
    );
    let a_base = base_flow_area(&row, ineffective, None);
    let depth = (wsel - z_bed).max(0.0);
    let a_piers = pier_submerged_area_at_wsel(geom, wsel, z_bed);
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
    let top_width = (t_base
        - total_pier_flow_width_at_wsel_m(geom, wsel, z_bed)
        - abut_width_at_wsel)
        .max(1e-3);

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
    let row = lookup_row(table, section_xs(geom, is_upstream), ineffective, None, wsel);
    let a_base = base_flow_area(&row, ineffective, None);
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
            let (props_opening, _) =
                obstructed_opening_at_wsel(geom, table_up, table_down, opening_wsel);
            let a_approach = reach_cut_flow_area(geom, true, wsel_up).unwrap_or(props_up.a_eff);
            wspro_contraction_loss(
                q_metric,
                a_approach,
                props_opening.a_eff.max(1e-5),
                geom.wspro_coeff_c,
            )
            .max(0.0)
        } else if approach_departure_cut_modifiers_active(geom, true)
            || approach_departure_cut_modifiers_active(geom, false)
        {
            let opening_wsel = wsel_up.min(tw_m).min(geom.low_chord_m);
            let (props_opening, _) =
                obstructed_opening_at_wsel(geom, table_up, table_down, opening_wsel);
            let a_bridge = props_opening.a_eff.max(1e-5);
            let a_approach = reach_cut_flow_area(geom, true, wsel_up).unwrap_or(props_up.a_eff);
            let a_departure =
                reach_cut_flow_area(geom, false, tw_m).unwrap_or(props_down.a_eff);
            let hv_approach = velocity_head(q_metric, a_approach);
            let hv_bridge = velocity_head(q_metric, a_bridge);
            let hv_departure = velocity_head(q_metric, a_departure);
            let h_contract = if hv_approach > hv_bridge {
                geom.coeff_contraction * (hv_approach - hv_bridge)
            } else {
                0.0
            };
            let h_expand = if hv_bridge > hv_departure {
                geom.coeff_expansion * (hv_bridge - hv_departure)
            } else {
                0.0
            };
            h_contract + h_expand
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
    let a_pier = pier_submerged_area_at_wsel(geom, wsel, z_bed);
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
    yarnell_pier_head_loss_from_area(
        q_metric,
        wsel_down_metric,
        z_bed_down_metric,
        a_piers,
        flow_area_m2,
        pier_shape,
    )
}

fn yarnell_pier_head_loss_integrated(
    q_metric: f64,
    wsel_down_metric: f64,
    z_bed_down_metric: f64,
    geom: &BridgeGeometry,
    flow_area_m2: f64,
) -> f64 {
    if q_metric <= 1e-5 || flow_area_m2 <= 1e-5 {
        return 0.0;
    }
    let a_piers = pier_submerged_area_at_wsel(geom, wsel_down_metric, z_bed_down_metric);
    yarnell_pier_head_loss_from_area(
        q_metric,
        wsel_down_metric,
        z_bed_down_metric,
        a_piers,
        flow_area_m2,
        geom.pier_shape,
    )
}

fn yarnell_pier_head_loss_from_area(
    q_metric: f64,
    wsel_down_metric: f64,
    z_bed_down_metric: f64,
    a_piers: f64,
    flow_area_m2: f64,
    pier_shape: PierShape,
) -> f64 {
    let depth_down = (wsel_down_metric - z_bed_down_metric).max(0.0);
    if depth_down <= 1e-5 || a_piers <= 1e-6 {
        return 0.0;
    }

    let a_unobstructed = (flow_area_m2 - a_piers).max(1e-5);
    let a_piers_clamped = a_piers.min(a_unobstructed * 0.9);
    let alpha = a_piers_clamped / a_unobstructed;

    let v_ds = q_metric / flow_area_m2;
    let velocity_head = (v_ds * v_ds) / (2.0 * G_METRIC);
    let omega = velocity_head / depth_down;
    let k = pier_shape.yarnell_coefficient();

    2.0 * k * (k + 10.0 * omega - 0.6) * (alpha + 15.0 * alpha.powi(4)) * velocity_head
}

fn gross_opening_area_at_low_chord(
    geom: &BridgeGeometry,
    table: &GeometryTable,
    z_bed: f64,
    is_upstream: bool,
) -> f64 {
    let wsel = geom.low_chord_m;
    let height_under_deck = (wsel - z_bed).max(0.0);
    let deck_width = gross_projected_opening_width_m(geom);
    let a_gross = if deck_width > 1e-6 {
        deck_width * height_under_deck
    } else {
        let ineffective = ineffective_for_side(geom, is_upstream);
        let row = lookup_row(
            table,
            section_xs(geom, is_upstream),
            ineffective,
            None,
            wsel,
        );
        base_flow_area(&row, ineffective, None)
    };
    let a_piers = pier_submerged_area_at_wsel(geom, wsel, z_bed);
    let a_abut = geom.abutments.submerged_area_m2(wsel, z_bed);
    (a_gross - a_piers - a_abut).max(1e-4)
}

/// Net opening area at the low chord using HEC-RAS min(BU, BD) weighting.
fn net_opening_area_at_low_chord(
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let factor = profile_opening_area_factor(geom);
    let wsel = geom.low_chord_m;
    if geom.xs_up.is_some() || geom.xs_down.is_some() {
        let (props, _) = obstructed_opening_at_wsel(geom, table_up, table_down, wsel);
        return props.a_eff.max(1e-4) * factor;
    }
    let a_up = gross_opening_area_at_low_chord(geom, table_up, geom.z_up_m, true);
    let a_down = gross_opening_area_at_low_chord(geom, table_down, geom.z_down_m, false);
    a_up.min(a_down) * factor
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
    (tail_above / head_above).clamp(0.0, 1.0)
}

/// Maximum Bradley submergence ratio over deck segments where $E_{up}$ clears the local crest.
fn max_active_weir_submergence_ratio(tw_m: f64, e_upstream: f64, geom: &BridgeGeometry) -> f64 {
    if let Some(deck) = geom.deck.as_ref().filter(|d| d.is_valid()) {
        let mut max_ratio = 0.0_f64;
        for i in 0..deck.stations_m.len().saturating_sub(1) {
            let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
            if w <= 0.0 {
                continue;
            }
            let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
            let crest = interpolate_profile(
                &deck.stations_m,
                &deck.high_elevations_m,
                s_mid,
            );
            if e_upstream > crest + 1e-6 {
                max_ratio = max_ratio.max(weir_submergence_ratio(tw_m, e_upstream, crest));
            }
        }
        max_ratio
    } else if e_upstream > geom.high_chord_m + 1e-6 {
        weir_submergence_ratio(tw_m, e_upstream, geom.high_chord_m)
    } else {
        0.0
    }
}

fn weir_submergence_exceeds_cap(tw_m: f64, e_upstream: f64, geom: &BridgeGeometry) -> bool {
    max_active_weir_submergence_ratio(tw_m, e_upstream, geom) >= geom.max_weir_submergence
}

/// Segment-wise Bradley weir overtopping (HEC-RAS effective length per crest segment).
fn segment_weir_discharge_m3s(tw_m: f64, e_upstream: f64, geom: &BridgeGeometry) -> f64 {
    if let Some(deck) = geom.deck.as_ref().filter(|d| d.is_valid()) {
        let mut q = 0.0;
        for i in 0..deck.stations_m.len().saturating_sub(1) {
            let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
            if w <= 0.0 {
                continue;
            }
            let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
            let crest = interpolate_profile(
                &deck.stations_m,
                &deck.high_elevations_m,
                s_mid,
            );
            let h = (e_upstream - crest).max(0.0);
            if h <= 1e-6 {
                continue;
            }
            let sub_ratio = weir_submergence_ratio(tw_m, e_upstream, crest);
            let factor = bradley_weir_submergence_factor(sub_ratio);
            q += geom.weir_coeff_m * factor * w * h.powf(1.5);
        }
        q
    } else {
        0.0
    }
}

fn main_pressure_flow_discharge(
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

    if tw_m >= geom.low_chord_max_m {
        let e_up = upstream_energy_grade(wsel_up, q_metric, geom, table_up, geom.z_up_m, true);
        let head = (e_up - tw_m).max(0.0);
        geom.pressure_coeff_submerged * a_net * (2.0 * G_METRIC * head).sqrt()
    } else {
        let z = opening_height_below_deck_m(geom);
        let y3 = (wsel_up - geom.z_up_m).max(1e-3);
        let cd = sluice_gate_discharge_coeff(y3 / z, geom.pressure_coeff_inlet);
        let props = obstructed_hydraulics(table_up, wsel_up, geom.z_up_m, geom, true);
        let v_head = velocity_head(q_metric, props.a_eff);
        let drive = (y3 - 0.5 * z + v_head).max(0.0);
        cd * a_net * (2.0 * G_METRIC * drive).sqrt()
    }
}

fn deck_vents_active_at_wsel(geom: &BridgeGeometry, wsel_m: f64) -> bool {
    geom.deck_vents
        .iter()
        .any(|v| wsel_m > v.invert_m + 1e-9)
}

fn deck_vent_pressure_discharge_m3s(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
) -> f64 {
    if !deck_vents_active_at_wsel(geom, wsel_up) {
        return 0.0;
    }
    let e_up = upstream_energy_grade(wsel_up, q_metric, geom, table_up, geom.z_up_m, true);
    total_deck_vent_discharge_m3s(&geom.deck_vents, wsel_up, e_up, tw_m)
}

/// High-flow discharge split: main opening under low chord, deck vents/slots, roadway weir.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct HighFlowDischargeComponents {
    pub q_opening_m3s: f64,
    pub q_vents_m3s: f64,
    pub q_weir_m3s: f64,
}

impl HighFlowDischargeComponents {
    pub fn total_m3s(self) -> f64 {
        self.q_opening_m3s + self.q_vents_m3s + self.q_weir_m3s
    }

    pub fn pressure_paths_m3s(self) -> f64 {
        self.q_opening_m3s + self.q_vents_m3s
    }
}

/// Combined high flow: $Q = Q_{opening} + Q_{vents} + Q_{weir}$.
///
/// Pass `weir_length_m = None` for pressure paths only (opening + vents).
pub(crate) fn combined_high_flow_discharge(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    a_net: f64,
    weir_length_m: Option<f64>,
) -> HighFlowDischargeComponents {
    let q_opening = if wsel_up > geom.low_chord_m + 1e-6 {
        main_pressure_flow_discharge(wsel_up, tw_m, q_metric, geom, table_up, a_net)
    } else {
        0.0
    };
    let q_vents = deck_vent_pressure_discharge_m3s(wsel_up, tw_m, q_metric, geom, table_up);
    let q_weir = weir_length_m
        .filter(|&l| l > 1e-6)
        .map(|l| weir_flow_discharge(wsel_up, tw_m, q_metric, geom, table_up, l))
        .unwrap_or(0.0);
    HighFlowDischargeComponents {
        q_opening_m3s: q_opening,
        q_vents_m3s: q_vents,
        q_weir_m3s: q_weir,
    }
}

/// Main opening pressure flow plus parallel vent/slot paths when the deck blocks the primary opening.
fn pressure_flow_discharge(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    a_net: f64,
) -> f64 {
    combined_high_flow_discharge(wsel_up, tw_m, q_metric, geom, table_up, a_net, None)
        .pressure_paths_m3s()
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
    if geom.deck.as_ref().is_some_and(|d| d.is_valid()) {
        return segment_weir_discharge_m3s(tw_m, e_up, geom);
    }
    let h_weir = (e_up - geom.high_chord_m).max(0.0);
    if h_weir <= 1e-6 {
        return 0.0;
    }
    let l = l_weir.max(1e-3);
    let sub_ratio = weir_submergence_ratio(tw_m, e_up, geom.high_chord_m);
    let factor = bradley_weir_submergence_factor(sub_ratio);
    geom.weir_coeff_m * factor * l * h_weir.powf(1.5)
}

fn solve_pressure_headwater(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let a_net = net_opening_area_at_low_chord(geom, table_up, table_down);
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

fn weir_head_active_at_energy(e_upstream: f64, geom: &BridgeGeometry) -> bool {
    if let Some(deck) = geom.deck.as_ref().filter(|d| d.is_valid()) {
        for i in 0..deck.stations_m.len().saturating_sub(1) {
            let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
            let crest = interpolate_profile(
                &deck.stations_m,
                &deck.high_elevations_m,
                s_mid,
            );
            if e_upstream > crest + 1e-6 {
                return true;
            }
        }
        false
    } else {
        e_upstream > geom.high_chord_m + 1e-6
    }
}

fn solve_bridge_headwater_metric(
    q_metric: f64,
    tw_clamped: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> BridgeHeadwaterSolve {
    if tw_clamped < geom.low_chord_m {
        solve_low_flow(q_metric, tw_clamped, geom, table_up, table_down)
    } else {
        solve_high_flow(q_metric, geom, tw_clamped, table_up, table_down)
    }
}

fn reconcile_low_flow_with_high_flow(
    q_metric: f64,
    tw_m: f64,
    wsel_low: f64,
    low_class: LowFlowClass,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> BridgeHeadwaterSolve {
    let egl = upstream_energy_grade(wsel_low, q_metric, geom, table_up, geom.z_up_m, true);
    if egl <= geom.low_chord_max_m {
        return BridgeHeadwaterSolve {
            wsel_m: wsel_low,
            regime: low_class.flow_regime(),
        };
    }
    let high = solve_high_flow(q_metric, geom, tw_m, table_up, table_down);
    if high.wsel_m > wsel_low + 1e-6 {
        high
    } else {
        BridgeHeadwaterSolve {
            wsel_m: wsel_low,
            regime: low_class.flow_regime(),
        }
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

    let use_yarnell =
        matches!(method, LowFlowMethod::Yarnell) && !active_resolved_piers(geom).is_empty();

    if use_yarnell {
        let flow_area_net = yarnell_downstream_flow_area_m2(table_down, tw_m, geom.z_down_m, geom);

        if flow_area_net > 1e-5 && q_metric > 1e-5 {
            let hl = yarnell_pier_head_loss_integrated(
                q_metric,
                tw_m,
                geom.z_down_m,
                geom,
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
) -> BridgeHeadwaterSolve {
    let class = classify_low_flow(q_metric, tw_m, geom, table_up, table_down);
    let wsel_up = match class {
        LowFlowClass::A => solve_low_flow_class_a(q_metric, tw_m, geom, table_up, table_down),
        LowFlowClass::B => solve_low_flow_class_b(q_metric, tw_m, geom, table_up, table_down),
        LowFlowClass::C => solve_low_flow_class_c(q_metric, tw_m, geom, table_up, table_down),
    };
    reconcile_low_flow_with_high_flow(
        q_metric, tw_m, wsel_up, class, geom, table_up, table_down,
    )
}

/// HEC-RAS high-flow headwater: pressure/weir (default) or explicit energy method.
fn solve_high_flow(
    q_metric: f64,
    geom: &BridgeGeometry,
    tw_clamped: f64,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> BridgeHeadwaterSolve {
    if geom.high_flow_method == HighFlowMethod::Energy {
        return BridgeHeadwaterSolve {
            wsel_m: solve_high_flow_energy(q_metric, tw_clamped, geom, table_up, table_down),
            regime: BridgeFlowRegimeKind::Energy,
        };
    }

    let a_net = net_opening_area_at_low_chord(geom, table_up, table_down);
    let fallback_weir_width = table_up.interpolate(geom.high_chord_m).top_width.max(1.0);
    let pressure_only = solve_pressure_headwater(q_metric, tw_clamped, geom, table_up, table_down);
    let e_pressure =
        upstream_energy_grade(pressure_only, q_metric, geom, table_up, geom.z_up_m, true);

    if weir_submergence_exceeds_cap(tw_clamped, e_pressure, geom) {
        return BridgeHeadwaterSolve {
            wsel_m: solve_high_flow_energy_fallback(
                q_metric,
                tw_clamped,
                geom,
                table_up,
                table_down,
            ),
            regime: BridgeFlowRegimeKind::Energy,
        };
    }

    let q_weir_at_pressure = weir_flow_discharge(
        pressure_only,
        tw_clamped,
        q_metric,
        geom,
        table_up,
        fallback_weir_width,
    );
    if q_weir_at_pressure <= 1e-9 {
        return BridgeHeadwaterSolve {
            wsel_m: pressure_only,
            regime: BridgeFlowRegimeKind::Pressure,
        };
    }

    let combined_q_at = |h_up: f64| -> f64 {
        let e_up = upstream_energy_grade(h_up, q_metric, geom, table_up, geom.z_up_m, true);
        let l_weir = effective_weir_length_m(geom, e_up, fallback_weir_width);
        combined_high_flow_discharge(
            h_up,
            tw_clamped,
            q_metric,
            geom,
            table_up,
            a_net,
            Some(l_weir),
        )
        .total_m3s()
    };

    let mut low = tw_clamped.max(geom.z_up_m + 1e-4);
    let mut high = pressure_only.max(geom.high_chord_m).max(low + 1e-3);
    if combined_q_at(high) < q_metric {
        high = high + 50.0;
    } else if combined_q_at(low) > q_metric {
        // Weir adds capacity below the pressure-only headwater.
        high = pressure_only;
    }

    let residual = |h_up: f64| -> f64 {
        if weir_submergence_exceeds_cap(
            tw_clamped,
            upstream_energy_grade(h_up, q_metric, geom, table_up, geom.z_up_m, true),
            geom,
        ) {
            return -1.0;
        }
        combined_q_at(h_up) - q_metric
    };

    let mut best_h = high;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let e_up = upstream_energy_grade(mid, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_exceeds_cap(tw_clamped, e_up, geom) {
            return BridgeHeadwaterSolve {
                wsel_m: solve_high_flow_energy_fallback(
                    q_metric,
                    tw_clamped,
                    geom,
                    table_up,
                    table_down,
                ),
                regime: BridgeFlowRegimeKind::Energy,
            };
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

    let e_best = upstream_energy_grade(best_h, q_metric, geom, table_up, geom.z_up_m, true);
    if weir_submergence_exceeds_cap(tw_clamped, e_best, geom) {
        return BridgeHeadwaterSolve {
            wsel_m: solve_high_flow_energy_fallback(
                q_metric,
                tw_clamped,
                geom,
                table_up,
                table_down,
            ),
            regime: BridgeFlowRegimeKind::Energy,
        };
    }

    let l_weir = effective_weir_length_m(geom, e_best, fallback_weir_width);
    let parts = combined_high_flow_discharge(
        best_h,
        tw_clamped,
        q_metric,
        geom,
        table_up,
        a_net,
        Some(l_weir),
    );
    let regime = if parts.q_weir_m3s > 1e-6 {
        BridgeFlowRegimeKind::Weir
    } else {
        BridgeFlowRegimeKind::Pressure
    };
    BridgeHeadwaterSolve {
        wsel_m: best_h,
        regime,
    }
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

#[allow(dead_code)]
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
    let solved = solve_bridge_headwater_metric(
        q_metric,
        tw_clamped,
        &geom,
        table_up,
        table_down,
    );
    let wsel_up = if units == UnitSystem::USCustomary {
        solved.wsel_m / FT_TO_M
    } else {
        solved.wsel_m
    };
    BridgeSolveResult {
        wsel_up,
        wsel_down: tw_wsel,
        head_loss: (wsel_up - tw_wsel).max(0.0),
        flow_regime: solved.regime.as_str().to_string(),
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
        ineffective_flow_areas: None,
    guide_banks: None,
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
    let interior = crate::solvers::bridge_interior::BridgeInteriorInput {
        bu: params.xs_up.clone(),
        bd: params.xs_down.clone(),
        internal: params.xs_internal.clone().unwrap_or_default(),
        opening_reach_station_origin: params.opening_reach_station_origin,
        ..Default::default()
    };
    let from_faces = crate::solvers::bridge_interior::resolve_bridge_friction_length_metric(
        &interior,
        0.0,
        params.length,
        params.units,
    );
    if from_faces > 1e-3 {
        return from_faces;
    }
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
    let mut params = params.clone();
    crate::solvers::bridge_roadway_compose::apply_roadway_embankment_compose_params(&mut params);
    let (_table_up, _table_down, mut xs_up, mut xs_down) = geometry_tables_from_params(&params);
    let opening_origin = params
        .opening_reach_station_origin
        .or_else(|| {
            Some(crate::solvers::bridge_interior::infer_opening_reach_station_origin(
                &xs_up,
            ))
        });
    if let Some(blocked) = params.composed_embankment_blocked.as_ref() {
        crate::solvers::bridge_roadway_compose::merge_embankment_blocked_into_section(
            &mut xs_up,
            blocked.left.as_ref(),
            blocked.right.as_ref(),
            opening_origin,
        );
        crate::solvers::bridge_roadway_compose::merge_embankment_blocked_into_section(
            &mut xs_down,
            blocked.left.as_ref(),
            blocked.right.as_ref(),
            opening_origin,
        );
    }
    let (table_up, table_down) = {
        let up_metric = xs_up.to_metric();
        let down_metric = xs_down.to_metric();
        (
            up_metric.generate_lookup_table(params.num_slices),
            down_metric.generate_lookup_table(params.num_slices),
        )
    };
    let coupling = coupling_from_params(&params);
    let deck = build_bridge_deck_profile(
        params.low_chord,
        params.high_chord,
        params.deck_stations.as_deref(),
        params.deck_low_elevations.as_deref(),
        params.deck_high_elevations.as_deref(),
        params.units,
    );
    let interior = crate::solvers::bridge_interior::BridgeInteriorInput {
        bu: Some(xs_up.clone()),
        bd: Some(xs_down.clone()),
        internal: params.xs_internal.clone().unwrap_or_default(),
        opening_reach_station_origin: opening_origin,
        ..Default::default()
    };
    let friction_length_m = crate::solvers::bridge_interior::resolve_bridge_friction_length_metric(
        &interior,
        0.0,
        params.length,
        params.units,
    );
    let sections = BridgeSectionContext {
        ineffective_up: ineffective_upstream_from_params(&params),
        ineffective_down: ineffective_downstream_from_params(&params),
        xs_up: Some(xs_up),
        xs_down: Some(xs_down),
        internal_xs: interior.internal,
        opening_reach_station_origin: opening_origin,
        skew_deg: params.skew_deg,
        pier_stations: params.pier_stations.clone(),
        pier_widths: crate::solvers::pier_geometry::pier_width_user_from_rating_params(
            &params.pier_top_widths,
            &params.pier_bottom_widths,
            &params.pier_width_elevations,
            &params.pier_width_values,
            &params.pier_top_elevations,
            &params.pier_base_elevations,
        ),
        pier_attachments: crate::solvers::pier_geometry::pier_attachments_from_rating_params(
            &params.pier_footing_top_elevations,
            &params.pier_footing_widths,
            &params.pier_footing_bottom_elevations,
            &params.pier_nosing_lengths,
            &params.pier_nosing_widths,
        ),
        deck_vents: crate::solvers::deck_vent_geometry::deck_vents_from_rating_params(
            &params.deck_vent_left_stations,
            &params.deck_vent_right_stations,
            &params.deck_vent_stations,
            &params.deck_vent_widths,
            &params.deck_vent_invert_elevations,
            &params.deck_vent_soffit_elevations,
            &params.deck_vent_discharge_coefficients,
            &params.deck_vent_types,
        ),
        friction_length_m,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
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
        interval_length_metric(&params),
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

    let solved = solve_bridge_headwater_metric(
        q_metric,
        tw_clamped,
        &geom,
        table_up,
        table_down,
    );

    if units == UnitSystem::USCustomary {
        solved.wsel_m / FT_TO_M
    } else {
        solved.wsel_m
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

fn pier_width_user_to_metric(user: &PierWidthUserInput, units: UnitSystem) -> PierWidthUserInput {
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let to_m_vec = |v: &Option<Vec<f64>>| v.as_ref().map(|xs| xs.iter().map(|x| to_m(*x)).collect());
    let to_m_mat = |v: &Option<Vec<Vec<f64>>>| {
        v.as_ref().map(|rows| {
            rows.iter()
                .map(|row| row.iter().map(|x| to_m(*x)).collect())
                .collect()
        })
    };
    PierWidthUserInput {
        top_widths: to_m_vec(&user.top_widths),
        bottom_widths: to_m_vec(&user.bottom_widths),
        width_elevations: to_m_mat(&user.width_elevations),
        width_values: to_m_mat(&user.width_values),
        top_elevations: to_m_vec(&user.top_elevations),
        base_elevations: to_m_vec(&user.base_elevations),
    }
}

fn pier_attachments_user_to_metric(
    user: &PierAttachmentsUserInput,
    units: UnitSystem,
) -> PierAttachmentsUserInput {
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let to_m_vec = |v: &Option<Vec<f64>>| v.as_ref().map(|xs| xs.iter().map(|x| to_m(*x)).collect());
    PierAttachmentsUserInput {
        footing_top_elevations: to_m_vec(&user.footing_top_elevations),
        footing_widths: to_m_vec(&user.footing_widths),
        footing_bottom_elevations: to_m_vec(&user.footing_bottom_elevations),
        nosing_lengths: to_m_vec(&user.nosing_lengths),
        nosing_widths: to_m_vec(&user.nosing_widths),
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
    let length_base_m = sections
        .map(|s| s.friction_length_m)
        .filter(|&l| l > 1e-3)
        .unwrap_or_else(|| {
            if interval_length > 1e-3 {
                interval_length
            } else if coupling.length > 1e-6 {
                if units == UnitSystem::USCustomary {
                    coupling.length * FT_TO_M
                } else {
                    coupling.length
                }
            } else {
                10.0
            }
        });
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

    let opening_origin_user = sections.and_then(|s| s.opening_reach_station_origin);
    let deck_owned = deck.cloned().map(|mut d| {
        if let Some(origin) = opening_origin_user {
            d.remap_stations_to_reach(origin, units);
        }
        d
    });
    let abutment_input = opening_origin_user
        .map(|origin| {
            crate::solvers::bridge_abutment::remap_abutment_input_to_reach(&coupling.abutment, origin)
        })
        .unwrap_or_else(|| coupling.abutment.clone());
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
    let to_metric_xs = |xs: CrossSection| {
        if units == UnitSystem::USCustomary {
            xs.to_metric()
        } else {
            xs
        }
    };
    let to_metric_guide = |gb: &GuideBanks| {
        if units == UnitSystem::USCustomary {
            gb.to_metric(UnitSystem::USCustomary)
        } else {
            gb.clone()
        }
    };
    let xs_approach = sections
        .and_then(|s| s.xs_approach.clone())
        .map(to_metric_xs);
    let xs_departure = sections
        .and_then(|s| s.xs_departure.clone())
        .map(to_metric_xs);
    let guide_banks_approach = sections
        .and_then(|s| s.guide_banks_approach.as_ref())
        .filter(|g| g.is_configured())
        .map(to_metric_guide);
    let guide_banks_departure = sections
        .and_then(|s| s.guide_banks_departure.as_ref())
        .filter(|g| g.is_configured())
        .map(to_metric_guide);
    let table_approach = xs_approach
        .as_ref()
        .map(|xs| xs.generate_lookup_table(APPROACH_DEPARTURE_TABLE_SLICES));
    let table_departure = xs_departure
        .as_ref()
        .map(|xs| xs.generate_lookup_table(APPROACH_DEPARTURE_TABLE_SLICES));

    let (opening_s_min, opening_s_max) = opening_station_bounds_from_deck(deck_owned.as_ref());
    let abutments = resolve_abutments(&abutment_input, opening_s_min, opening_s_max, skew_cos, units);

    let pier_width_perp_m = if units == UnitSystem::USCustomary {
        pier_width * FT_TO_M
    } else {
        pier_width
    };
    let z_up_m = if units == UnitSystem::USCustomary {
        z_up * FT_TO_M
    } else {
        z_up
    };
    let z_down_m = if units == UnitSystem::USCustomary {
        z_down * FT_TO_M
    } else {
        z_down
    };
    let pier_width_user = sections
        .and_then(|s| s.pier_widths.as_ref())
        .map(|u| pier_width_user_to_metric(u, units));
    let pier_attachments_user = sections
        .and_then(|s| s.pier_attachments.as_ref())
        .map(|u| pier_attachments_user_to_metric(u, units));
    let inset = pier_width_perp_m.max(0.0) * 0.5;
    let pier_station_list = if !pier_stations_m.is_empty() {
        pier_stations_m.clone()
    } else {
        evenly_spaced_pier_stations(num_piers, opening_s_min, opening_s_max, inset)
    };
    let z_bed_m = z_up_m.min(z_down_m);
    let z_top_defaults: Vec<f64> = pier_station_list
        .iter()
        .map(|&s| {
            deck_owned
                .as_ref()
                .map(|d| interpolate_profile(&d.stations_m, &d.low_elevations_m, s))
                .unwrap_or(low_min)
        })
        .collect();
    let pier_specs = resolve_pier_width_specs(
        pier_width_perp_m,
        &pier_station_list,
        z_bed_m,
        &z_top_defaults,
        pier_width_user.as_ref(),
        pier_attachments_user.as_ref(),
    );

    let deck_vents = sections
        .and_then(|s| s.deck_vents.as_ref())
        .map(|u| resolve_deck_vents(u, skew_cos, units, submerged_c))
        .unwrap_or_default();

    if units == UnitSystem::USCustomary {
        BridgeGeometry {
            low_chord_m: low_min,
            low_chord_max_m: low_max,
            high_chord_m: high_min,
            high_chord_max_m: high_max,
            pier_width_m: pier_width_perp_m,
            num_piers,
            pier_stations_m: pier_stations_m.clone(),
            pier_specs: pier_specs.clone(),
            skew_deg,
            skew_cos,
            pier_shape: PierShape::from_i32(pier_shape_type),
            abutments: abutments.clone(),
            weir_coeff_m: weir_coeff / 1.8113,
            orifice_coeff: submerged_c,
            z_up_m,
            z_down_m,
            low_flow_method: LowFlowMethod::from_i32(coupling.low_flow_method),
            high_flow_method: HighFlowMethod::from_i32(coupling.high_flow_method),
            length_m,
            wspro_coeff_c: coupling.wspro_coeff,
            coeff_contraction: coupling.coeff_contraction,
            coeff_expansion: coupling.coeff_expansion,
            pressure_coeff_inlet: coupling.pressure_coeff_inlet,
            pressure_coeff_submerged: submerged_c,
            max_weir_submergence: coupling.max_weir_submergence,
            deck: deck_owned.clone(),
            ineffective_up: ineffective_up.clone(),
            ineffective_down: ineffective_down.clone(),
            xs_up: xs_up.clone(),
            xs_down: xs_down.clone(),
            xs_approach: xs_approach.clone(),
            xs_departure: xs_departure.clone(),
            guide_banks_approach: guide_banks_approach.clone(),
            guide_banks_departure: guide_banks_departure.clone(),
            table_approach: table_approach.clone(),
            table_departure: table_departure.clone(),
            deck_vents: deck_vents.clone(),
        }
    } else {
        BridgeGeometry {
            low_chord_m: low_min,
            low_chord_max_m: low_max,
            high_chord_m: high_min,
            high_chord_max_m: high_max,
            pier_width_m: pier_width_perp_m,
            num_piers,
            pier_stations_m,
            pier_specs,
            skew_deg,
            skew_cos,
            pier_shape: PierShape::from_i32(pier_shape_type),
            abutments,
            weir_coeff_m: weir_coeff,
            orifice_coeff: submerged_c,
            z_up_m,
            z_down_m,
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
            xs_approach,
            xs_departure,
            guide_banks_approach,
            guide_banks_departure,
            table_approach,
            table_departure,
            deck_vents,
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
        let hw_calc = solve_bridge_headwater_metric(q_metric, mid, geom, table_up, table_down).wsel_m;
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

    let a_net = net_opening_area_at_low_chord(geom, table_up, table_down);
    let fallback_weir_width = table_down.interpolate(geom.high_chord_m).top_width.max(1.0);
    let e_up = upstream_energy_grade(hw_m, q_metric, geom, table_up, geom.z_up_m, true);

    if !weir_head_active_at_energy(e_up, geom) {
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

    let residual = |tw: f64| -> f64 {
        let e_up = upstream_energy_grade(hw_m, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_exceeds_cap(tw, e_up, geom) {
            return -1.0;
        }
        let l_weir = effective_weir_length_m(geom, e_up, fallback_weir_width);
        combined_high_flow_discharge(
            hw_m,
            tw,
            q_metric,
            geom,
            table_up,
            a_net,
            Some(l_weir),
        )
        .total_m3s()
            - q_metric
    };

    let mut low = geom.z_down_m + 1e-4;
    let mut high = hw_m;
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let e_up = upstream_energy_grade(hw_m, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_exceeds_cap(mid, e_up, geom) {
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
#[path = "bridge_tests.rs"]
mod tests;
