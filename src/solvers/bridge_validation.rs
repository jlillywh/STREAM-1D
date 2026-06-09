//! Steady-input validation for bridge opening ↔ reach alignment (checklist §1.3).

use crate::geometry::CrossSection;
use crate::solvers::bridge_abutment::{abutment_user_input_from_steady, BridgeAbutmentUserInput};
use crate::solvers::bridge_interior::{
    interior_from_steady, opening_station_to_reach_x, resolve_opening_reach_station_origin,
};
use crate::solvers::steady::SteadyInputs;
use crate::utils::{UnitSystem, FT_TO_M, STRUCTURE_STATION_TOL};

/// Lateral tolerance when comparing opening extent to parent XS width (user units).
const LATERAL_TOL: f64 = 1e-3;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SteadyValidationResult {
    pub warnings: Vec<String>,
}

/// Semantic validation of steady inputs (non-fatal warnings).
pub fn validate_steady_inputs(inputs: &SteadyInputs) -> SteadyValidationResult {
    let mut warnings = validate_bridge_opening_lateral_extent(inputs);
    warnings.extend(validate_bridge_guide_banks(inputs));
    SteadyValidationResult { warnings }
}

fn validate_bridge_guide_banks(inputs: &SteadyInputs) -> Vec<String> {
    let n_bridges = inputs
        .bridge_stations
        .as_ref()
        .map(|v| v.len())
        .unwrap_or(0);
    if n_bridges == 0 {
        return vec![];
    }
    let mut warnings = Vec::new();
    for b_idx in 0..n_bridges {
        let label = format!("Bridge {b_idx}");
        if let Some(gb) = inputs
            .bridge_approach_guide_banks
            .as_ref()
            .and_then(|v| v.get(b_idx))
        {
            warnings.extend(gb.validation_warnings(&format!("{label} approach")));
        }
        if let Some(gb) = inputs
            .bridge_departure_guide_banks
            .as_ref()
            .and_then(|v| v.get(b_idx))
        {
            warnings.extend(gb.validation_warnings(&format!("{label} departure")));
        }
        if let Some(xs) = inputs
            .bridge_approach_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
        {
            if let Some(ref gb) = xs.guide_banks {
                warnings.extend(gb.validation_warnings(&format!("{label} approach cross section")));
            }
        }
        if let Some(xs) = inputs
            .bridge_departure_cross_sections
            .as_ref()
            .and_then(|v| v.get(b_idx))
        {
            if let Some(ref gb) = xs.guide_banks {
                warnings.extend(gb.validation_warnings(&format!("{label} departure cross section")));
            }
        }
    }
    warnings
}

/// Minimum and maximum lateral `x` on a cross-section polyline (section user units).
pub fn cross_section_lateral_bounds(xs: &CrossSection) -> (f64, f64) {
    let min_x = xs.x.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = xs.x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (min_x, max_x)
}

fn river_station_to_metric(station: f64, units: UnitSystem) -> f64 {
    if units == UnitSystem::USCustomary {
        station * FT_TO_M
    } else {
        station
    }
}

fn cross_section_at_river_station(sections: &[CrossSection], station_user: f64) -> Option<CrossSection> {
    sections
        .iter()
        .find(|xs| (xs.station - station_user).abs() <= STRUCTURE_STATION_TOL)
        .cloned()
}

/// Reach cross section immediately upstream of `bridge_st_user` on the main reach.
fn upstream_reach_xs_at_bridge(inputs: &SteadyInputs, bridge_st_user: f64) -> Option<CrossSection> {
    if inputs.cross_sections.is_empty() {
        return None;
    }
    let raw_units = inputs
        .cross_sections
        .first()
        .map(|xs| xs.unit_system)
        .unwrap_or(UnitSystem::Metric);
    let bridge_m = river_station_to_metric(bridge_st_user, raw_units);

    let mut sorted: Vec<&CrossSection> = inputs.cross_sections.iter().collect();
    sorted.sort_by(|a, b| b.station.partial_cmp(&a.station).unwrap());

    for w in sorted.windows(2) {
        let us_m = river_station_to_metric(w[0].station, w[0].unit_system);
        let ds_m = river_station_to_metric(w[1].station, w[1].unit_system);
        if bridge_m <= us_m + STRUCTURE_STATION_TOL && bridge_m >= ds_m - STRUCTURE_STATION_TOL {
            return Some(w[0].clone());
        }
    }
    sorted.first().map(|xs| (*xs).clone())
}

