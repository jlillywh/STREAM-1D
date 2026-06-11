//! HEC-RAS BU / BD / internal bridge cross-section resolution (API v22).

use crate::geometry::{
    apply_reach_modifier_policy, interpolate_cross_section, resolve_guide_banks, CrossSection,
    DensifyReachModifierPolicy, GeometryTable, GuideBanks, IneffectiveBlock, IneffectiveFlowAreas,
};
use crate::solvers::bridge::{
    BridgeFrictionLengths, BridgeFrictionWeighting, BridgeSectionContext,
};
use crate::utils::{structure_in_reach_interval, UnitSystem, FT_TO_M, STRUCTURE_STATION_TOL};

fn ineffective_from_cross_section(
    xs: &CrossSection,
    raw_units: UnitSystem,
) -> Option<IneffectiveFlowAreas> {
    let areas = xs
        .ineffective_flow_areas
        .as_ref()
        .filter(|i| i.is_configured())?;
    Some(areas.convert_units(xs.unit_system, raw_units))
}

fn shift_ineffective_stations(
    areas: IneffectiveFlowAreas,
    station_offset: f64,
) -> IneffectiveFlowAreas {
    let shift = |b: IneffectiveBlock| IneffectiveBlock {
        station: b.station + station_offset,
        elevation: b.elevation,
    };
    IneffectiveFlowAreas {
        left_blocks: areas.left_blocks.iter().copied().map(shift).collect(),
        right_blocks: areas.right_blocks.iter().copied().map(shift).collect(),
    }
}

/// Map bridge opening-frame ineffective blocks onto reach lateral coordinates.
fn opening_frame_ineffective_to_reach(
    areas: Option<IneffectiveFlowAreas>,
    opening_origin: Option<f64>,
) -> Option<IneffectiveFlowAreas> {
    let areas = areas.filter(|i| i.is_configured())?;
    let origin = opening_origin?;
    Some(shift_ineffective_stations(areas, origin))
}

/// Resolve ineffective blocks for one bridge face (BU or BD).
///
/// Explicit BU/BD cuts use `ineffective_flow_areas` on that section when present; they do not
/// inherit ineffective blocks from the adjacent reach face. When the explicit cut omits
/// section ineffective areas, bridge-level `bridge_ineffective_*` fields apply (opening frame,
/// shifted by `opening_origin` when known).
fn resolve_face_ineffective(
    face_xs: Option<&CrossSection>,
    is_explicit_face: bool,
    reach_xs: Option<&CrossSection>,
    bridge_level: Option<IneffectiveFlowAreas>,
    opening_origin: Option<f64>,
    raw_units: UnitSystem,
) -> Option<IneffectiveFlowAreas> {
    if is_explicit_face {
        if let Some(xs) = face_xs {
            if let Some(from_xs) = ineffective_from_cross_section(xs, raw_units) {
                return Some(from_xs);
            }
        }
        return opening_frame_ineffective_to_reach(bridge_level, opening_origin);
    }

    if let Some(from_reach) = reach_xs.and_then(|xs| ineffective_from_cross_section(xs, raw_units))
    {
        return Some(from_reach);
    }
    opening_frame_ineffective_to_reach(bridge_level, opening_origin)
}

/// How bridge opening station 0 is anchored to reach lateral coordinates (API v23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BridgeOpeningAnchorMode {
    /// Opening station 0 = leftmost lateral `x` on resolved BU (or reach US fallback).
    #[default]
    BuLeft = 0,
    /// Opening station 0 = leftmost lateral `x` on the reach XS at `opening_anchor_reach_station`.
    ReachRiverStation = 1,
    /// Opening station 0 = explicit reach lateral `x` in `opening_reach_station_origin`.
    ReachLateralX = 2,
}

impl BridgeOpeningAnchorMode {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::ReachRiverStation,
            2 => Self::ReachLateralX,
            _ => Self::BuLeft,
        }
    }
}

/// Per-bridge optional interior section inputs (steady or unsteady).
#[derive(Debug, Clone, Default)]
pub struct BridgeInteriorInput {
    pub bu: Option<CrossSection>,
    pub bd: Option<CrossSection>,
    pub internal: Vec<CrossSection>,
    pub opening_reach_station_origin: Option<f64>,
    pub opening_anchor_mode: Option<BridgeOpeningAnchorMode>,
    /// Longitudinal reach river station (user units) when `opening_anchor_mode` is `ReachRiverStation`.
    pub opening_anchor_reach_station: Option<f64>,
    /// Explicit approach (upstream) cross section per bridge.
    pub approach: Option<CrossSection>,
    /// Explicit departure (downstream exit) cross section per bridge.
    pub departure: Option<CrossSection>,
    /// Reach river station of approach cut when `approach` is omitted.
    pub approach_reach_station: Option<f64>,
    /// Reach river station of departure cut when `departure` is omitted.
    pub departure_reach_station: Option<f64>,
    /// Guide banks on approach cut when not on embedded `CrossSection.guide_banks`.
    pub approach_guide_banks: Option<GuideBanks>,
    /// Guide banks on departure cut when not on embedded `CrossSection.guide_banks`.
    pub departure_guide_banks: Option<GuideBanks>,
}

/// Resolved face geometry for one bridge interval solve.
#[derive(Debug, Clone)]
pub struct BridgeFaceSolveGeometry {
    pub table_up: GeometryTable,
    pub table_down: GeometryTable,
    pub sections: BridgeSectionContext,
    /// Minimum bed elevation at BU (user units).
    pub z_up_user: f64,
    /// Minimum bed elevation at BD (user units).
    pub z_down_user: f64,
}

/// Named inputs for [`resolve_bridge_face_solve_geometry`] (steady/unsteady bridge interval).
#[derive(Debug)]
pub struct BridgeFaceSolveParams<'a> {
    pub interior: &'a BridgeInteriorInput,
    pub anchor_reach_xs: Option<&'a CrossSection>,
    pub reach_xs_up: Option<&'a CrossSection>,
    pub reach_xs_down: Option<&'a CrossSection>,
    pub reach_table_up: &'a GeometryTable,
    pub reach_table_down: &'a GeometryTable,
    pub reach_z_up_user: f64,
    pub reach_z_down_user: f64,
    pub raw_units: UnitSystem,
    pub num_slices: usize,
    pub ineffective_up: Option<IneffectiveFlowAreas>,
    pub ineffective_down: Option<IneffectiveFlowAreas>,
    pub skew_deg: f64,
    pub pier_stations: Option<Vec<f64>>,
    pub interval_length_m: f64,
    pub bridge_length_user: f64,
    /// Friction weighting: 0 = opening only, 1 = HEC-RAS approach + opening + departure.
    pub friction_weighting: BridgeFrictionWeighting,
    /// Override approach friction length (user units). 0 = auto from river stations.
    pub approach_friction_length_user: f64,
    /// Override departure friction length (user units). 0 = auto from river stations.
    pub departure_friction_length_user: f64,
    pub approach_xs: Option<CrossSection>,
    pub departure_xs: Option<CrossSection>,
    pub guide_banks_approach: Option<GuideBanks>,
    pub guide_banks_departure: Option<GuideBanks>,
    pub pier_widths: Option<crate::solvers::pier_geometry::PierWidthUserInput>,
    pub pier_attachments: Option<crate::solvers::pier_geometry::PierAttachmentsUserInput>,
    pub deck_vents: Option<crate::solvers::deck_vent_geometry::DeckVentUserInput>,
    pub embankment_blocked: Option<&'a crate::solvers::bridge_roadway_compose::ComposedEmbankmentBlocked>,
}

impl<'a> BridgeFaceSolveParams<'a> {
    /// Minimal constructor; optional fields default to unset / zero.
    pub fn new(
        interior: &'a BridgeInteriorInput,
        reach_table_up: &'a GeometryTable,
        reach_table_down: &'a GeometryTable,
    ) -> Self {
        Self {
            interior,
            anchor_reach_xs: None,
            reach_xs_up: None,
            reach_xs_down: None,
            reach_table_up,
            reach_table_down,
            reach_z_up_user: 0.0,
            reach_z_down_user: 0.0,
            raw_units: UnitSystem::Metric,
            num_slices: 50,
            ineffective_up: None,
            ineffective_down: None,
            skew_deg: 0.0,
            pier_stations: None,
            interval_length_m: 0.0,
            bridge_length_user: 0.0,
            friction_weighting: BridgeFrictionWeighting::OpeningOnly,
            approach_friction_length_user: 0.0,
            departure_friction_length_user: 0.0,
            approach_xs: None,
            departure_xs: None,
            guide_banks_approach: None,
            guide_banks_departure: None,
            pier_widths: None,
            pier_attachments: None,
            deck_vents: None,
            embankment_blocked: None,
        }
    }
}

