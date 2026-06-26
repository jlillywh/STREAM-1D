//! HEC-RAS-style inline structure reach: explicit US/DS bounding cross-sections.

use crate::geometry::{CrossSection, DensifyReachModifierPolicy, GeometryTable};
use crate::solvers::bridge_interior::{
    find_bridge_face_interval, insert_reach_layout_cuts, BridgeFaceStations, BridgeLayoutCut,
    BridgeLayoutCutKind,
};
use crate::utils::{structure_in_reach_interval, UnitSystem, FT_TO_M, STRUCTURE_STATION_TOL};

/// Upstream / downstream bounding face stations (metric, upstream ≥ downstream).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InlineStructureFaceStations {
    pub us_station_m: f64,
    pub ds_station_m: f64,
}

fn user_length_to_metric(value: f64, raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        value * FT_TO_M
    } else {
        value
    }
}

pub fn resolve_inline_structure_face_stations_metric(
    center_station_user: f64,
    raw_units: UnitSystem,
    approach_reach_station_user: Option<f64>,
    departure_reach_station_user: Option<f64>,
    structure_length_user: f64,
) -> InlineStructureFaceStations {
    let center_m = user_length_to_metric(center_station_user, raw_units);
    let length_m = user_length_to_metric(structure_length_user, raw_units);

    let (us_m, ds_m) = match (
        approach_reach_station_user,
        departure_reach_station_user,
    ) {
        (Some(us), Some(ds)) => {
            let us_m = user_length_to_metric(us, raw_units);
            let ds_m = user_length_to_metric(ds, raw_units);
            (us_m.max(ds_m), us_m.min(ds_m))
        }
        (Some(us), None) => {
            let us_m = user_length_to_metric(us, raw_units);
            let ds_m = if length_m > 0.0 {
                us_m - length_m
            } else {
                us_m
            };
            (us_m, ds_m)
        }
        (None, Some(ds)) => {
            let ds_m = user_length_to_metric(ds, raw_units);
            let us_m = if length_m > 0.0 {
                ds_m + length_m
            } else {
                ds_m
            };
            (us_m, ds_m)
        }
        (None, None) => {
            if length_m > 0.0 {
                (center_m + length_m * 0.5, center_m - length_m * 0.5)
            } else {
                (center_m, center_m)
            }
        }
    };

    InlineStructureFaceStations {
        us_station_m: us_m,
        ds_station_m: ds_m,
    }
}

fn layout_cuts_for_inline_structure(faces: InlineStructureFaceStations) -> Vec<BridgeLayoutCut> {
    if (faces.us_station_m - faces.ds_station_m).abs() <= STRUCTURE_STATION_TOL {
        return vec![];
    }

    vec![
        BridgeLayoutCut {
            station_m: faces.us_station_m,
            xs: None,
            kind: BridgeLayoutCutKind::Internal,
            face_meta: None,
        },
        BridgeLayoutCut {
            station_m: faces.ds_station_m,
            xs: None,
            kind: BridgeLayoutCutKind::Internal,
            face_meta: None,
        },
    ]
}

fn faces_to_bridge(faces: InlineStructureFaceStations) -> BridgeFaceStations {
    BridgeFaceStations {
        bu_station_m: faces.us_station_m,
        bd_station_m: faces.ds_station_m,
    }
}

pub fn find_inline_structure_face_interval(
    faces: InlineStructureFaceStations,
    stations: &[f64],
) -> Option<usize> {
    find_bridge_face_interval(faces_to_bridge(faces), stations)
}

pub fn inline_structure_intervals_from_faces(
    face_intervals: &[Option<usize>],
) -> Vec<(usize, usize)> {
    face_intervals
        .iter()
        .enumerate()
        .filter_map(|(is_idx, interval)| interval.map(|i| (i, is_idx)))
        .collect()
}

pub fn apply_inline_structure_reach_layout_steady(
    inputs: &crate::solvers::steady::SteadyInputs,
    raw_units: UnitSystem,
    num_slices: usize,
    stations: &mut Vec<f64>,
    tables: &mut Vec<GeometryTable>,
    z_mins: &mut Vec<f64>,
    xs: &mut Vec<Option<CrossSection>>,
) -> Vec<Option<usize>> {
    let Some(ref centers) = inputs.inline_structure_stations else {
        return vec![];
    };

    let mut all_cuts = Vec::new();
    let mut face_list = Vec::with_capacity(centers.len());

    for (is_idx, &center) in centers.iter().enumerate() {
        let approach = inputs.inline_structure_approach_reach_stations.as_ref().and_then(|v| v.get(is_idx)).copied();
        let departure = inputs.inline_structure_departure_reach_stations.as_ref().and_then(|v| v.get(is_idx)).copied();
        let length = inputs.inline_structure_lengths.as_ref().and_then(|v| v.get(is_idx)).copied().unwrap_or(0.0);

        let faces = resolve_inline_structure_face_stations_metric(
            center,
            raw_units,
            approach,
            departure,
            length,
        );
        face_list.push(faces);
        let has_explicit = approach.is_some() || departure.is_some() || length > 0.0;
        if has_explicit {
            all_cuts.extend(layout_cuts_for_inline_structure(faces));
        }
    }

    if !all_cuts.is_empty() {
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
    }

    face_list
        .iter()
        .enumerate()
        .map(|(is_idx, faces)| {
            let approach = inputs.inline_structure_approach_reach_stations.as_ref().and_then(|v| v.get(is_idx)).copied();
            let departure = inputs.inline_structure_departure_reach_stations.as_ref().and_then(|v| v.get(is_idx)).copied();
            let length = inputs.inline_structure_lengths.as_ref().and_then(|v| v.get(is_idx)).copied().unwrap_or(0.0);
            let has_explicit = approach.is_some() || departure.is_some() || length > 0.0;

            if has_explicit {
                find_inline_structure_face_interval(*faces, stations)
            } else {
                fallback_inline_structure_interval(*faces, centers[is_idx], raw_units, stations)
            }
        })
        .collect()
}

fn fallback_inline_structure_interval(
    faces: InlineStructureFaceStations,
    center_user: f64,
    raw_units: UnitSystem,
    stations: &[f64],
) -> Option<usize> {
    if let Some(i) = find_inline_structure_face_interval(faces, stations) {
        return Some(i);
    }
    let center_m = user_length_to_metric(center_user, raw_units);
    for i in 0..stations.len().saturating_sub(1) {
        if structure_in_reach_interval(center_m, stations, i) {
            return Some(i);
        }
    }
    None
}
