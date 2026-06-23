//! HEC-RAS-style culvert inline structure reach: explicit US/DS bounding cross-sections.
//!
//! When `culvert_approach_reach_stations` / `culvert_departure_reach_stations` are supplied,
//! inserts upstream and downstream face nodes on the densified grid and couples the culvert on
//! the interval between those faces (mirroring bridge BU/BD layout).

use crate::geometry::{CrossSection, DensifyReachModifierPolicy, GeometryTable};
use crate::solvers::bridge_interior::{
    find_bridge_face_interval, insert_reach_layout_cuts, BridgeFaceStations, BridgeLayoutCut,
    BridgeLayoutCutKind,
};
use crate::utils::{structure_in_reach_interval, UnitSystem, FT_TO_M, STRUCTURE_STATION_TOL};

/// Upstream / downstream bounding face stations (metric, upstream ≥ downstream).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CulvertFaceStations {
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

/// Resolve US/DS bounding face stations for one culvert.
///
/// Precedence:
/// 1. Explicit `approach_reach_station` / `departure_reach_station` (user units)
/// 2. One bound + `culvert_length` for the other
/// 3. `center ± length/2`
pub fn resolve_culvert_face_stations_metric(
    center_station_user: f64,
    raw_units: UnitSystem,
    approach_reach_station_user: Option<f64>,
    departure_reach_station_user: Option<f64>,
    culvert_length_user: f64,
) -> CulvertFaceStations {
    let center_m = user_length_to_metric(center_station_user, raw_units);
    let length_m = user_length_to_metric(culvert_length_user, raw_units);

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

    CulvertFaceStations {
        us_station_m: us_m,
        ds_station_m: ds_m,
    }
}