/// Minimum ground elevation from a cross-section polyline (user units of the section).
pub fn cross_section_min_bed(xs: &CrossSection) -> f64 {
    xs.y.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Infer reach XS lateral coordinate at bridge opening station 0 (leftmost polyline `x`).
pub fn infer_opening_reach_station_origin(xs: &CrossSection) -> f64 {
    xs.x.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Resolve reach lateral `x` at opening station 0 from anchor mode and optional explicit override.
///
/// Precedence: explicit `opening_reach_station_origin` (backward compatible with v22) wins over mode.
pub fn resolve_opening_reach_station_origin(
    explicit_lateral_x: Option<f64>,
    anchor_mode: Option<BridgeOpeningAnchorMode>,
    bu_xs: Option<&CrossSection>,
    anchor_reach_xs: Option<&CrossSection>,
    reach_xs_up: Option<&CrossSection>,
) -> Option<f64> {
    if let Some(x) = explicit_lateral_x {
        return Some(x);
    }
    match anchor_mode.unwrap_or(BridgeOpeningAnchorMode::BuLeft) {
        BridgeOpeningAnchorMode::ReachLateralX => None,
        BridgeOpeningAnchorMode::ReachRiverStation => anchor_reach_xs
            .or(reach_xs_up)
            .map(infer_opening_reach_station_origin),
        BridgeOpeningAnchorMode::BuLeft => bu_xs
            .or(reach_xs_up)
            .map(infer_opening_reach_station_origin),
    }
}

/// Look up a reach cross section at a longitudinal river station on a densified grid.
pub fn cross_section_at_reach_station(
    stations_metric: &[f64],
    xs: &[Option<CrossSection>],
    station_user: f64,
    raw_units: UnitSystem,
) -> Option<CrossSection> {
    let target_m = user_length_to_metric(station_user, raw_units);
    let idx = stations_metric
        .iter()
        .position(|&s| (s - target_m).abs() <= STRUCTURE_STATION_TOL)?;
    xs.get(idx).and_then(|opt| opt.clone())
}

/// Same as [`cross_section_at_reach_station`] for unsteady grids where every node has an XS.
pub fn cross_section_at_reach_station_dense(
    stations_metric: &[f64],
    xs: &[CrossSection],
    station_user: f64,
    raw_units: UnitSystem,
) -> Option<CrossSection> {
    let target_m = user_length_to_metric(station_user, raw_units);
    let idx = stations_metric
        .iter()
        .position(|&s| (s - target_m).abs() <= STRUCTURE_STATION_TOL)?;
    xs.get(idx).cloned()
}

/// Map bridge opening station to reach cross-section lateral `x`.
pub fn opening_station_to_reach_x(opening_s: f64, origin: f64) -> f64 {
    origin + opening_s
}

/// Map reach cross-section lateral `x` to bridge opening station.
pub fn reach_x_to_opening_station(reach_x: f64, origin: f64) -> f64 {
    reach_x - origin
}

/// Shift opening-frame lateral stations to reach XS coordinates (user units).
pub fn remap_opening_stations_user(stations: &[f64], origin_user: f64) -> Vec<f64> {
    stations
        .iter()
        .map(|&s| opening_station_to_reach_x(s, origin_user))
        .collect()
}

/// Shift optional opening-frame stations when a reach origin is known.
pub fn remap_opening_stations_option(
    stations: Option<Vec<f64>>,
    origin_user: Option<f64>,
) -> Option<Vec<f64>> {
    match (stations, origin_user) {
        (Some(st), Some(origin)) => Some(remap_opening_stations_user(&st, origin)),
        (st, _) => st,
    }
}

/// Map opening-frame ineffective blocks onto reach lateral coordinates (public preprocessor).
pub fn remap_ineffective_opening_to_reach(
    areas: Option<IneffectiveFlowAreas>,
    origin_user: f64,
) -> Option<IneffectiveFlowAreas> {
    opening_frame_ineffective_to_reach(areas, Some(origin_user))
}

fn bed_user_from_xs(xs: &CrossSection, raw_units: UnitSystem) -> f64 {
    let bed = cross_section_min_bed(xs);
    if raw_units == UnitSystem::USCustomary && xs.unit_system == UnitSystem::Metric {
        bed / FT_TO_M
    } else if raw_units == UnitSystem::Metric && xs.unit_system == UnitSystem::USCustomary {
        bed * FT_TO_M
    } else {
        bed
    }
}

fn table_from_xs(xs: &CrossSection, num_slices: usize) -> GeometryTable {
    xs.to_metric().generate_lookup_table(num_slices)
}

/// Sum of reach segments along explicit BU → internal → BD river stations (metric).
pub fn friction_path_from_interior(
    interior: &BridgeInteriorInput,
    raw_units: UnitSystem,
) -> Option<f64> {
    let has_explicit = interior.bu.is_some()
        || interior.bd.is_some()
        || !interior.internal.is_empty();
    if !has_explicit {
        return None;
    }

    let mut stations = Vec::new();
    if let Some(xs) = &interior.bu {
        stations.push(xs_river_station_to_metric(
            xs.station,
            xs.unit_system,
            raw_units,
        ));
    }
    for xs in &interior.internal {
        stations.push(xs_river_station_to_metric(
            xs.station,
            xs.unit_system,
            raw_units,
        ));
    }
    if let Some(xs) = &interior.bd {
        stations.push(xs_river_station_to_metric(
            xs.station,
            xs.unit_system,
            raw_units,
        ));
    }
    if stations.len() < 2 {
        return None;
    }
    stations.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let path: f64 = stations
        .windows(2)
        .map(|w| (w[0] - w[1]).abs())
        .sum();
    if path > STRUCTURE_STATION_TOL {
        Some(path)
    } else {
        None
    }
}

/// HEC-RAS bridge friction reach length (metric): BU–BD path, densified interval, then `bridge_lengths`.
pub fn resolve_bridge_friction_length_metric(
    interior: &BridgeInteriorInput,
    interval_length_m: f64,
    bridge_length_user: f64,
    raw_units: UnitSystem,
) -> f64 {
    if let Some(path) = friction_path_from_interior(interior, raw_units) {
        return path;
    }
    if interval_length_m > STRUCTURE_STATION_TOL {
        return interval_length_m;
    }
    let user_m = user_length_to_metric(bridge_length_user, raw_units);
    if user_m > STRUCTURE_STATION_TOL {
        return user_m;
    }
    0.0
}

fn segment_station_distance_m(a: &CrossSection, b: &CrossSection, raw_units: UnitSystem) -> f64 {
    let sa = xs_river_station_to_metric(a.station, a.unit_system, raw_units);
    let sb = xs_river_station_to_metric(b.station, b.unit_system, raw_units);
    (sa - sb).abs()
}

/// Resolve opening / approach / departure friction reach segments (metric, before skew).
pub fn resolve_bridge_friction_lengths_metric(
    interior: &BridgeInteriorInput,
    interval_length_m: f64,
    bridge_length_user: f64,
    approach_xs: Option<&CrossSection>,
    departure_xs: Option<&CrossSection>,
    bu_xs: Option<&CrossSection>,
    bd_xs: Option<&CrossSection>,
    weighting: BridgeFrictionWeighting,
    approach_friction_length_user: f64,
    departure_friction_length_user: f64,
    raw_units: UnitSystem,
) -> BridgeFrictionLengths {
    let opening_m = resolve_bridge_friction_length_metric(
        interior,
        interval_length_m,
        bridge_length_user,
        raw_units,
    );
    let approach_m = if approach_friction_length_user > STRUCTURE_STATION_TOL {
        user_length_to_metric(approach_friction_length_user, raw_units)
    } else if let (Some(ap), Some(bu)) = (approach_xs, bu_xs) {
        segment_station_distance_m(ap, bu, raw_units)
    } else {
        0.0
    };
    let departure_m = if departure_friction_length_user > STRUCTURE_STATION_TOL {
        user_length_to_metric(departure_friction_length_user, raw_units)
    } else if let (Some(dep), Some(bd)) = (departure_xs, bd_xs) {
        segment_station_distance_m(bd, dep, raw_units)
    } else {
        0.0
    };
    BridgeFrictionLengths {
        weighting,
        opening_m,
        approach_m,
        departure_m,
    }
}

/// Resolve approach / departure cuts and guide banks for one bridge interval.
///
/// Approach / departure `CrossSection` precedence:
/// 1. Explicit `interior.approach` / `interior.departure`
/// 2. Reach XS at `approach_reach_station` / `departure_reach_station` on the densified grid
/// 3. Nearest reach node upstream of BU (`bu_interval_idx - 1`) / downstream of BD (`bu_interval_idx + 2`)
pub fn resolve_approach_departure_sections(
    interior: &BridgeInteriorInput,
    bu_interval_idx: usize,
    densified_stations: &[f64],
    densified_xs: &[Option<CrossSection>],
    raw_units: UnitSystem,
) -> (
    Option<CrossSection>,
    Option<CrossSection>,
    Option<GuideBanks>,
    Option<GuideBanks>,
) {
    let approach_xs = interior
        .approach
        .clone()
        .or_else(|| {
            interior.approach_reach_station.and_then(|st| {
                cross_section_at_reach_station(densified_stations, densified_xs, st, raw_units)
            })
        })
        .or_else(|| {
            let idx = bu_interval_idx.checked_sub(1)?;
            densified_xs.get(idx).and_then(|opt| opt.clone())
        });
    let departure_xs = interior
        .departure
        .clone()
        .or_else(|| {
            interior.departure_reach_station.and_then(|st| {
                cross_section_at_reach_station(densified_stations, densified_xs, st, raw_units)
            })
        })
        .or_else(|| {
            let idx = bu_interval_idx + 2;
            densified_xs.get(idx).and_then(|opt| opt.clone())
        });
    let guide_banks_approach = resolve_guide_banks(
        approach_xs.as_ref(),
        interior.approach_guide_banks.as_ref(),
    );
    let guide_banks_departure = resolve_guide_banks(
        departure_xs.as_ref(),
        interior.departure_guide_banks.as_ref(),
    );
    (
        approach_xs,
        departure_xs,
        guide_banks_approach,
        guide_banks_departure,
    )
}

/// Build geometry tables and section context for a bridge interval.
pub fn resolve_bridge_face_solve_geometry(
    params: BridgeFaceSolveParams<'_>,
) -> BridgeFaceSolveGeometry {
    let BridgeFaceSolveParams {
        interior,
        anchor_reach_xs,
        reach_xs_up,
        reach_xs_down,
        reach_table_up,
        reach_table_down,
        reach_z_up_user,
        reach_z_down_user,
        raw_units,
        num_slices,
        ineffective_up,
        ineffective_down,
        skew_deg,
        pier_stations,
        interval_length_m,
        bridge_length_user,
        friction_weighting,
        approach_friction_length_user,
        departure_friction_length_user,
        approach_xs,
        departure_xs,
        guide_banks_approach,
        guide_banks_departure,
        pier_widths,
        pier_attachments,
        deck_vents,
        embankment_blocked,
    } = params;

    let mut xs_up = interior
        .bu
        .clone()
        .or_else(|| reach_xs_up.cloned());
    let mut xs_down = interior
        .bd
        .clone()
        .or_else(|| reach_xs_down.cloned());

    let opening_origin = resolve_opening_reach_station_origin(
        interior.opening_reach_station_origin,
        interior.opening_anchor_mode,
        xs_up.as_ref(),
        anchor_reach_xs,
        reach_xs_up,
    );
    if let Some(blocked) = embankment_blocked {
        if let Some(ref mut xs) = xs_up {
            crate::solvers::bridge_roadway_compose::merge_embankment_blocked_into_section(
                xs,
                blocked.left.as_ref(),
                blocked.right.as_ref(),
                opening_origin,
            );
        }
        if let Some(ref mut xs) = xs_down {
            crate::solvers::bridge_roadway_compose::merge_embankment_blocked_into_section(
                xs,
                blocked.left.as_ref(),
                blocked.right.as_ref(),
                opening_origin,
            );
        }
    }

    let table_up = xs_up
        .as_ref()
        .map(|xs| table_from_xs(xs, num_slices))
        .map(|t| t)
        .unwrap_or_else(|| reach_table_up.clone());
    let table_down = xs_down
        .as_ref()
        .map(|xs| table_from_xs(xs, num_slices))
        .unwrap_or_else(|| reach_table_down.clone());

    let z_up_user = xs_up
        .as_ref()
        .map(|xs| bed_user_from_xs(xs, raw_units))
        .unwrap_or(reach_z_up_user);
    let z_down_user = xs_down
        .as_ref()
        .map(|xs| bed_user_from_xs(xs, raw_units))
        .unwrap_or(reach_z_down_user);

    let ineffective_up = resolve_face_ineffective(
        xs_up.as_ref(),
        interior.bu.is_some(),
        reach_xs_up,
        ineffective_up,
        opening_origin,
        raw_units,
    );
    let ineffective_down = resolve_face_ineffective(
        xs_down.as_ref(),
        interior.bd.is_some(),
        reach_xs_down,
        ineffective_down,
        opening_origin,
        raw_units,
    );

    let friction_lengths = resolve_bridge_friction_lengths_metric(
        interior,
        interval_length_m,
        bridge_length_user,
        approach_xs.as_ref(),
        departure_xs.as_ref(),
        xs_up.as_ref(),
        xs_down.as_ref(),
        friction_weighting,
        approach_friction_length_user,
        departure_friction_length_user,
        raw_units,
    );
    let friction_length_m = friction_lengths.opening_m;

    let pier_stations = remap_opening_stations_option(pier_stations, opening_origin);

    let sections = BridgeSectionContext {
        ineffective_up,
        ineffective_down,
        xs_up,
        xs_down,
        internal_xs: interior.internal.clone(),
        opening_reach_station_origin: opening_origin,
        skew_deg,
        pier_stations,
        pier_widths,
        pier_attachments,
        deck_vents,
        friction_length_m,
        friction_lengths,
        xs_approach: approach_xs,
        xs_departure: departure_xs,
        guide_banks_approach,
        guide_banks_departure,
    };

    BridgeFaceSolveGeometry {
        table_up,
        table_down,
        sections,
        z_up_user,
        z_down_user,
    }
}

pub fn interior_from_steady(
    inputs: &crate::solvers::steady::SteadyInputs,
    b_idx: usize,
) -> BridgeInteriorInput {
    BridgeInteriorInput {
        bu: inputs
            .bridge_upstream_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        bd: inputs
            .bridge_downstream_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        internal: inputs
            .bridge_internal_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned()
            .unwrap_or_default(),
        opening_reach_station_origin: inputs
            .bridge_opening_reach_station_origins
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        opening_anchor_mode: inputs
            .bridge_opening_anchor_modes
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .map(BridgeOpeningAnchorMode::from_i32),
        opening_anchor_reach_station: inputs
            .bridge_opening_anchor_reach_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        approach: inputs
            .bridge_approach_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        departure: inputs
            .bridge_departure_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        approach_reach_station: inputs
            .bridge_approach_reach_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        departure_reach_station: inputs
            .bridge_departure_reach_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        approach_guide_banks: inputs
            .bridge_approach_guide_banks
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        departure_guide_banks: inputs
            .bridge_departure_guide_banks
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
    }
}

/// River stations (metric) of BU and BD faces for one bridge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BridgeFaceStations {
    pub bu_station_m: f64,
    pub bd_station_m: f64,
}

/// BU / BD / internal role for a bridge layout insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeLayoutCutKind {
    Bu,
    Bd,
    Internal,
}

