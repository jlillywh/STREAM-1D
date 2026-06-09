use crate::geometry::CrossSection;
use crate::solvers::steady::{solve_steady_single_reach, SteadyInputs, SteadyResult};

const STATION_TOL: f64 = 1e-4;

pub fn has_tributary_junction(inputs: &SteadyInputs) -> bool {
    inputs
        .tributary_cross_sections
        .as_ref()
        .map(|xs| !xs.is_empty())
        .unwrap_or(false)
        && inputs.tributary_flow_rate.is_some()
        && inputs.junction_main_station.is_some()
}

fn strip_junction_fields(inputs: &SteadyInputs) -> SteadyInputs {
    SteadyInputs {
        tributary_cross_sections: None,
        tributary_flow_rate: None,
        junction_main_station: None,
        ..inputs.clone()
    }
}

fn find_section_index(sections: &[CrossSection], station: f64) -> Option<usize> {
    sections
        .iter()
        .position(|xs| (xs.station - station).abs() < STATION_TOL)
}

fn filter_main_downstream(sections: &[CrossSection], junction_station: f64) -> Vec<CrossSection> {
    let mut out: Vec<CrossSection> = sections
        .iter()
        .filter(|xs| xs.station <= junction_station + STATION_TOL)
        .cloned()
        .collect();
    out.sort_by(|a, b| b.station.partial_cmp(&a.station).unwrap());
    out
}

fn filter_main_upstream(sections: &[CrossSection], junction_station: f64) -> Vec<CrossSection> {
    let mut out: Vec<CrossSection> = sections
        .iter()
        .filter(|xs| xs.station >= junction_station - STATION_TOL)
        .cloned()
        .collect();
    out.sort_by(|a, b| b.station.partial_cmp(&a.station).unwrap());
    out
}

fn wsel_at_station(result: &SteadyResult, sections: &[CrossSection], station: f64) -> Option<f64> {
    find_section_index(sections, station).map(|idx| result.wsel[idx])
}

