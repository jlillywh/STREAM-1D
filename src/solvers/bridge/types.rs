use crate::geometry::CrossSection;
use crate::solvers::bridge_abutment::BridgeAbutmentUserInput;
use crate::utils::UnitSystem;

use super::ice_debris::BridgeIceDebrisParams;
use super::section::BridgeFrictionWeighting;


/// Supported pier shape types (Yarnell $K$ and momentum $C_D$ per HEC-RAS 6.x low-flow tables).
///
/// Values `0`–`3` are unchanged from API v1. Values `4`–`11` added in API v29
/// (`docs/development/bridge_extensions.md`).
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
    pub(crate) fn flow_regime(self) -> BridgeFlowRegimeKind {
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
    /// Friction weighting between opening and approach/departure reaches (API v30).
    pub friction_weighting: BridgeFrictionWeighting,
    /// Override approach friction length (user units). 0 = auto from river stations.
    pub approach_friction_length: f64,
    /// Override departure friction length (user units). 0 = auto from river stations.
    pub departure_friction_length: f64,
    /// Floating pier debris, ice cover, deck ice, and opening blockage factor.
    pub ice_debris: BridgeIceDebrisParams,
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
            friction_weighting: BridgeFrictionWeighting::OpeningOnly,
            approach_friction_length: 0.0,
            departure_friction_length: 0.0,
            ice_debris: BridgeIceDebrisParams::default(),
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
    /// Fixed downstream tailwater for subcritical rating when `q > 0` (BD face, user units).
    pub tw_wsel: f64,
    /// Tailwater at BU when `q < 0`. Omit to reuse `tw_wsel`.
    #[serde(default)]
    pub tw_wsel_reverse: Option<f64>,
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
    /// Friction weighting: 0 = opening only, 1 = HEC-RAS approach + opening + departure segments.
    #[serde(default)]
    pub friction_weighting: i32,
    /// Override approach friction length (user units). 0 = auto from river stations.
    #[serde(default)]
    pub approach_friction_length: f64,
    /// Override departure friction length (user units). 0 = auto from river stations.
    #[serde(default)]
    pub departure_friction_length: f64,
    /// Opening area / conveyance multiplier (0–1]. Omit or `1.0` = no extra blockage.
    #[serde(default = "default_opening_blockage_factor")]
    pub opening_blockage_factor: f64,
    /// Per-pier floating debris total width in opening coordinates (user units).
    #[serde(default)]
    pub pier_debris_widths: Option<Vec<f64>>,
    /// Per-pier floating debris height below WSEL (user units).
    #[serde(default)]
    pub pier_debris_heights: Option<Vec<f64>>,
    /// Constant ice thickness through opening (user units).
    #[serde(default)]
    pub ice_thickness: f64,
    /// `0` = none, `1` = constant thickness, `2` = reserved.
    #[serde(default)]
    pub ice_mode: i32,
    /// Roadway ice lowering weir crest (user units).
    #[serde(default)]
    pub deck_ice_thickness: f64,
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

fn default_opening_blockage_factor() -> f64 {
    1.0
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
            tw_wsel_reverse: None,
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
            friction_weighting: 0,
            approach_friction_length: 0.0,
            departure_friction_length: 0.0,
            opening_blockage_factor: default_opening_blockage_factor(),
            pier_debris_widths: None,
            pier_debris_heights: None,
            ice_thickness: 0.0,
            ice_mode: 0,
            deck_ice_thickness: 0.0,
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
