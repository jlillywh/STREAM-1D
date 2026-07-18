//! HEC-RAS-style guide banks on approach / departure cross sections (API v24).

use crate::utils::{UnitSystem, FT_TO_M};

/// One guide-bank polyline across a cross section (reach lateral `stations`, paired `elevations`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuideBankPolyline {
    pub stations: Vec<f64>,
    pub elevations: Vec<f64>,
}

impl GuideBankPolyline {
    pub fn is_valid(&self) -> bool {
        let n = self.stations.len();
        n >= 2
            && n == self.elevations.len()
            && self.stations.windows(2).all(|w| w[1] > w[0])
    }
}

/// Simplified left or right guide-bank toe (one reach lateral station + ground/crest elevation).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuideBankToe {
    pub station: f64,
    pub elevation: f64,
}

/// Guide banks on an approach or departure cut — defines the guided channel width for bridge hydraulics.
///
/// Coordinates are reach lateral `x` on that cut (same frame as `CrossSection.x` and ineffective blocks).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuideBanks {
    /// Piecewise left guide-bank line(s). Flow left of the leftmost active line is outside the guided channel.
    #[serde(default)]
    pub left_polylines: Vec<GuideBankPolyline>,
    /// Piecewise right guide-bank line(s). Flow right of the rightmost active line is outside the guided channel.
    #[serde(default)]
    pub right_polylines: Vec<GuideBankPolyline>,
    /// Optional simplified left toe when a single station/elevation pair is sufficient.
    #[serde(default)]
    pub left_toe: Option<GuideBankToe>,
    /// Optional simplified right toe.
    #[serde(default)]
    pub right_toe: Option<GuideBankToe>,
}

impl GuideBanks {
    pub fn is_configured(&self) -> bool {
        self.left_polylines.iter().any(|p| p.is_valid())
            || self.right_polylines.iter().any(|p| p.is_valid())
            || self.left_toe.is_some()
            || self.right_toe.is_some()
    }

    /// Non-fatal validation messages (empty when OK).
    pub fn validation_warnings(&self, label: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (side, polylines) in [
            ("left", &self.left_polylines),
            ("right", &self.right_polylines),
        ] {
            for (idx, poly) in polylines.iter().enumerate() {
                if poly.stations.is_empty() && poly.elevations.is_empty() {
                    continue;
                }
                if !poly.is_valid() {
                    out.push(format!(
                        "{label}: {side} guide-bank polyline {idx} needs ≥2 monotonic station/elevation pairs"
                    ));
                }
            }
        }
        out
    }

    /// Convert guide-bank coordinates to metric.
    pub fn to_metric(&self, from_units: UnitSystem) -> Self {
        if from_units == UnitSystem::Metric {
            return self.clone();
        }
        let scale_poly = |p: &GuideBankPolyline| GuideBankPolyline {
            stations: p.stations.iter().map(|s| s * FT_TO_M).collect(),
            elevations: p.elevations.iter().map(|e| e * FT_TO_M).collect(),
        };
        let scale_toe = |t: GuideBankToe| GuideBankToe {
            station: t.station * FT_TO_M,
            elevation: t.elevation * FT_TO_M,
        };
        Self {
            left_polylines: self.left_polylines.iter().map(scale_poly).collect(),
            right_polylines: self.right_polylines.iter().map(scale_poly).collect(),
            left_toe: self.left_toe.map(scale_toe),
            right_toe: self.right_toe.map(scale_toe),
        }
    }
}

/// Lateral `x` where a guide-bank polyline crosses water-surface elevation `wsel`.
fn polyline_crossing_station_at_wsel(poly: &GuideBankPolyline, wsel: f64) -> Option<f64> {
    if !poly.is_valid() {
        return None;
    }
    for i in 0..poly.stations.len() - 1 {
        let e0 = poly.elevations[i];
        let e1 = poly.elevations[i + 1];
        let s0 = poly.stations[i];
        let s1 = poly.stations[i + 1];
        if (e0 - wsel).abs() < 1e-9 {
            return Some(s0);
        }
        if (e1 - wsel).abs() < 1e-9 {
            return Some(s1);
        }
        if (e0 - wsel) * (e1 - wsel) < 0.0 {
            let t = (wsel - e0) / (e1 - e0);
            return Some(s0 + t * (s1 - s0));
        }
        if e0 <= wsel && e1 <= wsel {
            // Submerged segment — use inner station toward channel center.
            continue;
        }
    }
    None
}