/// Steady subcritical junction: tributary joins main channel at a shared WSEL node.
///
/// `flow_rate` is main-channel discharge upstream of the junction.
/// `tributary_flow_rate` is added below the junction on the main stem.
pub fn solve_steady_junction(inputs: &SteadyInputs) -> SteadyResult {
    let junction_station = inputs.junction_main_station.unwrap();
    let q_trib = inputs.tributary_flow_rate.unwrap();
    let q_main = inputs.flow_rate;

    find_section_index(&inputs.cross_sections, junction_station).expect(
        "junction_main_station must match a main-channel cross-section station",
    );

    let trib_sections = inputs
        .tributary_cross_sections
        .as_ref()
        .expect("tributary_cross_sections required")
        .clone();

    assert!(
        !trib_sections.is_empty(),
        "tributary_cross_sections must not be empty"
    );

    // 1) Main channel downstream of junction (includes junction node): Q_main + Q_trib
    let ds_sections = filter_main_downstream(&inputs.cross_sections, junction_station);
    assert!(
        ds_sections.len() >= 2,
        "Need at least two main cross-sections at/below the junction"
    );

    let mut ds_inputs = strip_junction_fields(inputs);
    ds_inputs.cross_sections = ds_sections.clone();
    ds_inputs.flow_rate = q_main + q_trib;
    ds_inputs.regime = 0;
    ds_inputs.upstream_wsel = None;
    ds_inputs.upstream_bc_type = None;

    let ds_result = solve_steady_single_reach(&ds_inputs);
    let junction_wsel = wsel_at_station(&ds_result, &ds_sections, junction_station)
        .expect("junction WSEL missing from downstream main solve");

    // 2) Main channel upstream of junction: Q_main, downstream BC = junction WSEL
    let us_sections = filter_main_upstream(&inputs.cross_sections, junction_station);
    assert!(
        us_sections.len() >= 1,
        "Need at least one main cross-section at/above the junction"
    );

    let mut us_inputs = strip_junction_fields(inputs);
    us_inputs.cross_sections = us_sections.clone();
    us_inputs.flow_rate = q_main;
    us_inputs.regime = 0;
    us_inputs.downstream_wsel = Some(junction_wsel);
    us_inputs.downstream_bc_type = Some(0);

    let us_result = solve_steady_single_reach(&us_inputs);

    // 3) Tributary reach: mouth at minimum station, downstream BC = junction WSEL
    let mut trib_inputs = strip_junction_fields(inputs);
    trib_inputs.cross_sections = trib_sections.clone();
    trib_inputs.flow_rate = q_trib;
    trib_inputs.regime = 0;
    trib_inputs.downstream_wsel = Some(junction_wsel);
    trib_inputs.downstream_bc_type = Some(0);
    trib_inputs.upstream_wsel = None;
    trib_inputs.upstream_bc_type = None;
    trib_inputs.culvert_stations = None;
    trib_inputs.bridge_stations = None;

    let trib_result = solve_steady_single_reach(&trib_inputs);

    // 4) Merge main-channel profiles back into original cross-section order
    let mut out_wsel = vec![0.0; inputs.cross_sections.len()];
    let mut out_yc = vec![0.0; inputs.cross_sections.len()];
    let mut out_vel = vec![0.0; inputs.cross_sections.len()];
    let mut out_area = vec![0.0; inputs.cross_sections.len()];
    let mut out_fr = vec![0.0; inputs.cross_sections.len()];
    let mut out_top_width = vec![0.0; inputs.cross_sections.len()];
    let mut out_eg_slope = vec![0.0; inputs.cross_sections.len()];

    for (orig_idx, xs) in inputs.cross_sections.iter().enumerate() {
        let st = xs.station;
        let below = st <= junction_station + STATION_TOL;
        let above = st >= junction_station - STATION_TOL;

        let (src, src_idx) = if (st - junction_station).abs() < STATION_TOL {
            // Junction node: prefer downstream combined-flow solve
            (
                &ds_result,
                find_section_index(&ds_sections, st).unwrap(),
            )
        } else if below {
            (
                &ds_result,
                find_section_index(&ds_sections, st)
                    .unwrap_or_else(|| panic!("missing downstream section {}", st)),
            )
        } else if above {
            (
                &us_result,
                find_section_index(&us_sections, st)
                    .unwrap_or_else(|| panic!("missing upstream section {}", st)),
            )
        } else {
            panic!("cross-section station {} not in main network", st);
        };

        out_wsel[orig_idx] = src.wsel[src_idx];
        out_yc[orig_idx] = src.critical_wsel[src_idx];
        out_vel[orig_idx] = src.velocity[src_idx];
        out_area[orig_idx] = src.area[src_idx];
        out_fr[orig_idx] = src.froude[src_idx];
        out_top_width[orig_idx] = src.top_width[src_idx];
        out_eg_slope[orig_idx] = src.eg_slope[src_idx];
    }

    let culvert_control_types = merge_culvert_controls(
        inputs.culvert_stations.as_deref(),
        ds_result.culvert_control_types.as_deref(),
        us_result.culvert_control_types.as_deref(),
    );
    let culvert_wsel_inlet = merge_culvert_f64(
        inputs.culvert_stations.as_deref(),
        ds_result.culvert_wsel_inlet.as_deref(),
        us_result.culvert_wsel_inlet.as_deref(),
    );
    let culvert_wsel_outlet = merge_culvert_f64(
        inputs.culvert_stations.as_deref(),
        ds_result.culvert_wsel_outlet.as_deref(),
        us_result.culvert_wsel_outlet.as_deref(),
    );
    let culvert_q_barrels = merge_culvert_f64(
        inputs.culvert_stations.as_deref(),
        ds_result.culvert_q_barrels.as_deref(),
        us_result.culvert_q_barrels.as_deref(),
    );
    let culvert_q_weirs = merge_culvert_f64(
        inputs.culvert_stations.as_deref(),
        ds_result.culvert_q_weirs.as_deref(),
        us_result.culvert_q_weirs.as_deref(),
    );
    let culvert_barrel_depths = merge_culvert_f64(
        inputs.culvert_stations.as_deref(),
        ds_result.culvert_barrel_depths.as_deref(),
        us_result.culvert_barrel_depths.as_deref(),
    );
    let culvert_barrel_velocities = merge_culvert_f64(
        inputs.culvert_stations.as_deref(),
        ds_result.culvert_barrel_velocities.as_deref(),
        us_result.culvert_barrel_velocities.as_deref(),
    );
    let culvert_barrel_froude = merge_culvert_f64(
        inputs.culvert_stations.as_deref(),
        ds_result.culvert_barrel_froude.as_deref(),
        us_result.culvert_barrel_froude.as_deref(),
    );

    SteadyResult {
        wsel: out_wsel,
        critical_wsel: out_yc,
        velocity: out_vel,
        area: out_area,
        froude: out_fr,
        top_width: out_top_width,
        eg_slope: out_eg_slope,
        tributary_wsel: Some(trib_result.wsel),
        tributary_velocity: Some(trib_result.velocity),
        tributary_froude: Some(trib_result.froude),
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

fn merge_culvert_f64(
    stations: Option<&[f64]>,
    ds: Option<&[f64]>,
    us: Option<&[f64]>,
) -> Option<Vec<f64>> {
    let n = stations?.len();
    if n == 0 {
        return None;
    }
    let mut out = vec![0.0; n];
    for i in 0..n {
        if let Some(ds_vals) = ds {
            if i < ds_vals.len() && ds_vals[i].abs() > 1e-12 {
                out[i] = ds_vals[i];
            }
        }
        if let Some(us_vals) = us {
            if i < us_vals.len() && us_vals[i].abs() > 1e-12 {
                out[i] = us_vals[i];
            }
        }
    }
    if out.iter().all(|v| v.abs() < 1e-12) {
        None
    } else {
        Some(out)
    }
}

fn merge_culvert_controls(
    stations: Option<&[f64]>,
    ds: Option<&[String]>,
    us: Option<&[String]>,
) -> Option<Vec<String>> {
    let n = stations?.len();
    if n == 0 {
        return None;
    }
    let mut out = vec![String::new(); n];
    for i in 0..n {
        if let Some(ds_types) = ds {
            if i < ds_types.len() && !ds_types[i].is_empty() {
                out[i] = ds_types[i].clone();
            }
        }
        if let Some(us_types) = us {
            if i < us_types.len() && !us_types[i].is_empty() {
                out[i] = us_types[i].clone();
            }
        }
    }
    if out.iter().all(|s| s.is_empty()) {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::UnitSystem;

    fn rect(station: f64, bed: f64, n: f64) -> CrossSection {
        CrossSection {
            station,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![bed + 5.0, bed, bed, bed + 5.0],
            n_stations: vec![0.0],
            n_values: vec![n],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        }
    }

    #[test]
    fn test_merge_culvert_controls() {
        let merged = merge_culvert_controls(
            Some(&[100.0, 50.0]),
            Some(&[String::new(), "outlet".to_string()]),
            Some(&["inlet".to_string(), String::new()]),
        )
        .unwrap();
        assert_eq!(merged, vec!["inlet", "outlet"]);
        assert!(merge_culvert_controls(
            Some(&[100.0]),
            Some(&[String::new()]),
            Some(&[String::new()])
        )
        .is_none());
    }

    fn us_rect(station: f64, bed: f64, n: f64) -> CrossSection {
        CrossSection {
            station,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![bed + 5.0, bed, bed, bed + 5.0],
            n_stations: vec![0.0],
            n_values: vec![n],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        }
    }

    #[test]
    fn test_junction_culvert_control_types() {
        let main = vec![
            us_rect(1000.0, 2.0, 0.02),
            us_rect(500.0, 1.0, 0.02),
            us_rect(100.0, 1.0, 0.02),
            us_rect(0.0, 0.0, 0.02),
        ];
        let trib = vec![us_rect(800.0, 1.5, 0.03), us_rect(400.0, 1.0, 0.03)];

        let inputs = SteadyInputs {
            cross_sections: main,
            flow_rate: 10.0,
            num_slices: Some(50),
            regime: 0,
            downstream_wsel: Some(1.5),
            downstream_bc_type: Some(0),
            tributary_cross_sections: Some(trib),
            tributary_flow_rate: Some(5.0),
            junction_main_station: Some(500.0),
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
        };

        let result = solve_steady_junction(&inputs);
        let controls = result
            .culvert_control_types
            .as_ref()
            .expect("culvert_control_types on junction run");
        assert_eq!(controls.len(), 1);
        assert_eq!(controls[0], "inlet");
    }

    #[test]
    fn test_tributary_junction_adds_downstream_flow() {
        // Main: 1000 -> 500 (junction) -> 0, Q_main = 10 cms above junction
        // Trib: 800 -> 600 -> 400 (mouth), Q_trib = 5 cms
        let main = vec![rect(1000.0, 0.2, 0.025), rect(500.0, 0.1, 0.025), rect(0.0, 0.0, 0.025)];
        let trib = vec![rect(800.0, 0.15, 0.030), rect(600.0, 0.12, 0.030), rect(400.0, 0.10, 0.030)];

        let inputs = SteadyInputs {
            cross_sections: main,
            flow_rate: 10.0,
            num_slices: Some(50),
            regime: 0,
            downstream_wsel: Some(1.5),
            downstream_bc_type: Some(0),
            tributary_cross_sections: Some(trib),
            tributary_flow_rate: Some(5.0),
            junction_main_station: Some(500.0),
            ..Default::default()
        };

        let result = solve_steady_junction(&inputs);

        // Junction WSEL shared between main and trib mouth solve
        let j_idx = find_section_index(&inputs.cross_sections, 500.0).unwrap();
        let trib_mouth_idx = 2; // station 400 is downstream end of trib
        assert!(
            (result.wsel[j_idx] - result.tributary_wsel.as_ref().unwrap()[trib_mouth_idx]).abs()
                < 1e-3,
            "Junction WSEL should match tributary mouth WSEL"
        );

        // Downstream of junction should reflect combined discharge (higher velocity than upstream at same depth trend)
        let ds_idx = find_section_index(&inputs.cross_sections, 0.0).unwrap();
        let us_idx = j_idx;
        assert!(
            result.velocity[ds_idx] > result.velocity[us_idx],
            "Downstream main velocity should exceed upstream main velocity with added trib flow"
        );

        for &fr in result.tributary_froude.as_ref().unwrap() {
            assert!(fr < 1.0, "Tributary should remain subcritical, got Fr={fr}");
        }
    }
}