/// Horizontal opening extent in opening coordinates (user units), including abutments and pier faces.
fn opening_span_opening_frame_user(
    deck_stations: Option<&[f64]>,
    abutment: &BridgeAbutmentUserInput,
    pier_stations: Option<&[f64]>,
    pier_width: f64,
    skew_deg: f64,
) -> Option<(f64, f64)> {
    let mut s_min: Option<f64> = None;
    let mut s_max: Option<f64> = None;

    if let Some(st) = deck_stations.filter(|s| s.len() >= 2) {
        for &s in st {
            s_min = Some(s_min.map(|m| m.min(s)).unwrap_or(s));
            s_max = Some(s_max.map(|m| m.max(s)).unwrap_or(s));
        }
    }

    let skew_cos = skew_deg.clamp(0.0, 59.0).to_radians().cos().max(0.52);
    let deck_lo = s_min.unwrap_or(0.0);
    let deck_hi = s_max.unwrap_or(deck_lo);

    let mut absorb = |s: f64| {
        s_min = Some(s_min.map(|m| m.min(s)).unwrap_or(s));
        s_max = Some(s_max.map(|m| m.max(s)).unwrap_or(s));
    };

    if let Some(w) = abutment.left_width.filter(|&w| w > 1e-6) {
        let outer = abutment.left_station.unwrap_or(deck_lo);
        let width_along = w / skew_cos;
        absorb(outer);
        absorb(outer + width_along);
    }
    if let Some(w) = abutment.right_width.filter(|&w| w > 1e-6) {
        let outer = abutment.right_station.unwrap_or(deck_hi);
        let width_along = w / skew_cos;
        absorb(outer - width_along);
        absorb(outer);
    }
    if abutment.legacy_total_width > 1e-6
        && abutment.left_width.is_none()
        && abutment.right_width.is_none()
        && deck_stations.is_some()
    {
        absorb(deck_lo);
        absorb(deck_hi);
    }

    if let Some(piers) = pier_stations.filter(|p| !p.is_empty()) {
        let half = pier_width * 0.5;
        for &p in piers {
            absorb(p - half);
            absorb(p + half);
        }
    }

    match (s_min, s_max) {
        (Some(a), Some(b)) if b > a + LATERAL_TOL => Some((a, b)),
        _ => None,
    }
}

fn parent_xs_label(has_bu: bool, has_anchor: bool) -> &'static str {
    if has_bu {
        "BU upstream face"
    } else if has_anchor {
        "anchor reach cross section"
    } else {
        "upstream reach cross section"
    }
}