/// Active-flow lateral limits `(left, right)` from guide banks at `wsel` (metric reach `x`).
///
/// Flow is guided between `left` and `right`. Polylines take precedence over toe pairs.
pub fn lateral_limits_at_wsel(gb: &GuideBanks, wsel: f64) -> Option<(f64, f64)> {
    if !gb.is_configured() {
        return None;
    }

    let mut left = f64::NEG_INFINITY;
    let mut right = f64::INFINITY;

    for poly in gb
        .left_polylines
        .iter()
        .filter(|p| p.is_valid())
    {
        if let Some(s) = polyline_crossing_station_at_wsel(poly, wsel) {
            left = left.max(s);
        }
    }
    if let Some(toe) = gb.left_toe {
        if wsel + 1e-9 >= toe.elevation {
            left = left.max(toe.station);
        }
    }

    for poly in gb
        .right_polylines
        .iter()
        .filter(|p| p.is_valid())
    {
        if let Some(s) = polyline_crossing_station_at_wsel(poly, wsel) {
            right = right.min(s);
        }
    }
    if let Some(toe) = gb.right_toe {
        if wsel + 1e-9 >= toe.elevation {
            right = right.min(toe.station);
        }
    }

    if !left.is_finite() && !right.is_finite() {
        return None;
    }
    let left = if left.is_finite() { left } else { f64::NEG_INFINITY };
    let right = if right.is_finite() { right } else { f64::INFINITY };
    if left >= right {
        return None;
    }
    Some((left, right))
}

/// True when lateral coordinate `x` lies outside the guided channel.
pub fn segment_outside_guided_channel(x: f64, limits: (f64, f64)) -> bool {
    x < limits.0 || x > limits.1
}

/// Fraction of a wetted segment `[xa, xb]` inside guided lateral limits.
pub fn segment_guide_fraction(xa: f64, xb: f64, limits: (f64, f64)) -> f64 {
    let lo = xa.min(xb);
    let hi = xa.max(xb);
    let clip_lo = lo.max(limits.0);
    let clip_hi = hi.min(limits.1);
    if clip_hi <= clip_lo + 1e-9 {
        0.0
    } else {
        (clip_hi - clip_lo) / (hi - lo).max(1e-9)
    }
}