fn layout_cuts_for_culvert(faces: CulvertFaceStations) -> Vec<BridgeLayoutCut> {
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

fn faces_to_bridge(faces: CulvertFaceStations) -> BridgeFaceStations {
    BridgeFaceStations {
        bu_station_m: faces.us_station_m,
        bd_station_m: faces.ds_station_m,
    }
}

/// Interval `i` spans US face (`stations[i]`) → DS face (`stations[i+1]`).
pub fn find_culvert_face_interval(
    faces: CulvertFaceStations,
    stations: &[f64],
) -> Option<usize> {
    find_bridge_face_interval(faces_to_bridge(faces), stations)
}

pub fn culvert_intervals_from_faces(
    face_intervals: &[Option<usize>],
) -> Vec<(usize, usize)> {
    face_intervals
        .iter()
        .enumerate()
        .filter_map(|(c_idx, interval)| interval.map(|i| (i, c_idx)))
        .collect()
}

fn culvert_length_user_steady(
    inputs: &crate::solvers::steady::SteadyInputs,
    c_idx: usize,
) -> f64 {
    inputs
        .culvert_lengths
        .as_ref()
        .and_then(|v| v.get(c_idx))
        .copied()
        .unwrap_or(0.0)
}

fn culvert_length_user_unsteady(
    c: &crate::solvers::unsteady::UnsteadyCulvertInputs,
    c_idx: usize,
) -> f64 {
    c.culvert_lengths
        .as_ref()
        .and_then(|v| v.get(c_idx))
        .copied()
        .unwrap_or(0.0)
}

fn approach_reach_user_steady(
    inputs: &crate::solvers::steady::SteadyInputs,
    c_idx: usize,
) -> Option<f64> {
    inputs
        .culvert_approach_reach_stations
        .as_ref()
        .and_then(|v| v.get(c_idx))
        .copied()
}

fn departure_reach_user_steady(
    inputs: &crate::solvers::steady::SteadyInputs,
    c_idx: usize,
) -> Option<f64> {
    inputs
        .culvert_departure_reach_stations
        .as_ref()
        .and_then(|v| v.get(c_idx))
        .copied()
}

fn approach_reach_user_unsteady(
    c: &crate::solvers::unsteady::UnsteadyCulvertInputs,
    c_idx: usize,
) -> Option<f64> {
    c.culvert_approach_reach_stations
        .as_ref()
        .and_then(|v| v.get(c_idx))
        .copied()
}

fn departure_reach_user_unsteady(
    c: &crate::solvers::unsteady::UnsteadyCulvertInputs,
    c_idx: usize,
) -> Option<f64> {
    c.culvert_departure_reach_stations
        .as_ref()
        .and_then(|v| v.get(c_idx))
        .copied()
}

pub fn culvert_has_explicit_bounds_steady(
    inputs: &crate::solvers::steady::SteadyInputs,
    c_idx: usize,
) -> bool {
    approach_reach_user_steady(inputs, c_idx).is_some()
        || departure_reach_user_steady(inputs, c_idx).is_some()
}

pub fn culvert_has_explicit_bounds_unsteady(
    c: &crate::solvers::unsteady::UnsteadyCulvertInputs,
    c_idx: usize,
) -> bool {
    approach_reach_user_unsteady(c, c_idx).is_some()
        || departure_reach_user_unsteady(c, c_idx).is_some()
}

/// Insert US/DS bounding nodes and return culvert interval index per culvert.
pub fn apply_culvert_reach_layout_steady(
    inputs: &crate::solvers::steady::SteadyInputs,
    raw_units: UnitSystem,
    num_slices: usize,
    stations: &mut Vec<f64>,
    tables: &mut Vec<GeometryTable>,
    z_mins: &mut Vec<f64>,
    xs: &mut Vec<Option<CrossSection>>,
) -> Vec<Option<usize>> {
    let Some(ref centers) = inputs.culvert_stations else {
        return vec![];
    };

    let mut all_cuts = Vec::new();
    let mut face_list = Vec::with_capacity(centers.len());

    for (c_idx, &center) in centers.iter().enumerate() {
        let faces = resolve_culvert_face_stations_metric(
            center,
            raw_units,
            approach_reach_user_steady(inputs, c_idx),
            departure_reach_user_steady(inputs, c_idx),
            culvert_length_user_steady(inputs, c_idx),
        );
        face_list.push(faces);
        if culvert_has_explicit_bounds_steady(inputs, c_idx) {
            all_cuts.extend(layout_cuts_for_culvert(faces));
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
        .map(|(c_idx, faces)| {
            if culvert_has_explicit_bounds_steady(inputs, c_idx) {
                find_culvert_face_interval(*faces, stations)
            } else {
                fallback_culvert_interval(*faces, centers[c_idx], raw_units, stations)
            }
        })
        .collect()
}

pub fn apply_culvert_reach_layout_unsteady(
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
    let c = &inputs.culvert;
    let Some(ref centers) = c.culvert_stations else {
        return vec![];
    };

    let mut all_cuts = Vec::new();
    let mut face_list = Vec::with_capacity(centers.len());

    for (c_idx, &center) in centers.iter().enumerate() {
        let faces = resolve_culvert_face_stations_metric(
            center,
            raw_units,
            approach_reach_user_unsteady(c, c_idx),
            departure_reach_user_unsteady(c, c_idx),
            culvert_length_user_unsteady(c, c_idx),
        );
        face_list.push(faces);
        if culvert_has_explicit_bounds_unsteady(c, c_idx) {
            all_cuts.extend(layout_cuts_for_culvert(faces));
        }
    }

    if !all_cuts.is_empty() {
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
    }

    face_list
        .iter()
        .enumerate()
        .map(|(c_idx, faces)| {
            if culvert_has_explicit_bounds_unsteady(c, c_idx) {
                find_culvert_face_interval(*faces, stations)
            } else {
                fallback_culvert_interval(*faces, centers[c_idx], raw_units, stations)
            }
        })
        .collect()
}

fn fallback_culvert_interval(
    faces: CulvertFaceStations,
    center_user: f64,
    raw_units: UnitSystem,
    stations: &[f64],
) -> Option<usize> {
    if let Some(i) = find_culvert_face_interval(faces, stations) {
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
    use crate::geometry::CrossSection;

    fn flat_xs(station: f64) -> CrossSection {
        CrossSection {
            station,
            x: vec![0.0, 10.0, 20.0],
            y: vec![5.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
            coeff_contraction: None,
            coeff_expansion: None,
        }
    }

    #[test]
    fn resolve_faces_from_explicit_bounds() {
        let faces = resolve_culvert_face_stations_metric(
            500.0,
            UnitSystem::Metric,
            Some(550.0),
            Some(450.0),
            100.0,
        );
        assert!((faces.us_station_m - 550.0).abs() < 1e-9);
        assert!((faces.ds_station_m - 450.0).abs() < 1e-9);
    }

    #[test]
    fn apply_layout_inserts_bounding_nodes() {
        let mut stations = vec![600.0, 400.0, 200.0];
        let mut tables: Vec<GeometryTable> = stations
            .iter()
            .map(|_| flat_xs(0.0).generate_lookup_table(20))
            .collect();
        let mut z_mins: Vec<f64> = stations.iter().map(|_| 0.0).collect();
        let mut xs: Vec<Option<CrossSection>> = stations
            .iter()
            .map(|&st| Some(flat_xs(st)))
            .collect();

        let inputs = crate::solvers::steady::SteadyInputs {
            culvert_stations: Some(vec![500.0]),
            culvert_lengths: Some(vec![100.0]),
            culvert_approach_reach_stations: Some(vec![550.0]),
            culvert_departure_reach_stations: Some(vec![450.0]),
            ..Default::default()
        };

        let intervals = apply_culvert_reach_layout_steady(
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
        assert!(stations.iter().any(|&s| (s - 550.0).abs() < 1e-6));
        assert!(stations.iter().any(|&s| (s - 450.0).abs() < 1e-6));
        let i = intervals[0].unwrap();
        assert!((stations[i] - 550.0).abs() < 1e-6);
        assert!((stations[i + 1] - 450.0).abs() < 1e-6);
    }
}