fn validate_bridge_opening_lateral_extent(inputs: &SteadyInputs) -> Vec<String> {
    let bridge_stations = match &inputs.bridge_stations {
        Some(s) if !s.is_empty() => s,
        _ => return Vec::new(),
    };

    let mut warnings = Vec::new();

    for (b_idx, &bridge_st) in bridge_stations.iter().enumerate() {
        let interior = interior_from_steady(inputs, b_idx);
        let reach_us = upstream_reach_xs_at_bridge(inputs, bridge_st);
        let anchor_xs = interior
            .opening_anchor_reach_station
            .and_then(|st| cross_section_at_river_station(&inputs.cross_sections, st));

        let origin = resolve_opening_reach_station_origin(
            interior.opening_reach_station_origin,
            interior.opening_anchor_mode,
            interior.bu.as_ref(),
            anchor_xs.as_ref(),
            reach_us.as_ref(),
        );

        let parent_xs = interior
            .bu
            .as_ref()
            .or(anchor_xs.as_ref())
            .or(reach_us.as_ref());
        let Some(parent_xs) = parent_xs else {
            continue;
        };

        let deck_st = inputs
            .bridge_deck_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|v| v.as_slice());
        let abutment = abutment_user_input_from_steady(
            inputs
                .bridge_abutment_block_widths
                .as_ref()
                .and_then(|v| v.get(b_idx))
                .copied(),
            inputs.bridge_abutment_left_widths.as_ref(),
            inputs.bridge_abutment_right_widths.as_ref(),
            inputs.bridge_abutment_left_stations.as_ref(),
            inputs.bridge_abutment_right_stations.as_ref(),
            inputs.bridge_abutment_left_top_elevations.as_ref(),
            inputs.bridge_abutment_right_top_elevations.as_ref(),
            inputs.bridge_abutment_left_top_profile_stations.as_ref(),
            inputs.bridge_abutment_left_top_profile_elevations.as_ref(),
            inputs.bridge_abutment_right_top_profile_stations.as_ref(),
            inputs.bridge_abutment_right_top_profile_elevations.as_ref(),
            b_idx,
        );
        let pier_st = inputs
            .bridge_pier_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|v| v.as_slice());
        let pier_w = inputs
            .bridge_pier_widths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0);
        let skew = inputs
            .bridge_skew_angles
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0);

        let Some((s_min, s_max)) =
            opening_span_opening_frame_user(deck_st, &abutment, pier_st, pier_w, skew)
        else {
            continue;
        };

        let (open_min, open_max) = if let Some(o) = origin {
            (
                opening_station_to_reach_x(s_min, o),
                opening_station_to_reach_x(s_max, o),
            )
        } else {
            (s_min, s_max)
        };

        let (parent_min, parent_max) = cross_section_lateral_bounds(parent_xs);
        if open_min < parent_min - LATERAL_TOL || open_max > parent_max + LATERAL_TOL {
            warnings.push(format!(
                "Bridge {}: opening lateral extent [{:.4}, {:.4}] exceeds parent cross-section x range [{:.4}, {:.4}] ({})",
                b_idx,
                open_min,
                open_max,
                parent_min,
                parent_max,
                parent_xs_label(interior.bu.is_some(), anchor_xs.is_some()),
            ));
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::UnitSystem;

    fn box_xs(station: f64, x0: f64, width: f64) -> CrossSection {
        CrossSection {
            station,
            x: vec![x0, x0, x0 + width, x0 + width],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
        guide_banks: None,
        }
    }

    fn minimal_bridge_inputs(
        parent: CrossSection,
        deck: Vec<f64>,
        origin: Option<f64>,
    ) -> SteadyInputs {
        SteadyInputs {
            cross_sections: vec![parent.clone(), box_xs(0.0, 0.0, 200.0)],
            flow_rate: 10.0,
            bridge_stations: Some(vec![50.0]),
            bridge_low_chords: Some(vec![5.0]),
            bridge_high_chords: Some(vec![7.0]),
            bridge_deck_stations: Some(vec![deck.clone()]),
            bridge_deck_low_elevations: Some(vec![vec![5.0; deck.len()]]),
            bridge_deck_high_elevations: Some(vec![vec![7.0; deck.len()]]),
            bridge_opening_reach_station_origins: origin.map(|o| vec![o]),
            bridge_upstream_cross_sections: Some(vec![parent]),
            ..Default::default()
        }
    }

    #[test]
    fn warns_when_opening_exceeds_parent_xs() {
        let parent = box_xs(50.0, 100.0, 30.0);
        let inputs = minimal_bridge_inputs(parent, vec![0.0, 35.0], Some(100.0));
        let result = validate_steady_inputs(&inputs);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Bridge 0"));
        assert!(result.warnings[0].contains("exceeds parent"));
    }

    #[test]
    fn no_warning_when_opening_inside_parent_xs() {
        let parent = box_xs(50.0, 100.0, 30.0);
        let inputs = minimal_bridge_inputs(parent, vec![0.0, 30.0], Some(100.0));
        let result = validate_steady_inputs(&inputs);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn warns_on_invalid_guide_bank_polyline() {
        use crate::geometry::{GuideBankPolyline, GuideBanks};
        let parent = box_xs(50.0, 100.0, 30.0);
        let inputs = SteadyInputs {
            cross_sections: vec![parent.clone(), box_xs(0.0, 0.0, 200.0)],
            flow_rate: 10.0,
            bridge_stations: Some(vec![50.0]),
            bridge_low_chords: Some(vec![5.0]),
            bridge_high_chords: Some(vec![7.0]),
            bridge_approach_guide_banks: Some(vec![GuideBanks {
                left_polylines: vec![GuideBankPolyline {
                    stations: vec![0.0, 0.0],
                    elevations: vec![1.0, 2.0],
                }],
                ..Default::default()
            }]),
            ..Default::default()
        };
        let result = validate_steady_inputs(&inputs);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("approach"));
    }

    #[test]
    fn cross_section_lateral_bounds_returns_min_max() {
        let xs = box_xs(50.0, 100.0, 30.0);
        assert_eq!(cross_section_lateral_bounds(&xs), (100.0, 130.0));
    }

    #[test]
    fn warns_with_pier_extent_on_anchor_reach_parent() {
        let parent = box_xs(600.0, 80.0, 50.0);
        let inputs = SteadyInputs {
            cross_sections: vec![parent.clone(), box_xs(0.0, 0.0, 200.0)],
            flow_rate: 10.0,
            bridge_stations: Some(vec![50.0]),
            bridge_low_chords: Some(vec![5.0]),
            bridge_high_chords: Some(vec![7.0]),
            bridge_deck_stations: Some(vec![vec![0.0, 55.0]]),
            bridge_deck_low_elevations: Some(vec![vec![5.0, 5.0]]),
            bridge_deck_high_elevations: Some(vec![vec![7.0, 7.0]]),
            bridge_pier_stations: Some(vec![vec![50.0]]),
            bridge_pier_widths: Some(vec![2.0]),
            bridge_opening_anchor_modes: Some(vec![1]),
            bridge_opening_anchor_reach_stations: Some(vec![600.0]),
            ..Default::default()
        };
        let result = validate_steady_inputs(&inputs);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("anchor reach cross section"));
    }

    #[test]
    fn warns_on_invalid_departure_guide_bank_polyline() {
        use crate::geometry::{GuideBankPolyline, GuideBanks};
        let inputs = SteadyInputs {
            bridge_stations: Some(vec![50.0]),
            bridge_departure_guide_banks: Some(vec![GuideBanks {
                right_polylines: vec![GuideBankPolyline {
                    stations: vec![1.0, 1.0],
                    elevations: vec![2.0, 3.0],
                }],
                ..Default::default()
            }]),
            ..Default::default()
        };
        let result = validate_steady_inputs(&inputs);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("departure"));
    }

    #[test]
    fn warns_on_guide_banks_embedded_in_approach_cross_section() {
        use crate::geometry::{GuideBankPolyline, GuideBanks};
        let mut approach = box_xs(60.0, 100.0, 30.0);
        approach.guide_banks = Some(GuideBanks {
            left_polylines: vec![GuideBankPolyline {
                stations: vec![0.0, 0.0],
                elevations: vec![1.0, 2.0],
            }],
            ..Default::default()
        });
        let inputs = SteadyInputs {
            bridge_stations: Some(vec![50.0]),
            bridge_approach_cross_sections: Some(vec![approach]),
            ..Default::default()
        };
        let result = validate_steady_inputs(&inputs);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("approach cross section"));
    }

    #[test]
    fn no_warning_without_deck_or_abutment_span() {
        let parent = box_xs(50.0, 100.0, 30.0);
        let inputs = SteadyInputs {
            cross_sections: vec![parent.clone(), box_xs(0.0, 0.0, 200.0)],
            flow_rate: 10.0,
            bridge_stations: Some(vec![50.0]),
            bridge_low_chords: Some(vec![5.0]),
            bridge_high_chords: Some(vec![7.0]),
            bridge_upstream_cross_sections: Some(vec![parent]),
            ..Default::default()
        };
        assert!(validate_steady_inputs(&inputs).warnings.is_empty());
    }
}