/// Metadata for interpolated BU/BD faces (no explicit `CrossSection` on the cut).
#[derive(Debug, Clone)]
pub struct BridgeFaceInsertMeta {
    pub ineffective_opening: Option<IneffectiveFlowAreas>,
    pub embankment_blocked: Option<crate::solvers::bridge_roadway_compose::ComposedEmbankmentBlocked>,
    pub interior: BridgeInteriorInput,
}

/// One densification node to insert at a bridge layout river station.
#[derive(Debug, Clone)]
pub struct BridgeLayoutCut {
    pub station_m: f64,
    pub xs: Option<CrossSection>,
    pub kind: BridgeLayoutCutKind,
    /// When `xs` is omitted on BU/BD, opening-frame ineffective and anchor fields for reach shift.
    pub face_meta: Option<BridgeFaceInsertMeta>,
}

fn user_length_to_metric(value: f64, raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        value * FT_TO_M
    } else {
        value
    }
}

fn xs_river_station_to_metric(station: f64, xs_units: UnitSystem, raw_units: UnitSystem) -> f64 {
    match (raw_units, xs_units) {
        (UnitSystem::USCustomary, UnitSystem::Metric) => station * FT_TO_M,
        (UnitSystem::Metric, UnitSystem::USCustomary) => station / FT_TO_M,
        _ => station,
    }
}

/// Resolve BU/BD reach stations (metric, upstream ≥ downstream).
pub fn resolve_bridge_face_stations_metric(
    center_station_user: f64,
    raw_units: UnitSystem,
    bu: Option<&CrossSection>,
    bd: Option<&CrossSection>,
    bridge_length_user: f64,
) -> BridgeFaceStations {
    let center_m = user_length_to_metric(center_station_user, raw_units);
    let length_m = user_length_to_metric(bridge_length_user, raw_units);
    let bu_from_xs = bu.map(|xs| xs_river_station_to_metric(xs.station, xs.unit_system, raw_units));
    let bd_from_xs = bd.map(|xs| xs_river_station_to_metric(xs.station, xs.unit_system, raw_units));

    let (bu_station_m, bd_station_m) = match (bu_from_xs, bd_from_xs) {
        (Some(bu), Some(bd)) => (bu.max(bd), bu.min(bd)),
        (Some(bu), None) => {
            let bd = if length_m > 0.0 { bu - length_m } else { bu };
            (bu, bd)
        }
        (None, Some(bd)) => {
            let bu = if length_m > 0.0 { bd + length_m } else { bd };
            (bu, bd)
        }
        (None, None) => {
            if length_m > 0.0 {
                (center_m + length_m * 0.5, center_m - length_m * 0.5)
            } else {
                (center_m, center_m)
            }
        }
    };

    BridgeFaceStations {
        bu_station_m,
        bd_station_m,
    }
}

pub fn layout_cuts_for_bridge(
    interior: &BridgeInteriorInput,
    faces: BridgeFaceStations,
    raw_units: UnitSystem,
    ineffective_up: Option<IneffectiveFlowAreas>,
    ineffective_down: Option<IneffectiveFlowAreas>,
    embankment_blocked: Option<&crate::solvers::bridge_roadway_compose::ComposedEmbankmentBlocked>,
) -> Vec<BridgeLayoutCut> {
    let has_explicit = interior.bu.is_some()
        || interior.bd.is_some()
        || !interior.internal.is_empty();
    let faces_differ =
        (faces.bu_station_m - faces.bd_station_m).abs() > STRUCTURE_STATION_TOL;
    if !has_explicit && !faces_differ {
        return vec![];
    }

    let mut cuts = Vec::new();
    cuts.push(BridgeLayoutCut {
        station_m: faces.bu_station_m,
        xs: interior.bu.clone(),
        kind: BridgeLayoutCutKind::Bu,
        face_meta: if interior.bu.is_none() {
            Some(BridgeFaceInsertMeta {
                ineffective_opening: ineffective_up,
                embankment_blocked: embankment_blocked.cloned(),
                interior: interior.clone(),
            })
        } else {
            None
        },
    });
    for xs in &interior.internal {
        cuts.push(BridgeLayoutCut {
            station_m: xs_river_station_to_metric(xs.station, xs.unit_system, raw_units),
            xs: Some(xs.clone()),
            kind: BridgeLayoutCutKind::Internal,
            face_meta: None,
        });
    }
    cuts.push(BridgeLayoutCut {
        station_m: faces.bd_station_m,
        xs: interior.bd.clone(),
        kind: BridgeLayoutCutKind::Bd,
        face_meta: if interior.bd.is_none() {
            Some(BridgeFaceInsertMeta {
                ineffective_opening: ineffective_down,
                embankment_blocked: embankment_blocked.cloned(),
                interior: interior.clone(),
            })
        } else {
            None
        },
    });
    cuts
}

fn bridge_face_ineffective_on_reach(
    cut: &BridgeLayoutCut,
    reach_xs_upstream: Option<&CrossSection>,
    raw_units: UnitSystem,
) -> Option<IneffectiveFlowAreas> {
    let meta = cut.face_meta.as_ref()?;
    let origin = resolve_opening_reach_station_origin(
        meta.interior.opening_reach_station_origin,
        meta.interior.opening_anchor_mode,
        meta.interior.bu.as_ref(),
        None,
        reach_xs_upstream,
    );
    let opening = meta.ineffective_opening.clone()?;
    let user_units = if raw_units == UnitSystem::USCustomary {
        opening.to_metric(UnitSystem::USCustomary)
    } else {
        opening
    };
    opening_frame_ineffective_to_reach(Some(user_units.clone()), origin)
        .or(Some(user_units))
}

