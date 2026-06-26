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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_face_stations() {
        // (Some, Some)
        let f1 = resolve_inline_structure_face_stations_metric(50.0, UnitSystem::Metric, Some(55.0), Some(45.0), 10.0);
        assert_eq!(f1.us_station_m, 55.0);
        assert_eq!(f1.ds_station_m, 45.0);

        // (Some, None)
        let f2 = resolve_inline_structure_face_stations_metric(50.0, UnitSystem::Metric, Some(55.0), None, 10.0);
        assert_eq!(f2.us_station_m, 55.0);
        assert_eq!(f2.ds_station_m, 45.0);

        let f2_zero = resolve_inline_structure_face_stations_metric(50.0, UnitSystem::Metric, Some(55.0), None, 0.0);
        assert_eq!(f2_zero.us_station_m, 55.0);
        assert_eq!(f2_zero.ds_station_m, 55.0);

        // (None, Some)
        let f3 = resolve_inline_structure_face_stations_metric(50.0, UnitSystem::Metric, None, Some(45.0), 10.0);
        assert_eq!(f3.us_station_m, 55.0);
        assert_eq!(f3.ds_station_m, 45.0);

        let f3_zero = resolve_inline_structure_face_stations_metric(50.0, UnitSystem::Metric, None, Some(45.0), 0.0);
        assert_eq!(f3_zero.us_station_m, 45.0);
        assert_eq!(f3_zero.ds_station_m, 45.0);

        // (None, None)
        let f4 = resolve_inline_structure_face_stations_metric(50.0, UnitSystem::Metric, None, None, 10.0);
        assert_eq!(f4.us_station_m, 55.0);
        assert_eq!(f4.ds_station_m, 45.0);

        let f4_zero = resolve_inline_structure_face_stations_metric(50.0, UnitSystem::Metric, None, None, 0.0);
        assert_eq!(f4_zero.us_station_m, 50.0);
        assert_eq!(f4_zero.ds_station_m, 50.0);

        // USCustomary conversion
        let f_us = resolve_inline_structure_face_stations_metric(50.0, UnitSystem::USCustomary, None, None, 10.0);
        assert!((f_us.us_station_m - 55.0 * FT_TO_M).abs() < 1e-6);
    }

    #[test]
    fn test_layout_cuts() {
        let f_equal = InlineStructureFaceStations { us_station_m: 50.0, ds_station_m: 50.0 };
        assert!(layout_cuts_for_inline_structure(f_equal).is_empty());

        let f_diff = InlineStructureFaceStations { us_station_m: 55.0, ds_station_m: 45.0 };
        let cuts = layout_cuts_for_inline_structure(f_diff);
        assert_eq!(cuts.len(), 2);
        assert_eq!(cuts[0].station_m, 55.0);
        assert_eq!(cuts[1].station_m, 45.0);
    }

    #[test]
    fn test_fallback_interval() {
        let stations_exact = vec![100.0, 60.0, 40.0, 0.0];
        let faces = InlineStructureFaceStations { us_station_m: 60.0, ds_station_m: 40.0 };
        let res_exact = fallback_inline_structure_interval(faces, 50.0, UnitSystem::Metric, &stations_exact);
        assert_eq!(res_exact, Some(1));

        let stations = vec![100.0, 50.0, 0.0];
        let res = fallback_inline_structure_interval(faces, 50.0, UnitSystem::Metric, &stations);
        assert!(res.is_some());
        
        let res_none = fallback_inline_structure_interval(faces, 150.0, UnitSystem::Metric, &stations);
        assert_eq!(res_none, None);
    }

    #[test]
    fn test_apply_layout_empty() {
        let inputs = crate::solvers::steady::SteadyInputs::default();
        let mut stations = vec![100.0, 0.0];
        let mut tables = vec![GeometryTable::default(), GeometryTable::default()];
        let mut z_mins = vec![0.0, 0.0];
        let mut xs = vec![None, None];
        let res = apply_inline_structure_reach_layout_steady(
            &inputs,
            UnitSystem::Metric,
            50,
            &mut stations,
            &mut tables,
            &mut z_mins,
            &mut xs,
        );
        assert!(res.is_empty());
    }
}
