use crate::geometry::{CrossSection, GuideBanks, IneffectiveFlowAreas};
use crate::solvers::deck_vent_geometry::DeckVentUserInput;
use crate::solvers::pier_geometry::{PierAttachmentsUserInput, PierWidthUserInput};
use crate::solvers::culvert::apply_barrel_skew;
use crate::utils::{UnitSystem, CFS_TO_CMS};

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
    /// Opening / approach / departure friction segments (metric, before skew in `friction_length_m` only).
    pub friction_lengths: BridgeFrictionLengths,
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

/// How energy / WSPRO friction reach is split between the bridge opening and approach/departure (API v30).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum BridgeFrictionWeighting {
    /// Friction loss uses BU→BD opening length only (legacy default).
    #[default]
    OpeningOnly = 0,
    /// HEC-RAS three-segment friction: approach→BU + BU→BD + BD→departure.
    HecRasSegments = 1,
}

impl BridgeFrictionWeighting {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::HecRasSegments,
            _ => Self::OpeningOnly,
        }
    }
}

/// Metric friction reach segments for one bridge interval.
#[derive(Debug, Clone, Copy)]
pub struct BridgeFrictionLengths {
    pub weighting: BridgeFrictionWeighting,
    pub opening_m: f64,
    pub approach_m: f64,
    pub departure_m: f64,
}

impl Default for BridgeFrictionLengths {
    fn default() -> Self {
        Self {
            weighting: BridgeFrictionWeighting::OpeningOnly,
            opening_m: 0.0,
            approach_m: 0.0,
            departure_m: 0.0,
        }
    }
}

/// Reach discharge sign: downstream (+) vs upstream (−) per steady/unsteady `Q` convention.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BridgeFlowDirection {
    Downstream = 1,
    Upstream = -1,
}

impl BridgeFlowDirection {
    pub fn from_q(q: f64) -> Self {
        if q < -1e-12 {
            Self::Upstream
        } else {
            Self::Downstream
        }
    }
}

/// Mirror approach/departure and face ineffective for reverse-flow hydraulics (BU/BD reach labels unchanged).
pub fn mirror_bridge_section_context(ctx: &BridgeSectionContext) -> BridgeSectionContext {
    let mut mirrored = ctx.clone();
    std::mem::swap(&mut mirrored.ineffective_up, &mut mirrored.ineffective_down);
    std::mem::swap(&mut mirrored.xs_approach, &mut mirrored.xs_departure);
    std::mem::swap(&mut mirrored.guide_banks_approach, &mut mirrored.guide_banks_departure);
    mirrored.friction_lengths = BridgeFrictionLengths {
        weighting: ctx.friction_lengths.weighting,
        opening_m: ctx.friction_lengths.opening_m,
        approach_m: ctx.friction_lengths.departure_m,
        departure_m: ctx.friction_lengths.approach_m,
    };
    mirrored
}

pub(crate) fn bridge_q_to_metric_magnitude(q_user: f64, units: UnitSystem) -> f64 {
    let q = q_user.abs();
    if units == UnitSystem::USCustomary {
        q * CFS_TO_CMS
    } else {
        q
    }
}

pub(crate) fn hydraulic_hw_tw_reach(
    direction: BridgeFlowDirection,
    wsel_bu: f64,
    wsel_bd: f64,
) -> (f64, f64) {
    match direction {
        BridgeFlowDirection::Downstream => (wsel_bu, wsel_bd),
        BridgeFlowDirection::Upstream => (wsel_bd, wsel_bu),
    }
}

/// HEC-RAS-style bridge skew: projected opening width × cos(θ), friction length ÷ cos(θ).
pub fn apply_bridge_skew(skew_deg: f64, width_m: f64, length_m: f64) -> (f64, f64) {
    apply_barrel_skew(skew_deg, width_m, length_m)
}