fn apply_bridge_face_ineffective_to_section(
    section: &mut CrossSection,
    cut: &BridgeLayoutCut,
    reach_xs_upstream: Option<&CrossSection>,
    raw_units: UnitSystem,
) {
    if cut.xs.is_some() {
        return;
    }
    match cut.kind {
        BridgeLayoutCutKind::Bu | BridgeLayoutCutKind::Bd => {
            section.ineffective_flow_areas =
                bridge_face_ineffective_on_reach(cut, reach_xs_upstream, raw_units);
        }
        BridgeLayoutCutKind::Internal => {}
    }
}

fn apply_bridge_face_blocked_to_section(
    section: &mut CrossSection,
    cut: &BridgeLayoutCut,
    reach_xs_upstream: Option<&CrossSection>,
) {
    if cut.xs.is_some() {
        return;
    }
    let meta = match cut.face_meta.as_ref() {
        Some(m) => m,
        None => return,
    };
    let blocked = match meta.embankment_blocked.as_ref() {
        Some(b) => b,
        None => return,
    };
    let origin = resolve_opening_reach_station_origin(
        meta.interior.opening_reach_station_origin,
        meta.interior.opening_anchor_mode,
        meta.interior.bu.as_ref(),
        None,
        reach_xs_upstream,
    );
    crate::solvers::bridge_roadway_compose::merge_embankment_blocked_into_section(
        section,
        blocked.left.as_ref(),
        blocked.right.as_ref(),
        origin,
    );
}

fn build_interpolated_layout_node(
    cut: &BridgeLayoutCut,
    i: usize,
    t: f64,
    station_m: f64,
    tables: &[GeometryTable],
    z_mins: &[f64],
    xs: &[Option<CrossSection>],
    num_slices: usize,
    densify_policy: DensifyReachModifierPolicy,
    raw_units: UnitSystem,
) -> (GeometryTable, f64, Option<CrossSection>) {
    let up_ref = xs.get(i).and_then(|o| o.as_ref());
    let down_ref = xs.get(i + 1).and_then(|o| o.as_ref());

    if let (Some(up), Some(down)) = (up_ref, down_ref) {
        let up_m = up.to_metric();
        let down_m = down.to_metric();
        let mut synthetic = interpolate_cross_section(&up_m, &down_m, t, station_m);
        match cut.kind {
            BridgeLayoutCutKind::Bu | BridgeLayoutCutKind::Bd => {
                apply_bridge_face_ineffective_to_section(
                    &mut synthetic,
                    cut,
                    up_ref,
                    raw_units,
                );
                apply_bridge_face_blocked_to_section(&mut synthetic, cut, up_ref);
            }
            BridgeLayoutCutKind::Internal => {
                apply_reach_modifier_policy(&mut synthetic, &up_m, &down_m, t, densify_policy);
            }
        }
        let z = cross_section_min_bed(&synthetic);
        let table = synthetic.generate_lookup_table(num_slices);
        return (table, z, Some(synthetic));
    }

    let (table_interp, z_interp) = crate::geometry::processor::interpolate_geometry_table(
        &tables[i],
        z_mins[i],
        &tables[i + 1],
        z_mins[i + 1],
        t,
        num_slices,
    );
    let mut xs_new = xs[i]
        .clone()
        .or_else(|| xs.get(i + 1).and_then(|x| x.clone()))
        .map(|mut section| {
            section.station = station_m;
            section
        });
    if let Some(ref mut section) = xs_new {
        if matches!(
            cut.kind,
            BridgeLayoutCutKind::Bu | BridgeLayoutCutKind::Bd
        ) {
            section.ineffective_flow_areas = None;
            apply_bridge_face_ineffective_to_section(section, cut, up_ref, raw_units);
            apply_bridge_face_blocked_to_section(section, cut, up_ref);
            if section
                .ineffective_flow_areas
                .as_ref()
                .is_some_and(|i| i.is_configured())
                || section
                    .blocked_obstructions
                    .as_ref()
                    .is_some_and(|b| !b.is_empty())
            {
                let metric = section.to_metric();
                let z = cross_section_min_bed(&metric);
                let table = metric.generate_lookup_table(num_slices);
                return (table, z, Some(metric));
            }
        }
    }
    (table_interp, z_interp, xs_new)
}

fn station_exists(stations: &[f64], station_m: f64) -> bool {
    stations
        .iter()
        .any(|&s| (s - station_m).abs() <= STRUCTURE_STATION_TOL)
}

fn find_upstream_segment(stations: &[f64], station_m: f64) -> Option<usize> {
    for i in 0..stations.len().saturating_sub(1) {
        let us = stations[i];
        let ds = stations[i + 1];
        if station_m <= us + STRUCTURE_STATION_TOL && station_m >= ds - STRUCTURE_STATION_TOL {
            if (station_m - us).abs() > STRUCTURE_STATION_TOL
                || (station_m - ds).abs() > STRUCTURE_STATION_TOL
            {
                return Some(i);
            }
        }
    }
    None
}

/// Insert bridge layout cuts into a descending reach grid (stations in metric).
pub fn insert_reach_layout_cuts(
    stations: &mut Vec<f64>,
    tables: &mut Vec<GeometryTable>,
    z_mins: &mut Vec<f64>,
    xs: &mut Vec<Option<CrossSection>>,
    cuts: &[BridgeLayoutCut],
    num_slices: usize,
    densify_policy: DensifyReachModifierPolicy,
    raw_units: UnitSystem,
    interpolated_fields: &mut [&mut Vec<f64>],
) {
    let mut ordered: Vec<&BridgeLayoutCut> = cuts.iter().collect();
    ordered.sort_by(|a, b| b.station_m.partial_cmp(&a.station_m).unwrap());

    for cut in ordered {
        if station_exists(stations, cut.station_m) {
            if let Some(idx) = stations
                .iter()
                .position(|&s| (s - cut.station_m).abs() <= STRUCTURE_STATION_TOL)
            {
                if let Some(ref explicit) = cut.xs {
                    let metric = explicit.to_metric();
                    tables[idx] = metric.generate_lookup_table(num_slices);
                    z_mins[idx] = cross_section_min_bed(&metric);
                    xs[idx] = Some(metric);
                }
            }
            continue;
        }

        let Some(i) = find_upstream_segment(stations, cut.station_m) else {
            continue;
        };
        let us = stations[i];
        let ds = stations[i + 1];
        let span = us - ds;
        if span <= STRUCTURE_STATION_TOL {
            continue;
        }
        let t = (us - cut.station_m) / span;

        let (table_new, z_new, xs_new) = if let Some(ref explicit) = cut.xs {
            let metric = explicit.to_metric();
            let mut placed = metric.clone();
            placed.station = cut.station_m;
            (
                metric.generate_lookup_table(num_slices),
                cross_section_min_bed(&metric),
                Some(placed),
            )
        } else {
            build_interpolated_layout_node(
                cut,
                i,
                t,
                cut.station_m,
                tables,
                z_mins,
                xs,
                num_slices,
                densify_policy,
                raw_units,
            )
        };

        let insert_at = i + 1;
        stations.insert(insert_at, cut.station_m);
        tables.insert(insert_at, table_new);
        z_mins.insert(insert_at, z_new);
        xs.insert(insert_at, xs_new);
        for field in interpolated_fields.iter_mut() {
            let up = field[i];
            let down = field[i + 1];
            field.insert(insert_at, up + t * (down - up));
        }
    }
}

/// Interval `i` spans BU (`stations[i]`) → BD (`stations[i+1]`).
pub fn find_bridge_face_interval(
    faces: BridgeFaceStations,
    stations: &[f64],
) -> Option<usize> {
    let tol = STRUCTURE_STATION_TOL;
    for i in 0..stations.len().saturating_sub(1) {
        if (stations[i] - faces.bu_station_m).abs() <= tol
            && (stations[i + 1] - faces.bd_station_m).abs() <= tol
        {
            return Some(i);
        }
    }
    if (faces.bu_station_m - faces.bd_station_m).abs() <= tol {
        for i in 0..stations.len().saturating_sub(1) {
            if structure_in_reach_interval(faces.bu_station_m, stations, i) {
                return Some(i);
            }
        }
    }
    None
}

fn bridge_length_user_steady(inputs: &crate::solvers::steady::SteadyInputs, b_idx: usize) -> f64 {
    inputs
        .bridge_lengths
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.0)
}

fn bridge_length_user_unsteady(
    b: &crate::solvers::unsteady::UnsteadyBridgeInputs,
    b_idx: usize,
) -> f64 {
    b.bridge_lengths
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.0)
}

/// Insert BU/BD/internal nodes and return bridge interval index per bridge (`None` if unmatched).
pub fn apply_bridge_reach_layout_steady(
    inputs: &crate::solvers::steady::SteadyInputs,
    raw_units: UnitSystem,
    num_slices: usize,
    stations: &mut Vec<f64>,
    tables: &mut Vec<GeometryTable>,
    z_mins: &mut Vec<f64>,
    xs: &mut Vec<Option<CrossSection>>,
) -> Vec<Option<usize>> {
    let Some(ref centers) = inputs.bridge_stations else {
        return vec![];
    };
    let mut all_cuts = Vec::new();
    let mut face_list = Vec::with_capacity(centers.len());

    for (b_idx, &center) in centers.iter().enumerate() {
        let interior = interior_from_steady(inputs, b_idx);
        let faces = resolve_bridge_face_stations_metric(
            center,
            raw_units,
            interior.bu.as_ref(),
            interior.bd.as_ref(),
            bridge_length_user_steady(inputs, b_idx),
        );
        face_list.push(faces);
        all_cuts.extend(layout_cuts_for_bridge(
            &interior,
            faces,
            raw_units,
            crate::solvers::steady::bridge_ineffective_upstream_for(inputs, b_idx),
            crate::solvers::steady::bridge_ineffective_downstream_for(inputs, b_idx),
            crate::solvers::bridge_roadway_compose::composed_embankment_blocked_for(
                &inputs.bridge_composed_embankment_blocked,
                b_idx,
            ),
        ));
    }

    let densify_policy =
        DensifyReachModifierPolicy::from_option(inputs.densify_reach_modifier_policy);
    insert_reach_layout_cuts(
        stations,
        tables,
        z_mins,
        xs,
        &all_cuts,
        num_slices,
        densify_policy,
        raw_units,
        &mut [],
    );

    face_list
        .iter()
        .map(|faces| find_bridge_face_interval(*faces, stations))
        .collect()
}