/// Resolve guide banks for one approach or departure cut.
///
/// Precedence: `CrossSection.guide_banks` on the resolved cut, else per-bridge override.
pub fn resolve_guide_banks(
    cut_xs: Option<&crate::geometry::CrossSection>,
    bridge_level: Option<&GuideBanks>,
) -> Option<GuideBanks> {
    if let Some(xs) = cut_xs {
        if let Some(ref gb) = xs.guide_banks {
            if gb.is_configured() {
                return Some(gb.clone());
            }
        }
    }
    bridge_level.filter(|g| g.is_configured()).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyline_requires_monotonic_stations() {
        let valid = GuideBankPolyline {
            stations: vec![10.0, 20.0, 30.0],
            elevations: vec![5.0, 5.5, 6.0],
        };
        assert!(valid.is_valid());
        let invalid = GuideBankPolyline {
            stations: vec![10.0, 10.0],
            elevations: vec![5.0, 5.5],
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn toe_only_is_configured() {
        let gb = GuideBanks {
            left_toe: Some(GuideBankToe {
                station: 100.0,
                elevation: 12.0,
            }),
            ..Default::default()
        };
        assert!(gb.is_configured());
    }

    #[test]
    fn lateral_limits_from_submerged_toes() {
        let gb = GuideBanks {
            left_toe: Some(GuideBankToe {
                station: 8.0,
                elevation: 0.0,
            }),
            right_toe: Some(GuideBankToe {
                station: 12.0,
                elevation: 0.0,
            }),
            ..Default::default()
        };
        let limits = lateral_limits_at_wsel(&gb, 3.0).unwrap();
        assert!((limits.0 - 8.0).abs() < 1e-9);
        assert!((limits.1 - 12.0).abs() < 1e-9);
        assert!(lateral_limits_at_wsel(&gb, -1.0).is_none());
    }

    #[test]
    fn validation_warnings_for_invalid_polyline() {
        let gb = GuideBanks {
            left_polylines: vec![GuideBankPolyline {
                stations: vec![1.0, 1.0],
                elevations: vec![2.0, 3.0],
            }],
            ..Default::default()
        };
        let msgs = gb.validation_warnings("Test");
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("left guide-bank polyline 0"));
    }

    #[test]
    fn to_metric_scales_us_customary() {
        let gb = GuideBanks {
            left_toe: Some(GuideBankToe {
                station: 10.0,
                elevation: 5.0,
            }),
            left_polylines: vec![GuideBankPolyline {
                stations: vec![0.0, 10.0],
                elevations: vec![4.0, 6.0],
            }],
            ..Default::default()
        };
        let m = gb.to_metric(UnitSystem::USCustomary);
        assert!((m.left_toe.unwrap().station - 10.0 * FT_TO_M).abs() < 1e-9);
        assert!((m.left_polylines[0].stations[1] - 10.0 * FT_TO_M).abs() < 1e-9);
    }

    #[test]
    fn polyline_limits_interpolate_crossing() {
        let gb = GuideBanks {
            left_polylines: vec![GuideBankPolyline {
                stations: vec![0.0, 10.0],
                elevations: vec![2.0, 4.0],
            }],
            right_polylines: vec![GuideBankPolyline {
                stations: vec![20.0, 30.0],
                elevations: vec![4.0, 2.0],
            }],
            ..Default::default()
        };
        let limits = lateral_limits_at_wsel(&gb, 3.0).unwrap();
        assert!((limits.0 - 5.0).abs() < 1e-6);
        assert!((limits.1 - 25.0).abs() < 1e-6);
    }

    #[test]
    fn polyline_exact_vertex_elevation() {
        let gb = GuideBanks {
            left_polylines: vec![GuideBankPolyline {
                stations: vec![0.0, 10.0, 20.0],
                elevations: vec![2.0, 3.0, 4.0],
            }],
            right_toe: Some(GuideBankToe {
                station: 40.0,
                elevation: 0.0,
            }),
            ..Default::default()
        };
        let limits = lateral_limits_at_wsel(&gb, 3.0).unwrap();
        assert!((limits.0 - 10.0).abs() < 1e-9);
        assert!((limits.1 - 40.0).abs() < 1e-9);
    }

    #[test]
    fn invalid_limits_when_left_meets_right() {
        let gb = GuideBanks {
            left_toe: Some(GuideBankToe {
                station: 15.0,
                elevation: 0.0,
            }),
            right_toe: Some(GuideBankToe {
                station: 10.0,
                elevation: 0.0,
            }),
            ..Default::default()
        };
        assert!(lateral_limits_at_wsel(&gb, 2.0).is_none());
    }

    #[test]
    fn segment_helpers_clip_fraction() {
        let limits = (5.0, 15.0);
        assert!(segment_outside_guided_channel(3.0, limits));
        assert!(!segment_outside_guided_channel(10.0, limits));
        assert!((segment_guide_fraction(0.0, 20.0, limits) - 0.5).abs() < 1e-9);
        assert_eq!(segment_guide_fraction(0.0, 4.0, limits), 0.0);
    }

    #[test]
    fn resolve_falls_back_to_bridge_level() {
        let cut = crate::geometry::CrossSection {
            station: 1.0,
            x: vec![0.0, 10.0],
            y: vec![0.0, 0.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            coeff_contraction: None,
            coeff_expansion: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let bridge = GuideBanks {
            right_toe: Some(GuideBankToe {
                station: 7.0,
                elevation: 0.0,
            }),
            ..Default::default()
        };
        let resolved = resolve_guide_banks(Some(&cut), Some(&bridge)).unwrap();
        assert_eq!(resolved.right_toe.unwrap().station, 7.0);
    }

    #[test]
    fn resolve_prefers_cut_over_bridge_level() {
        use crate::geometry::CrossSection;
        let cut = CrossSection {
            station: 600.0,
            x: vec![0.0, 50.0],
            y: vec![5.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            coeff_contraction: None,
            coeff_expansion: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: Some(GuideBanks {
                left_toe: Some(GuideBankToe {
                    station: 5.0,
                    elevation: 4.0,
                }),
                ..Default::default()
            }),
        };
        let bridge = GuideBanks {
            left_toe: Some(GuideBankToe {
                station: 99.0,
                elevation: 4.0,
            }),
            ..Default::default()
        };
        let resolved = resolve_guide_banks(Some(&cut), Some(&bridge)).unwrap();
        assert_eq!(resolved.left_toe.unwrap().station, 5.0);
    }

    #[test]
    fn right_polyline_only_is_configured() {
        let gb = GuideBanks {
            right_polylines: vec![GuideBankPolyline {
                stations: vec![5.0, 15.0],
                elevations: vec![3.0, 4.0],
            }],
            ..Default::default()
        };
        assert!(gb.is_configured());
        let limits = lateral_limits_at_wsel(&gb, 3.5).unwrap();
        assert!((limits.1 - 10.0).abs() < 1e-6);
    }

    #[test]
    fn fully_submerged_polyline_defers_to_toe_limits() {
        let gb = GuideBanks {
            left_polylines: vec![GuideBankPolyline {
                stations: vec![0.0, 10.0],
                elevations: vec![2.0, 3.0],
            }],
            left_toe: Some(GuideBankToe {
                station: 8.0,
                elevation: 0.0,
            }),
            right_toe: Some(GuideBankToe {
                station: 30.0,
                elevation: 0.0,
            }),
            ..Default::default()
        };
        let limits = lateral_limits_at_wsel(&gb, 5.0).unwrap();
        assert!((limits.0 - 8.0).abs() < 1e-9);
        assert_eq!(limits.1, 30.0);
    }

    #[test]
    fn left_only_toe_yields_half_bounded_limits() {
        let gb = GuideBanks {
            left_toe: Some(GuideBankToe {
                station: 12.0,
                elevation: 0.0,
            }),
            ..Default::default()
        };
        let limits = lateral_limits_at_wsel(&gb, 2.0).unwrap();
        assert!((limits.0 - 12.0).abs() < 1e-9);
        assert!(limits.1.is_infinite());
    }

    #[test]
    fn to_metric_is_identity_for_metric_units() {
        let gb = GuideBanks {
            left_toe: Some(GuideBankToe {
                station: 3.0,
                elevation: 1.0,
            }),
            ..Default::default()
        };
        assert_eq!(gb.to_metric(UnitSystem::Metric), gb);
    }

    #[test]
    fn empty_polyline_arrays_are_ignored_in_validation() {
        let gb = GuideBanks {
            left_polylines: vec![GuideBankPolyline {
                stations: vec![],
                elevations: vec![],
            }],
            ..Default::default()
        };
        assert!(gb.validation_warnings("Cut").is_empty());
        assert!(!gb.is_configured());
    }

    #[test]
    fn polyline_crossing_rejects_invalid_polyline() {
        let poly = GuideBankPolyline {
            stations: vec![1.0, 1.0],
            elevations: vec![2.0, 3.0],
        };
        assert!(polyline_crossing_station_at_wsel(&poly, 2.5).is_none());
    }

    #[test]
    fn polyline_crossing_at_start_vertex_elevation() {
        let poly = GuideBankPolyline {
            stations: vec![0.0, 10.0],
            elevations: vec![3.0, 5.0],
        };
        assert_eq!(polyline_crossing_station_at_wsel(&poly, 3.0), Some(0.0));
    }

    #[test]
    fn polyline_crossing_skips_fully_submerged_segment() {
        let poly = GuideBankPolyline {
            stations: vec![0.0, 10.0, 20.0],
            elevations: vec![1.0, 2.0, 3.0],
        };
        assert!(polyline_crossing_station_at_wsel(&poly, 5.0).is_none());
    }

    #[test]
    fn polyline_crossing_skips_submerged_then_finds_emergent_segment() {
        let poly = GuideBankPolyline {
            stations: vec![0.0, 10.0, 20.0, 30.0],
            elevations: vec![1.0, 2.0, 2.0, 6.0],
        };
        // First segment fully submerged at wsel=5; third segment crosses between 2 and 6.
        let station = polyline_crossing_station_at_wsel(&poly, 5.0).expect("crossing");
        assert!(station > 20.0);
    }

    #[test]
    fn lateral_limits_none_when_unconfigured() {
        assert!(lateral_limits_at_wsel(&GuideBanks::default(), 2.0).is_none());
    }

    #[test]
    fn resolve_skips_unconfigured_cut_guide_banks() {
        use crate::geometry::CrossSection;
        let cut = CrossSection {
            station: 1.0,
            x: vec![0.0, 10.0],
            y: vec![0.0, 0.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            coeff_contraction: None,
            coeff_expansion: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: Some(GuideBanks::default()),
        };
        let bridge = GuideBanks {
            right_toe: Some(GuideBankToe {
                station: 7.0,
                elevation: 0.0,
            }),
            ..Default::default()
        };
        let resolved = resolve_guide_banks(Some(&cut), Some(&bridge)).unwrap();
        assert_eq!(resolved.right_toe.unwrap().station, 7.0);
    }
}
