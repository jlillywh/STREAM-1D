//! HEC-RAS BU / BD / internal bridge cross-section resolution (API v22).

use crate::geometry::{
    resolve_guide_banks, CrossSection, GeometryTable, GuideBanks, IneffectiveBlock,
    IneffectiveFlowAreas,
};
use crate::solvers::bridge::BridgeSectionContext;
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
    interior: &BridgeInteriorInput,
    anchor_reach_xs: Option<&CrossSection>,
    reach_xs_up: Option<&CrossSection>,
    reach_xs_down: Option<&CrossSection>,
    reach_table_up: &GeometryTable,
    reach_table_down: &GeometryTable,
    reach_z_up_user: f64,
    reach_z_down_user: f64,
    raw_units: UnitSystem,
    num_slices: usize,
    ineffective_up: Option<crate::geometry::IneffectiveFlowAreas>,
    ineffective_down: Option<crate::geometry::IneffectiveFlowAreas>,
    skew_deg: f64,
    pier_stations: Option<Vec<f64>>,
    interval_length_m: f64,
    bridge_length_user: f64,
    approach_xs: Option<CrossSection>,
    departure_xs: Option<CrossSection>,
    guide_banks_approach: Option<GuideBanks>,
    guide_banks_departure: Option<GuideBanks>,
) -> BridgeFaceSolveGeometry {
    let xs_up = interior
        .bu
        .clone()
        .or_else(|| reach_xs_up.cloned());
    let xs_down = interior
        .bd
        .clone()
        .or_else(|| reach_xs_down.cloned());

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

    let opening_origin = resolve_opening_reach_station_origin(
        interior.opening_reach_station_origin,
        interior.opening_anchor_mode,
        xs_up.as_ref(),
        anchor_reach_xs,
        reach_xs_up,
    );

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

    let friction_length_m = resolve_bridge_friction_length_metric(
        interior,
        interval_length_m,
        bridge_length_user,
        raw_units,
    );

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
        friction_length_m,
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

/// One densification node to insert at a bridge layout river station.
#[derive(Debug, Clone)]
pub struct BridgeLayoutCut {
    pub station_m: f64,
    pub xs: Option<CrossSection>,
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
    });
    for xs in &interior.internal {
        cuts.push(BridgeLayoutCut {
            station_m: xs_river_station_to_metric(xs.station, xs.unit_system, raw_units),
            xs: Some(xs.clone()),
        });
    }
    cuts.push(BridgeLayoutCut {
        station_m: faces.bd_station_m,
        xs: interior.bd.clone(),
    });
    cuts
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
            let (table_interp, z_interp) = crate::geometry::processor::interpolate_geometry_table(
                &tables[i],
                z_mins[i],
                &tables[i + 1],
                z_mins[i + 1],
                t,
                num_slices,
            );
            let xs_new = xs[i]
                .clone()
                .or_else(|| xs.get(i + 1).and_then(|x| x.clone()))
                .map(|mut section| {
                    section.station = cut.station_m;
                    section
                });
            (table_interp, z_interp, xs_new)
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
        all_cuts.extend(layout_cuts_for_bridge(&interior, faces, raw_units));
    }

    insert_reach_layout_cuts(
        stations,
        tables,
        z_mins,
        xs,
        &all_cuts,
        num_slices,
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
        all_cuts.extend(layout_cuts_for_bridge(&interior, faces, raw_units));
    }

    let mut xs_opt: Vec<Option<CrossSection>> = xs.iter().cloned().map(Some).collect();
    insert_reach_layout_cuts(
        stations,
        tables,
        z_mins,
        &mut xs_opt,
        &all_cuts,
        num_slices,
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
        let geo = resolve_bridge_face_solve_geometry(
            &interior,
            None,
            Some(&reach),
            Some(&reach),
            &flat_table(),
            &flat_table(),
            0.0,
            0.0,
            UnitSystem::Metric,
            30,
            None,
            None,
            0.0,
            None,
            0.0,
            0.0,
            None,
            None,
            None,
            None,
        );
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
        );
        insert_reach_layout_cuts(
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
            &cuts,
            20,
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
        let geo = resolve_bridge_face_solve_geometry(
            &interior,
            None,
            None,
            None,
            &flat_table(),
            &flat_table(),
            0.0,
            0.0,
            UnitSystem::Metric,
            20,
            None,
            None,
            0.0,
            None,
            0.0,
            0.0,
            None,
            None,
            None,
            None,
        );
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

        let geo = resolve_bridge_face_solve_geometry(
            &interior,
            None,
            Some(&reach),
            Some(&reach),
            &flat_table(),
            &flat_table(),
            0.0,
            0.0,
            UnitSystem::Metric,
            30,
            Some(bridge_opening),
            None,
            0.0,
            None,
            0.0,
            0.0,
            None,
            None,
            None,
            None,
        );

        let up = geo
            .sections
            .ineffective_up
            .expect("BU section ineffective should apply");
        assert_eq!(up.left_blocks.len(), 1);
        assert!((up.left_blocks[0].station - 12.0).abs() < 1e-9);
    }

    #[test]
    fn reach_fallback_uses_reach_xs_ineffective() {
        let mut reach = box_xs(0.0, 40.0, 0.0, 5.0);
        reach.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[25.0], &[3.0], &[], &[]).unwrap(),
        );

        let geo = resolve_bridge_face_solve_geometry(
            &BridgeInteriorInput::default(),
            None,
            Some(&reach),
            Some(&reach),
            &flat_table(),
            &flat_table(),
            0.0,
            0.0,
            UnitSystem::Metric,
            30,
            None,
            None,
            0.0,
            None,
            0.0,
            0.0,
            None,
            None,
            None,
            None,
        );

        let up = geo.sections.ineffective_up.expect("reach ineffective");
        assert!((up.left_blocks[0].station - 25.0).abs() < 1e-9);
    }

    #[test]
    fn bridge_opening_ineffective_shifted_to_reach_x() {
        let reach = box_xs(100.0, 40.0, 0.0, 5.0);
        let bridge_opening = IneffectiveFlowAreas::from_block_pairs(&[5.0], &[3.0], &[], &[]).unwrap();

        let geo = resolve_bridge_face_solve_geometry(
            &BridgeInteriorInput {
                opening_reach_station_origin: Some(100.0),
                ..Default::default()
            },
            None,
            Some(&reach),
            None,
            &flat_table(),
            &flat_table(),
            0.0,
            0.0,
            UnitSystem::Metric,
            30,
            Some(bridge_opening),
            None,
            0.0,
            None,
            0.0,
            0.0,
            None,
            None,
            None,
            None,
        );

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
        let geo = resolve_bridge_face_solve_geometry(
            &interior,
            None,
            None,
            None,
            &flat_table(),
            &flat_table(),
            0.0,
            0.0,
            UnitSystem::Metric,
            20,
            None,
            None,
            0.0,
            None,
            6.0,
            50.0,
            None,
            None,
            None,
            None,
        );
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
        let geo = resolve_bridge_face_solve_geometry(
            &interior,
            Some(&approach),
            None,
            None,
            &flat_table(),
            &flat_table(),
            0.0,
            0.0,
            UnitSystem::Metric,
            20,
            None,
            None,
            0.0,
            None,
            0.0,
            0.0,
            None,
            None,
            None,
            None,
        );
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

    fn pier_stations_remapped_to_reach_frame() {
        let bu = box_xs(100.0, 30.0, 1.0, 6.0);
        let interior = BridgeInteriorInput {
            bu: Some(bu),
            bd: None,
            internal: vec![],
            opening_reach_station_origin: Some(100.0),
            ..Default::default()
        };
        let geo = resolve_bridge_face_solve_geometry(
            &interior,
            None,
            None,
            None,
            &flat_table(),
            &flat_table(),
            0.0,
            0.0,
            UnitSystem::Metric,
            20,
            None,
            None,
            0.0,
            Some(vec![5.0, 20.0]),
            0.0,
            0.0,
            None,
            None,
            None,
            None,
        );
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
}