pub fn apply_bridge_reach_layout_unsteady(
    inputs: &crate::solvers::unsteady::UnsteadyInputs,
    raw_units: UnitSystem,
    num_slices: usize,
    stations: &mut Vec<f64>,
    tables: &mut Vec<GeometryTable>,
    z_mins: &mut Vec<f64>,
    xs: &mut Vec<CrossSection>,
    y_current: &mut Vec<f64>,
    q_current: &mut Vec<f64>,
) -> Vec<Option<usize>> {
    let b = &inputs.bridge;
    let Some(ref centers) = b.bridge_stations else {
        return vec![];
    };
    let mut all_cuts = Vec::new();
    let mut face_list = Vec::with_capacity(centers.len());

    for (b_idx, &center) in centers.iter().enumerate() {
        let interior = interior_from_unsteady(b, b_idx);
        let faces = resolve_bridge_face_stations_metric(
            center,
            raw_units,
            interior.bu.as_ref(),
            interior.bd.as_ref(),
            bridge_length_user_unsteady(b, b_idx),
        );
        face_list.push(faces);
        all_cuts.extend(layout_cuts_for_bridge(
            &interior,
            faces,
            raw_units,
            crate::solvers::unsteady::bridge_ineffective_upstream_for(inputs, b_idx),
            crate::solvers::unsteady::bridge_ineffective_downstream_for(inputs, b_idx),
            crate::solvers::bridge_roadway_compose::composed_embankment_blocked_for(
                &b.bridge_composed_embankment_blocked,
                b_idx,
            ),
        ));
    }

    let densify_policy =
        DensifyReachModifierPolicy::from_option(inputs.densify_reach_modifier_policy);
    let mut xs_opt: Vec<Option<CrossSection>> = xs.iter().cloned().map(Some).collect();
    insert_reach_layout_cuts(
        stations,
        tables,
        z_mins,
        &mut xs_opt,
        &all_cuts,
        num_slices,
        densify_policy,
        raw_units,
        &mut [y_current, q_current],
    );
    xs.clear();
    xs.extend(xs_opt.into_iter().enumerate().map(|(idx, opt)| {
        let mut section = opt.expect("unsteady reach grid requires a cross-section at every node");
        section.station = stations[idx];
        section
    }));

    face_list
        .iter()
        .map(|faces| find_bridge_face_interval(*faces, stations))
        .collect()
}

/// Re-map original cross-section indices after layout nodes are inserted.
pub fn refresh_original_to_densified(
    original_stations: &[f64],
    densified_stations: &[f64],
    original_to_densified: &mut [usize],
) {
    for (orig_idx, slot) in original_to_densified.iter_mut().enumerate() {
        let target = original_stations[orig_idx];
        if let Some(i) = densified_stations
            .iter()
            .position(|&s| (s - target).abs() <= STRUCTURE_STATION_TOL)
        {
            *slot = i;
        }
    }
}

pub fn bridge_intervals_from_faces(
    face_intervals: &[Option<usize>],
) -> Vec<(usize, usize)> {
    face_intervals
        .iter()
        .enumerate()
        .filter_map(|(b_idx, interval)| interval.map(|i| (i, b_idx)))
        .collect()
}

pub fn interior_from_unsteady(
    b: &crate::solvers::unsteady::UnsteadyBridgeInputs,
    b_idx: usize,
) -> BridgeInteriorInput {
    BridgeInteriorInput {
        bu: b
            .bridge_upstream_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        bd: b
            .bridge_downstream_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        internal: b
            .bridge_internal_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned()
            .unwrap_or_default(),
        opening_reach_station_origin: b
            .bridge_opening_reach_station_origins
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        opening_anchor_mode: b
            .bridge_opening_anchor_modes
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .map(BridgeOpeningAnchorMode::from_i32),
        opening_anchor_reach_station: b
            .bridge_opening_anchor_reach_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        approach: b
            .bridge_approach_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        departure: b
            .bridge_departure_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        approach_reach_station: b
            .bridge_approach_reach_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        departure_reach_station: b
            .bridge_departure_reach_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        approach_guide_banks: b
            .bridge_approach_guide_banks
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        departure_guide_banks: b
            .bridge_departure_guide_banks
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::GeometryTable;

    fn box_xs(x0: f64, width: f64, bed: f64, crest: f64) -> CrossSection {
        CrossSection {
            station: 0.0,
            x: vec![x0, x0, x0 + width, x0 + width],
            y: vec![crest, bed, bed, crest],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        }
    }

    fn flat_table() -> GeometryTable {
        box_xs(0.0, 20.0, 0.0, 5.0).generate_lookup_table(20)
    }

    fn resolve_face<'a>(
        interior: &'a BridgeInteriorInput,
        reach_table_up: &'a GeometryTable,
        reach_table_down: &'a GeometryTable,
        configure: impl FnOnce(&mut BridgeFaceSolveParams<'a>),
    ) -> BridgeFaceSolveGeometry {
        let mut params = BridgeFaceSolveParams::new(interior, reach_table_up, reach_table_down);
        configure(&mut params);
        resolve_bridge_face_solve_geometry(params)
    }

    #[test]
    fn explicit_bu_overrides_reach_table() {
        let bu = box_xs(50.0, 10.0, 2.0, 8.0);
        let interior = BridgeInteriorInput {
            bu: Some(bu.clone()),
            bd: None,
            internal: vec![],
            opening_reach_station_origin: Some(50.0),
            ..Default::default()
        };
        let reach = box_xs(0.0, 30.0, 0.0, 6.0);
        let table = flat_table();
        let geo = resolve_face(&interior, &table, &table, |p| {
            p.reach_xs_up = Some(&reach);
            p.reach_xs_down = Some(&reach);
            p.num_slices = 30;
        });
        assert!((geo.z_up_user - 2.0).abs() < 1e-9);
        assert_eq!(geo.sections.opening_reach_station_origin, Some(50.0));
        let a_bu = geo
            .table_up
            .interpolate(5.0)
            .area;
        let a_reach = flat_table().interpolate(5.0).area;
        assert!(a_bu < a_reach);
    }

    #[test]
    fn opening_station_mapping() {
        assert!((opening_station_to_reach_x(15.0, 100.0) - 115.0).abs() < 1e-9);
        assert!((reach_x_to_opening_station(115.0, 100.0) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn face_stations_from_explicit_bu_bd() {
        let bu = CrossSection {
            station: 505.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
        guide_banks: None,
        };
        let bd = CrossSection {
            station: 495.0,
            ..bu.clone()
        };
        let faces = resolve_bridge_face_stations_metric(
            500.0,
            UnitSystem::Metric,
            Some(&bu),
            Some(&bd),
            0.0,
        );
        assert_eq!(faces.bu_station_m, 505.0);
        assert_eq!(faces.bd_station_m, 495.0);
    }

    #[test]
    fn face_stations_from_bridge_length_when_no_explicit_faces() {
        let faces = resolve_bridge_face_stations_metric(
            500.0,
            UnitSystem::Metric,
            None,
            None,
            20.0,
        );
        assert!((faces.bu_station_m - 510.0).abs() < 1e-9);
        assert!((faces.bd_station_m - 490.0).abs() < 1e-9);
    }

    #[test]
    fn insert_bu_bd_nodes_and_match_interval() {
        let mut stations = vec![600.0, 400.0, 200.0, 0.0];
        let table = flat_table();
        let mut tables = vec![table.clone(), table.clone(), table.clone(), table.clone()];
        let mut z_mins = vec![0.0; 4];
        let mut xs: Vec<Option<CrossSection>> = stations
            .iter()
            .map(|&st| {
                let mut section = box_xs(0.0, 20.0, 0.0, 5.0);
                section.station = st;
                Some(section)
            })
            .collect();
        let bu = {
            let mut s = box_xs(0.0, 8.0, 0.0, 5.0);
            s.station = 505.0;
            s
        };
        let bd = {
            let mut s = box_xs(0.0, 8.0, 0.0, 5.0);
            s.station = 495.0;
            s
        };
        let faces = BridgeFaceStations {
            bu_station_m: 505.0,
            bd_station_m: 495.0,
        };
        let cuts = layout_cuts_for_bridge(
            &BridgeInteriorInput {
                bu: Some(bu),
                bd: Some(bd),
                internal: vec![],
                opening_reach_station_origin: None,
                ..Default::default()
            },
            faces,
            UnitSystem::Metric,
            None,
            None,
            None,
        );
        insert_reach_layout_cuts(
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
            &cuts,
            20,
            DensifyReachModifierPolicy::None,
            UnitSystem::Metric,
            &mut [],
        );
        let interval = find_bridge_face_interval(faces, &stations).expect("BU/BD interval");
        assert_eq!(stations[interval], 505.0);
        assert_eq!(stations[interval + 1], 495.0);
        assert!(tables[interval].interpolate(3.0).area < tables[interval + 2].interpolate(3.0).area);
    }

    #[test]
    fn infer_origin_from_bu_min_x() {
        let bu = box_xs(42.5, 12.0, 1.0, 6.0);
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: None,
            internal: vec![],
            ..Default::default()
        };
        let table = flat_table();
        let geo = resolve_face(&interior, &table, &table, |p| {
            p.num_slices = 20;
        });
        assert_eq!(geo.sections.opening_reach_station_origin, Some(42.5));
    }

    #[test]
    fn explicit_bu_ineffective_independent_of_reach_face() {
        let mut reach = box_xs(0.0, 40.0, 0.0, 5.0);
        reach.is_overbank = Some(vec![false, false, false, false, true, true, true, true]);
        reach.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[35.0], &[3.0], &[], &[]).unwrap(),
        );

        let mut bu = box_xs(0.0, 40.0, 0.0, 5.0);
        bu.is_overbank = reach.is_overbank.clone();
        bu.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[12.0], &[3.0], &[], &[]).unwrap(),
        );

        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: None,
            internal: vec![],
            opening_reach_station_origin: Some(0.0),
            ..Default::default()
        };
        let bridge_opening = IneffectiveFlowAreas::from_block_pairs(&[20.0], &[3.0], &[], &[]).unwrap();

        let table = flat_table();
        let geo = resolve_face(&interior, &table, &table, |p| {
            p.reach_xs_up = Some(&reach);
            p.reach_xs_down = Some(&reach);
            p.num_slices = 30;
            p.ineffective_up = Some(bridge_opening);
        });

        let up = geo
            .sections
            .ineffective_up
            .expect("BU section ineffective should apply");
        assert_eq!(up.left_blocks.len(), 1);
        assert!((up.left_blocks[0].station - 12.0).abs() < 1e-9);
    }

    #[test]
    fn interpolated_bu_inherits_bridge_not_reach_ineffective() {
        let mut reach_us = box_xs(0.0, 40.0, 0.0, 5.0);
        reach_us.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
        );
        reach_us.station = 520.0;
        let mut reach_ds = box_xs(0.0, 40.0, 0.0, 5.0);
        reach_ds.station = 480.0;

        let mut stations = vec![520.0, 480.0];
        let table_us = reach_us.generate_lookup_table(20);
        let table_ds = reach_ds.generate_lookup_table(20);
        let mut tables = vec![table_us, table_ds];
        let mut z_mins = vec![
            cross_section_min_bed(&reach_us),
            cross_section_min_bed(&reach_ds),
        ];
        let mut xs: Vec<Option<CrossSection>> = vec![Some(reach_us), Some(reach_ds)];

        let faces = resolve_bridge_face_stations_metric(
            500.0,
            UnitSystem::Metric,
            None,
            None,
            20.0,
        );
        let bridge_opening =
            IneffectiveFlowAreas::from_block_pairs(&[5.0], &[3.0], &[], &[]).unwrap();
        let interior = BridgeInteriorInput {
            opening_reach_station_origin: Some(0.0),
            ..Default::default()
        };
        let cuts = layout_cuts_for_bridge(
            &interior,
            faces,
            UnitSystem::Metric,
            Some(bridge_opening),
            None,
            None,
        );
        insert_reach_layout_cuts(
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
            &cuts,
            20,
            DensifyReachModifierPolicy::Upstream,
            UnitSystem::Metric,
            &mut [],
        );

        let bu_idx = stations
            .iter()
            .position(|&s| (s - faces.bu_station_m).abs() < 1e-6)
            .expect("BU node");
        let bu_xs = xs[bu_idx].as_ref().expect("BU synthetic xs");
        let ineff = bu_xs
            .ineffective_flow_areas
            .as_ref()
            .expect("bridge ineffective on BU");
        assert_eq!(ineff.left_blocks.len(), 1);
        assert!(
            (ineff.left_blocks[0].station - 5.0).abs() < 1e-9,
            "BU should use bridge ineffective (5), not reach (30)"
        );

        let geo = resolve_face(
            &interior,
            &tables[bu_idx],
            &tables[bu_idx + 1],
            |p| {
                p.reach_xs_up = Some(bu_xs);
                p.reach_xs_down = xs[bu_idx + 1].as_ref();
                p.num_slices = 20;
                p.ineffective_up = Some(
                    IneffectiveFlowAreas::from_block_pairs(&[5.0], &[3.0], &[], &[]).unwrap(),
                );
            },
        );
        let up = geo.sections.ineffective_up.expect("BU bridge ineffective");
        assert!((up.left_blocks[0].station - 5.0).abs() < 1e-9);
    }

    #[test]
    fn reach_fallback_uses_reach_xs_ineffective() {
        let mut reach = box_xs(0.0, 40.0, 0.0, 5.0);
        reach.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[25.0], &[3.0], &[], &[]).unwrap(),
        );

        let interior = BridgeInteriorInput::default();
        let table = flat_table();
        let geo = resolve_face(&interior, &table, &table, |p| {
            p.reach_xs_up = Some(&reach);
            p.reach_xs_down = Some(&reach);
            p.num_slices = 30;
        });

        let up = geo.sections.ineffective_up.expect("reach ineffective");
        assert!((up.left_blocks[0].station - 25.0).abs() < 1e-9);
    }

    #[test]
    fn bridge_opening_ineffective_shifted_to_reach_x() {
        let reach = box_xs(100.0, 40.0, 0.0, 5.0);
        let bridge_opening = IneffectiveFlowAreas::from_block_pairs(&[5.0], &[3.0], &[], &[]).unwrap();

        let interior = BridgeInteriorInput {
            opening_reach_station_origin: Some(100.0),
            ..Default::default()
        };
        let table = flat_table();
        let geo = resolve_face(&interior, &table, &table, |p| {
            p.reach_xs_up = Some(&reach);
            p.num_slices = 30;
            p.ineffective_up = Some(bridge_opening);
        });

        let up = geo.sections.ineffective_up.expect("shifted ineffective");
        assert!((up.left_blocks[0].station - 105.0).abs() < 1e-9);
    }

    #[test]
    fn friction_length_from_bu_bd_stations_not_bridge_lengths() {
        let mut bu = box_xs(0.0, 10.0, 0.0, 5.0);
        bu.station = 52.0;
        let mut bd = box_xs(0.0, 10.0, 0.0, 5.0);
        bd.station = 48.0;
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: Some(bd),
            internal: vec![],
            ..Default::default()
        };
        let len = resolve_bridge_friction_length_metric(&interior, 4.0, 100.0, UnitSystem::Metric);
        assert!((len - 4.0).abs() < 1e-9);
    }

    #[test]
    fn friction_path_sums_internal_cuts() {
        let mut bu = box_xs(0.0, 10.0, 0.0, 5.0);
        bu.station = 55.0;
        let mut internal = box_xs(0.0, 10.0, 0.0, 5.0);
        internal.station = 51.0;
        let mut bd = box_xs(0.0, 10.0, 0.0, 5.0);
        bd.station = 48.0;
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: Some(bd),
            internal: vec![internal],
            ..Default::default()
        };
        let len = resolve_bridge_friction_length_metric(&interior, 0.0, 0.0, UnitSystem::Metric);
        assert!((len - 7.0).abs() < 1e-9);
    }

    #[test]
    fn friction_lengths_auto_approach_departure_from_stations() {
        let mut bu = box_xs(0.0, 10.0, 0.0, 5.0);
        bu.station = 50.0;
        let mut bd = box_xs(0.0, 10.0, 0.0, 5.0);
        bd.station = 46.0;
        let mut approach = box_xs(0.0, 10.0, 0.0, 5.0);
        approach.station = 54.0;
        let mut departure = box_xs(0.0, 10.0, 0.0, 5.0);
        departure.station = 42.0;
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: Some(bd),
            internal: vec![],
            ..Default::default()
        };
        let lens = resolve_bridge_friction_lengths_metric(
            &interior,
            0.0,
            0.0,
            Some(&approach),
            Some(&departure),
            interior.bu.as_ref(),
            interior.bd.as_ref(),
            BridgeFrictionWeighting::HecRasSegments,
            0.0,
            0.0,
            UnitSystem::Metric,
        );
        assert!((lens.opening_m - 4.0).abs() < 1e-9);
        assert!((lens.approach_m - 4.0).abs() < 1e-9);
        assert!((lens.departure_m - 4.0).abs() < 1e-9);
        assert_eq!(lens.weighting, BridgeFrictionWeighting::HecRasSegments);
    }

    #[test]
    fn friction_lengths_user_overrides_approach_departure() {
        let mut bu = box_xs(0.0, 10.0, 0.0, 5.0);
        bu.station = 50.0;
        let mut bd = box_xs(0.0, 10.0, 0.0, 5.0);
        bd.station = 46.0;
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: Some(bd),
            internal: vec![],
            ..Default::default()
        };
        let lens = resolve_bridge_friction_lengths_metric(
            &interior,
            0.0,
            0.0,
            None,
            None,
            interior.bu.as_ref(),
            interior.bd.as_ref(),
            BridgeFrictionWeighting::HecRasSegments,
            12.0,
            8.0,
            UnitSystem::Metric,
        );
        assert!((lens.approach_m - 12.0).abs() < 1e-9);
        assert!((lens.departure_m - 8.0).abs() < 1e-9);
    }

    #[test]
    fn friction_length_stored_on_section_context() {
        let mut bu = box_xs(0.0, 10.0, 0.0, 5.0);
        bu.station = 60.0;
        let mut bd = box_xs(0.0, 10.0, 0.0, 5.0);
        bd.station = 54.0;
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: Some(bd),
            internal: vec![],
            ..Default::default()
        };
        let table = flat_table();
        let geo = resolve_face(&interior, &table, &table, |p| {
            p.num_slices = 20;
            p.interval_length_m = 6.0;
            p.bridge_length_user = 50.0;
        });
        assert!((geo.sections.friction_length_m - 6.0).abs() < 1e-9);
    }

    #[test]
    fn anchor_reach_river_station_uses_approach_min_x() {
        let approach = box_xs(80.0, 50.0, 0.0, 6.0);
        let bu = box_xs(100.0, 20.0, 1.0, 6.0);
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: None,
            internal: vec![],
            opening_anchor_mode: Some(BridgeOpeningAnchorMode::ReachRiverStation),
            opening_anchor_reach_station: Some(600.0),
            ..Default::default()
        };
        let table = flat_table();
        let geo = resolve_face(&interior, &table, &table, |p| {
            p.anchor_reach_xs = Some(&approach);
            p.num_slices = 20;
        });
        assert_eq!(geo.sections.opening_reach_station_origin, Some(80.0));
    }

    #[test]
    fn explicit_lateral_x_overrides_anchor_mode() {
        let origin = resolve_opening_reach_station_origin(
            Some(99.0),
            Some(BridgeOpeningAnchorMode::BuLeft),
            None,
            None,
            None,
        );
        assert_eq!(origin, Some(99.0));
    }

    #[test]
    fn approach_departure_guide_banks_resolve_from_explicit_cuts() {
        use crate::geometry::{GuideBankToe, GuideBanks};
        let approach = CrossSection {
            station: 600.0,
            x: vec![0.0, 100.0],
            y: vec![5.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: Some(GuideBanks {
                left_toe: Some(GuideBankToe {
                    station: 20.0,
                    elevation: 4.0,
                }),
                right_toe: Some(GuideBankToe {
                    station: 80.0,
                    elevation: 4.0,
                }),
                ..Default::default()
            }),
        };
        let interior = BridgeInteriorInput {
            approach: Some(approach),
            departure_guide_banks: Some(GuideBanks {
                left_toe: Some(GuideBankToe {
                    station: 99.0,
                    elevation: 4.0,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let stations = vec![600.0, 550.0, 500.0, 450.0];
        let xs: Vec<Option<CrossSection>> = stations
            .iter()
            .map(|&st| {
                Some(CrossSection {
                    station: st,
                    x: vec![0.0, 100.0],
                    y: vec![5.0, 5.0],
                    n_stations: vec![0.0],
                    n_values: vec![0.03],
                    unit_system: UnitSystem::Metric,
                    is_overbank: None,
                    blocked_obstructions: None,
                    ineffective_flow_areas: None,
                    guide_banks: None,
                })
            })
            .collect();
        let (app, dep, gb_app, gb_dep) =
            resolve_approach_departure_sections(&interior, 1, &stations, &xs, UnitSystem::Metric);
        assert_eq!(app.as_ref().map(|x| x.station), Some(600.0));
        assert_eq!(dep.as_ref().map(|x| x.station), Some(450.0));
        assert_eq!(gb_app.unwrap().left_toe.unwrap().station, 20.0);
        assert_eq!(gb_dep.unwrap().left_toe.unwrap().station, 99.0);
    }

    #[test]
    fn pier_stations_remapped_to_reach_frame() {
        let bu = box_xs(100.0, 30.0, 1.0, 6.0);
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: None,
            internal: vec![],
            opening_reach_station_origin: Some(100.0),
            ..Default::default()
        };
        let table = flat_table();
        let geo = resolve_face(&interior, &table, &table, |p| {
            p.num_slices = 20;
            p.pier_stations = Some(vec![5.0, 20.0]);
        });
        let piers = geo.sections.pier_stations.expect("piers");
        assert!((piers[0] - 105.0).abs() < 1e-9);
        assert!((piers[1] - 120.0).abs() < 1e-9);
    }

    #[test]
    fn ineffective_preprocessor_shifts_opening_blocks() {
        let blocks = IneffectiveFlowAreas::from_block_pairs(&[8.0], &[3.0], &[], &[]).unwrap();
        let shifted = remap_ineffective_opening_to_reach(Some(blocks), 100.0).unwrap();
        assert!((shifted.left_blocks[0].station - 108.0).abs() < 1e-9);
    }

    #[test]
    fn approach_resolves_from_reach_station_pointer() {
        use crate::geometry::{GuideBankToe, GuideBanks};
        let stations = vec![600.0, 550.0, 500.0, 450.0];
        let xs: Vec<Option<CrossSection>> = stations
            .iter()
            .map(|&st| {
                let width = if (st - 550.0_f64).abs() < 1e-9 { 30.0 } else { 10.0 };
                let mut section = box_xs(st, width, 0.0, 5.0);
                section.station = st;
                Some(section)
            })
            .collect();
        let interior = BridgeInteriorInput {
            approach_reach_station: Some(550.0),
            approach_guide_banks: Some(GuideBanks {
                left_toe: Some(GuideBankToe {
                    station: 60.0,
                    elevation: 0.0,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let (app, _, gb_app, _) =
            resolve_approach_departure_sections(&interior, 1, &stations, &xs, UnitSystem::Metric);
        let app = app.expect("approach from reach station");
        assert!((app.station - 550.0).abs() < 1e-9);
        let width: f64 = app.x[2] - app.x[0];
        assert!((width - 30.0).abs() < 1e-9);
        assert_eq!(gb_app.unwrap().left_toe.unwrap().station, 60.0);
    }

    #[test]
    fn interior_from_unsteady_carries_guide_bank_fields() {
        use crate::geometry::{GuideBankToe, GuideBanks};
        use crate::solvers::unsteady::UnsteadyBridgeInputs;
        let bridge = UnsteadyBridgeInputs {
            bridge_departure_guide_banks: Some(vec![GuideBanks {
                right_toe: Some(GuideBankToe {
                    station: 90.0,
                    elevation: 0.0,
                }),
                ..Default::default()
            }]),
            bridge_departure_reach_stations: Some(vec![45.0]),
            ..UnsteadyBridgeInputs::default()
        };
        let interior = interior_from_unsteady(&bridge, 0);
        assert_eq!(
            interior.departure_guide_banks.unwrap().right_toe.unwrap().station,
            90.0
        );
        assert_eq!(interior.departure_reach_station, Some(45.0));
    }

    #[test]
    fn interior_from_steady_carries_guide_bank_fields() {
        use crate::geometry::{GuideBankToe, GuideBanks};
        use crate::solvers::steady::SteadyInputs;
        let inputs = SteadyInputs {
            bridge_stations: Some(vec![50.0]),
            bridge_approach_guide_banks: Some(vec![GuideBanks {
                left_toe: Some(GuideBankToe {
                    station: 12.0,
                    elevation: 0.0,
                }),
                ..Default::default()
            }]),
            bridge_departure_reach_stations: Some(vec![40.0]),
            ..Default::default()
        };
        let interior = interior_from_steady(&inputs, 0);
        assert_eq!(
            interior.approach_guide_banks.unwrap().left_toe.unwrap().station,
            12.0
        );
        assert_eq!(interior.departure_reach_station, Some(40.0));
    }

    #[test]
    fn resolve_approach_preserves_ineffective_overbank() {
        let approach = CrossSection {
            station: 60.0,
            x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
            y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: Some(vec![
                false, false, false, false, true, true, true, true,
            ]),
            blocked_obstructions: None,
            ineffective_flow_areas: Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
            ),
            guide_banks: None,
        };
        let interior = BridgeInteriorInput {
            approach: Some(approach),
            ..Default::default()
        };
        let stations = vec![100.0, 50.0, 0.0];
        let xs: Vec<Option<CrossSection>> = stations
            .iter()
            .map(|&st| {
                let mut s = box_xs(0.0, 40.0, 0.0, 5.0);
                s.station = st;
                Some(s)
            })
            .collect();
        let (resolved, _, _, _) =
            resolve_approach_departure_sections(&interior, 1, &stations, &xs, UnitSystem::Metric);
        let resolved = resolved.expect("explicit approach");
        let areas = resolved
            .ineffective_flow_areas
            .as_ref()
            .expect("ineffective on approach");
        assert_eq!(areas.left_blocks.len(), 1);
        assert!((areas.left_blocks[0].station - 30.0).abs() < 1e-9);
    }

    #[test]
    fn cross_section_lookup_at_reach_station() {
        let stations = vec![600.0, 400.0, 200.0];
        let xs: Vec<Option<CrossSection>> = stations
            .iter()
            .map(|&st| {
                let mut section = box_xs(st, 10.0, 0.0, 5.0);
                section.station = st;
                Some(section)
            })
            .collect();
        let found = cross_section_at_reach_station(&stations, &xs, 400.0, UnitSystem::Metric)
            .expect("station 400");
        assert!((found.x[0] - 400.0).abs() < 1e-9);
    }

    #[test]
    fn layout_cuts_attach_face_meta_when_bu_bd_omitted() {
        let ineffective =
            IneffectiveFlowAreas::from_block_pairs(&[5.0], &[3.0], &[], &[]).unwrap();
        let faces = resolve_bridge_face_stations_metric(
            500.0,
            UnitSystem::Metric,
            None,
            None,
            20.0,
        );
        let cuts = layout_cuts_for_bridge(
            &BridgeInteriorInput {
                opening_reach_station_origin: Some(0.0),
                ..Default::default()
            },
            faces,
            UnitSystem::Metric,
            Some(ineffective.clone()),
            Some(ineffective),
            None,
        );
        assert_eq!(cuts.len(), 2);
        assert_eq!(cuts[0].kind, BridgeLayoutCutKind::Bu);
        assert!(cuts[0].xs.is_none());
        assert!(cuts[0].face_meta.is_some());
        assert_eq!(cuts[1].kind, BridgeLayoutCutKind::Bd);
    }

    #[test]
    fn layout_cuts_empty_when_no_explicit_and_faces_coincide() {
        let faces = BridgeFaceStations {
            bu_station_m: 500.0,
            bd_station_m: 500.0,
        };
        let cuts = layout_cuts_for_bridge(
            &BridgeInteriorInput::default(),
            faces,
            UnitSystem::Metric,
            None,
            None,
            None,
        );
        assert!(cuts.is_empty());
    }

    #[test]
    fn interpolated_bd_inherits_bridge_ineffective() {
        let mut reach_us = box_xs(0.0, 40.0, 0.0, 5.0);
        reach_us.station = 520.0;
        let mut reach_ds = box_xs(0.0, 40.0, 0.0, 5.0);
        reach_ds.station = 480.0;
        let mut stations = vec![520.0, 480.0];
        let table_us = reach_us.generate_lookup_table(20);
        let table_ds = reach_ds.generate_lookup_table(20);
        let mut tables = vec![table_us, table_ds];
        let mut z_mins = vec![
            cross_section_min_bed(&reach_us),
            cross_section_min_bed(&reach_ds),
        ];
        let mut xs: Vec<Option<CrossSection>> = vec![Some(reach_us), Some(reach_ds)];
        let faces = resolve_bridge_face_stations_metric(
            500.0,
            UnitSystem::Metric,
            None,
            None,
            20.0,
        );
        let bridge_opening =
            IneffectiveFlowAreas::from_block_pairs(&[], &[], &[8.0], &[3.0]).unwrap();
        let cuts = layout_cuts_for_bridge(
            &BridgeInteriorInput {
                opening_reach_station_origin: Some(0.0),
                ..Default::default()
            },
            faces,
            UnitSystem::Metric,
            None,
            Some(bridge_opening),
            None,
        );
        insert_reach_layout_cuts(
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
            &cuts,
            20,
            DensifyReachModifierPolicy::Upstream,
            UnitSystem::Metric,
            &mut [],
        );
        let bd_idx = stations
            .iter()
            .position(|&s| (s - faces.bd_station_m).abs() < 1e-6)
            .expect("BD node");
        let bd = xs[bd_idx].as_ref().expect("BD xs");
        let ineff = bd
            .ineffective_flow_areas
            .as_ref()
            .expect("BD bridge ineffective");
        assert_eq!(ineff.right_blocks.len(), 1);
        assert!((ineff.right_blocks[0].station - 8.0).abs() < 1e-9);
    }

    #[test]
    fn internal_layout_cut_inherits_reach_modifiers() {
        use crate::geometry::IneffectiveFlowAreas;

        let mut up = box_xs(0.0, 20.0, 0.0, 5.0);
        up.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[], &[], &[15.0], &[3.0]).unwrap(),
        );
        up.station = 510.0;
        let mut down = box_xs(0.0, 20.0, 0.0, 5.0);
        down.station = 490.0;
        let mut internal = box_xs(0.0, 20.0, 0.0, 5.0);
        internal.station = 500.0;
        let faces = BridgeFaceStations {
            bu_station_m: 510.0,
            bd_station_m: 490.0,
        };
        let cuts = layout_cuts_for_bridge(
            &BridgeInteriorInput {
                internal: vec![internal],
                ..Default::default()
            },
            faces,
            UnitSystem::Metric,
            None,
            None,
            None,
        );
        assert_eq!(cuts.len(), 3);
        assert_eq!(cuts[1].kind, BridgeLayoutCutKind::Internal);
        assert!(cuts[1].xs.is_some());

        let mut stations = vec![510.0, 490.0];
        let mut tables = vec![
            up.generate_lookup_table(20),
            down.generate_lookup_table(20),
        ];
        let mut z_mins = vec![
            cross_section_min_bed(&up),
            cross_section_min_bed(&down),
        ];
        let mut xs: Vec<Option<CrossSection>> = vec![Some(up), Some(down)];
        insert_reach_layout_cuts(
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
            &cuts,
            20,
            DensifyReachModifierPolicy::Upstream,
            UnitSystem::Metric,
            &mut [],
        );
        assert!(
            stations.iter().any(|&s| (s - 500.0).abs() < 1e-6),
            "explicit internal cut should land on grid"
        );
        assert!(stations.len() >= 3);
    }

    #[test]
    fn layout_insert_fallback_when_downstream_xs_missing() {
        let mut up = box_xs(0.0, 20.0, 0.0, 5.0);
        up.station = 520.0;
        let table = up.generate_lookup_table(20);
        let mut stations = vec![520.0, 480.0];
        let mut tables = vec![table.clone(), table.clone()];
        let mut z_mins = vec![0.0, 0.0];
        let mut xs: Vec<Option<CrossSection>> = vec![Some(up), None];
        let faces = resolve_bridge_face_stations_metric(
            500.0,
            UnitSystem::Metric,
            None,
            None,
            20.0,
        );
        let bridge_opening =
            IneffectiveFlowAreas::from_block_pairs(&[3.0], &[3.0], &[], &[]).unwrap();
        let cuts = layout_cuts_for_bridge(
            &BridgeInteriorInput {
                opening_reach_station_origin: Some(0.0),
                ..Default::default()
            },
            faces,
            UnitSystem::Metric,
            Some(bridge_opening),
            None,
            None,
        );
        insert_reach_layout_cuts(
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
            &cuts,
            20,
            DensifyReachModifierPolicy::None,
            UnitSystem::Metric,
            &mut [],
        );
        let bu_idx = stations
            .iter()
            .position(|&s| (s - faces.bu_station_m).abs() < 1e-6)
            .expect("BU inserted");
        let bu = xs[bu_idx].as_ref().expect("fallback BU xs");
        assert!(bu
            .ineffective_flow_areas
            .as_ref()
            .is_some_and(|i| i.is_configured()));
    }

    #[test]
    fn apply_bridge_reach_layout_steady_wires_ineffective() {
        use crate::solvers::steady::SteadyInputs;

        let mut up = box_xs(0.0, 10.0, 0.0, 5.0);
        up.station = 200.0;
        let mut down = box_xs(0.0, 10.0, 0.0, 5.0);
        down.station = 0.0;
        let mut stations = vec![200.0, 0.0];
        let mut tables = vec![
            up.generate_lookup_table(20),
            down.generate_lookup_table(20),
        ];
        let mut z_mins = vec![
            cross_section_min_bed(&up),
            cross_section_min_bed(&down),
        ];
        let mut xs: Vec<Option<CrossSection>> = vec![Some(up), Some(down)];
        let inputs = SteadyInputs {
            cross_sections: xs.iter().filter_map(|o| o.clone()).collect(),
            bridge_stations: Some(vec![100.0]),
            bridge_lengths: Some(vec![10.0]),
            bridge_ineffective_left_stations_upstream: Some(vec![vec![5.0]]),
            bridge_ineffective_left_elevations_upstream: Some(vec![vec![3.0]]),
            bridge_opening_reach_station_origins: Some(vec![0.0]),
            densify_reach_modifier_policy: Some(1),
            ..Default::default()
        };
        let intervals = apply_bridge_reach_layout_steady(
            &inputs,
            UnitSystem::Metric,
            20,
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
        );
        assert_eq!(intervals.len(), 1);
        assert!(intervals[0].is_some());
        assert!(stations.len() > 2);
    }

    #[test]
    fn apply_bridge_reach_layout_unsteady_interpolates_state() {
        use crate::solvers::unsteady::UnsteadyInputs;

        let mut up = box_xs(0.0, 10.0, 0.0, 5.0);
        up.station = 200.0;
        let mut down = box_xs(0.0, 10.0, 0.0, 5.0);
        down.station = 0.0;
        let mut stations = vec![200.0, 0.0];
        let mut tables = vec![
            up.generate_lookup_table(20),
            down.generate_lookup_table(20),
        ];
        let mut z_mins = vec![
            cross_section_min_bed(&up),
            cross_section_min_bed(&down),
        ];
        let mut xs = vec![up.clone(), down];
        let mut y = vec![2.0, 1.5];
        let mut q = vec![10.0, 10.0];
        let inputs = UnsteadyInputs {
            cross_sections: xs.clone(),
            initial_wsel: y.clone(),
            initial_q: q.clone(),
            dt: 10.0,
            num_steps: 1,
            upstream_q_hydrograph: vec![10.0],
            downstream_wsel_hydrograph: vec![1.5],
            theta: Some(0.6),
            num_slices: Some(20),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: crate::solvers::unsteady::UnsteadyCulvertInputs::default(),
            bridge: crate::solvers::unsteady::UnsteadyBridgeInputs {
                bridge_stations: Some(vec![100.0]),
                bridge_lengths: Some(vec![10.0]),
                ..Default::default()
            },
            structure_coupling_order: None,
        };
        let intervals = apply_bridge_reach_layout_unsteady(
            &inputs,
            UnitSystem::Metric,
            20,
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
            &mut y,
            &mut q,
        );
        assert_eq!(intervals.len(), 1);
        assert_eq!(xs.len(), stations.len());
        assert_eq!(y.len(), stations.len());
        assert_eq!(q.len(), stations.len());
    }

    #[test]
    fn insert_updates_existing_station_with_explicit_bu() {
        let mut bu = box_xs(50.0, 6.0, 0.0, 5.0);
        bu.station = 505.0;
        let faces = BridgeFaceStations {
            bu_station_m: 505.0,
            bd_station_m: 495.0,
        };
        let mut stations = vec![505.0, 400.0, 0.0];
        let table = flat_table();
        let mut tables = vec![table.clone(), table.clone(), table.clone()];
        let mut z_mins = vec![0.0; 3];
        let mut xs: Vec<Option<CrossSection>> = stations
            .iter()
            .map(|&st| {
                let mut s = box_xs(0.0, 20.0, 0.0, 5.0);
                s.station = st;
                Some(s)
            })
            .collect();
        let cuts = layout_cuts_for_bridge(
            &BridgeInteriorInput {
                bu: Some(bu.clone()),
                ..Default::default()
            },
            faces,
            UnitSystem::Metric,
            None,
            None,
            None,
        );
        insert_reach_layout_cuts(
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
            &cuts,
            20,
            DensifyReachModifierPolicy::None,
            UnitSystem::Metric,
            &mut [],
        );
        let idx = stations.iter().position(|&s| (s - 505.0).abs() < 1e-6).unwrap();
        assert!(tables[idx].interpolate(3.0).area < flat_table().interpolate(3.0).area);
    }
}
