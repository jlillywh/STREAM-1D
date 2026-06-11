//! Deck vent and slotted-opening segments for supplemental pressure-flow paths.

use crate::utils::{UnitSystem, FT_TO_M, G_METRIC};

fn nested_bridge_row(vec: &Option<Vec<Vec<f64>>>, b_idx: usize) -> Option<Vec<f64>> {
    vec.as_ref()?.get(b_idx).cloned()
}

/// Vent segment hydraulic model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckVentType {
    /// Vertical orifice: $Q = C_d A_{sub}\sqrt{2g\Delta H}$.
    Orifice = 0,
    /// Slot weir over invert until soffit submerged, then orifice through slot height.
    Slotted = 1,
}

impl DeckVentType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => DeckVentType::Slotted,
            _ => DeckVentType::Orifice,
        }
    }
}

/// Optional deck vent segments (user units before metric conversion in `build_bridge_geometry`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DeckVentUserInput {
    pub left_stations: Option<Vec<f64>>,
    pub right_stations: Option<Vec<f64>>,
    pub centers: Option<Vec<f64>>,
    pub widths: Option<Vec<f64>>,
    pub invert_elevations: Option<Vec<f64>>,
    pub soffit_elevations: Option<Vec<f64>>,
    pub discharge_coefficients: Option<Vec<f64>>,
    pub types: Option<Vec<i32>>,
}

/// Resolved vent segment in metric units.
#[derive(Debug, Clone)]
pub struct ResolvedDeckVent {
    pub left_station_m: f64,
    pub right_station_m: f64,
    pub invert_m: f64,
    pub soffit_m: f64,
    pub discharge_coeff: f64,
    pub vent_type: DeckVentType,
    /// Flow-normal width ($\Delta s / \cos\theta$).
    pub flow_width_m: f64,
    pub slot_height_m: f64,
}

impl ResolvedDeckVent {
    /// Submerged orifice area at WSEL.
    pub fn submerged_area_m2(&self, wsel_m: f64) -> f64 {
        if wsel_m <= self.invert_m + 1e-9 {
            return 0.0;
        }
        let h = (wsel_m.min(self.soffit_m) - self.invert_m).max(0.0);
        self.flow_width_m * h
    }

    /// Parallel-path discharge for one segment (m³/s).
    pub fn discharge_m3s(&self, wsel_m: f64, e_upstream_m: f64, tw_m: f64) -> f64 {
        if wsel_m <= self.invert_m + 1e-9 {
            return 0.0;
        }
        let head = (e_upstream_m - tw_m).max(0.0);
        if head <= 1e-9 {
            return 0.0;
        }
        match self.vent_type {
            DeckVentType::Orifice => {
                let a = self.submerged_area_m2(wsel_m);
                if a <= 1e-9 {
                    return 0.0;
                }
                self.discharge_coeff * a * (2.0 * G_METRIC * head).sqrt()
            }
            DeckVentType::Slotted => {
                if e_upstream_m <= self.invert_m + 1e-9 {
                    return 0.0;
                }
                if wsel_m < self.soffit_m - 1e-6 {
                    let h_weir = (e_upstream_m - self.invert_m).max(0.0);
                    if h_weir <= 1e-9 {
                        return 0.0;
                    }
                    return self.discharge_coeff * self.flow_width_m * h_weir.powf(1.5);
                }
                let a = self.flow_width_m * self.slot_height_m;
                if a <= 1e-9 {
                    return 0.0;
                }
                self.discharge_coeff * a * (2.0 * G_METRIC * head).sqrt()
            }
        }
    }
}

fn segment_count(user: &DeckVentUserInput) -> usize {
    let mut n = 0usize;
    for opt in [
        user.left_stations.as_ref(),
        user.right_stations.as_ref(),
        user.centers.as_ref(),
        user.widths.as_ref(),
        user.invert_elevations.as_ref(),
        user.soffit_elevations.as_ref(),
        user.discharge_coefficients.as_ref(),
    ] {
        if let Some(v) = opt {
            n = n.max(v.len());
        }
    }
    if let Some(v) = user.types.as_ref() {
        n = n.max(v.len());
    }
    n
}

fn resolve_lateral_extent(
    left: Option<f64>,
    right: Option<f64>,
    center: Option<f64>,
    width: Option<f64>,
) -> Option<(f64, f64)> {
    if let (Some(l), Some(r)) = (left, right) {
        if r > l + 1e-9 {
            return Some((l, r));
        }
        return None;
    }
    if let (Some(c), Some(w)) = (center, width) {
        if w > 1e-9 {
            return Some((c - 0.5 * w, c + 0.5 * w));
        }
    }
    None
}

/// Build resolved vent segments from user input (opening frame, user length/elevation units).
pub fn resolve_deck_vents(
    user: &DeckVentUserInput,
    skew_cos: f64,
    units: UnitSystem,
    default_orifice_coeff: f64,
) -> Vec<ResolvedDeckVent> {
    let n = segment_count(user);
    if n == 0 {
        return Vec::new();
    }
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let cos = skew_cos.max(1e-6);
    let default_cd = if default_orifice_coeff > 1e-6 {
        default_orifice_coeff
    } else {
        0.8
    };
    let mut out = Vec::new();
    for i in 0..n {
        let left = user.left_stations.as_ref().and_then(|v| v.get(i).copied());
        let right = user.right_stations.as_ref().and_then(|v| v.get(i).copied());
        let center = user.centers.as_ref().and_then(|v| v.get(i).copied());
        let width = user.widths.as_ref().and_then(|v| v.get(i).copied());
        let Some((l, r)) = resolve_lateral_extent(left, right, center, width) else {
            continue;
        };
        let Some(invert) = user
            .invert_elevations
            .as_ref()
            .and_then(|v| v.get(i))
            .copied()
        else {
            continue;
        };
        let Some(soffit) = user
            .soffit_elevations
            .as_ref()
            .and_then(|v| v.get(i))
            .copied()
        else {
            continue;
        };
        if soffit <= invert + 1e-9 {
            continue;
        }
        let cd = user
            .discharge_coefficients
            .as_ref()
            .and_then(|v| v.get(i))
            .copied()
            .filter(|&c| c > 1e-6)
            .unwrap_or(default_cd);
        let vent_type = user
            .types
            .as_ref()
            .and_then(|v| v.get(i))
            .copied()
            .map(DeckVentType::from_i32)
            .unwrap_or(DeckVentType::Orifice);
        let l_m = to_m(l);
        let r_m = to_m(r);
        let flow_width = ((r_m - l_m) / cos).max(0.0);
        let invert_m = to_m(invert);
        let soffit_m = to_m(soffit);
        out.push(ResolvedDeckVent {
            left_station_m: l_m,
            right_station_m: r_m,
            invert_m,
            soffit_m,
            discharge_coeff: cd,
            vent_type,
            flow_width_m: flow_width,
            slot_height_m: soffit_m - invert_m,
        });
    }
    out
}

/// Sum parallel vent/slot discharge (m³/s).
pub fn total_deck_vent_discharge_m3s(
    vents: &[ResolvedDeckVent],
    wsel_m: f64,
    e_upstream_m: f64,
    tw_m: f64,
) -> f64 {
    vents
        .iter()
        .map(|v| v.discharge_m3s(wsel_m, e_upstream_m, tw_m))
        .sum()
}

fn deck_vents_configured(input: &DeckVentUserInput) -> bool {
    input.left_stations.is_some()
        || input.right_stations.is_some()
        || input.centers.is_some()
        || input.widths.is_some()
        || input.invert_elevations.is_some()
        || input.soffit_elevations.is_some()
}

/// Per-bridge deck vents from steady/unsteady flat arrays.
pub fn deck_vents_user_for_bridge_index(
    left_stations: &Option<Vec<Vec<f64>>>,
    right_stations: &Option<Vec<Vec<f64>>>,
    centers: &Option<Vec<Vec<f64>>>,
    widths: &Option<Vec<Vec<f64>>>,
    invert_elevations: &Option<Vec<Vec<f64>>>,
    soffit_elevations: &Option<Vec<Vec<f64>>>,
    discharge_coefficients: &Option<Vec<Vec<f64>>>,
    types: &Option<Vec<Vec<i32>>>,
    b_idx: usize,
) -> Option<DeckVentUserInput> {
    let input = DeckVentUserInput {
        left_stations: nested_bridge_row(left_stations, b_idx),
        right_stations: nested_bridge_row(right_stations, b_idx),
        centers: nested_bridge_row(centers, b_idx),
        widths: nested_bridge_row(widths, b_idx),
        invert_elevations: nested_bridge_row(invert_elevations, b_idx),
        soffit_elevations: nested_bridge_row(soffit_elevations, b_idx),
        discharge_coefficients: nested_bridge_row(discharge_coefficients, b_idx),
        types: types.as_ref().and_then(|v| v.get(b_idx)).cloned(),
    };
    if deck_vents_configured(&input) {
        Some(input)
    } else {
        None
    }
}

/// Deck vents for standalone bridge / rating curve.
pub fn deck_vents_from_rating_params(
    left_stations: &Option<Vec<f64>>,
    right_stations: &Option<Vec<f64>>,
    centers: &Option<Vec<f64>>,
    widths: &Option<Vec<f64>>,
    invert_elevations: &Option<Vec<f64>>,
    soffit_elevations: &Option<Vec<f64>>,
    discharge_coefficients: &Option<Vec<f64>>,
    types: &Option<Vec<i32>>,
) -> Option<DeckVentUserInput> {
    let input = DeckVentUserInput {
        left_stations: left_stations.clone(),
        right_stations: right_stations.clone(),
        centers: centers.clone(),
        widths: widths.clone(),
        invert_elevations: invert_elevations.clone(),
        soffit_elevations: soffit_elevations.clone(),
        discharge_coefficients: discharge_coefficients.clone(),
        types: types.clone(),
    };
    if deck_vents_configured(&input) {
        Some(input)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orifice_area_and_discharge() {
        let vent = ResolvedDeckVent {
            left_station_m: 0.0,
            right_station_m: 2.0,
            invert_m: 10.0,
            soffit_m: 12.0,
            discharge_coeff: 0.8,
            vent_type: DeckVentType::Orifice,
            flow_width_m: 2.0,
            slot_height_m: 2.0,
        };
        assert!((vent.submerged_area_m2(11.0) - 2.0).abs() < 1e-9);
        assert!((vent.submerged_area_m2(9.0)).abs() < 1e-9);
        assert!((vent.submerged_area_m2(13.0) - 4.0).abs() < 1e-9);
        let q = vent.discharge_m3s(11.0, 12.0, 8.0);
        let a = 2.0;
        let head = 4.0;
        let expected = 0.8 * a * (2.0 * G_METRIC * head).sqrt();
        assert!((q - expected).abs() < 1e-6);
    }

    #[test]
    fn slotted_weir_then_full_orifice() {
        let vent = ResolvedDeckVent {
            left_station_m: 0.0,
            right_station_m: 5.0,
            invert_m: 100.0,
            soffit_m: 102.0,
            discharge_coeff: 0.7,
            vent_type: DeckVentType::Slotted,
            flow_width_m: 5.0,
            slot_height_m: 2.0,
        };
        let q_weir = vent.discharge_m3s(101.0, 101.5, 98.0);
        assert!(q_weir > 0.0);
        let q_full = vent.discharge_m3s(103.0, 104.0, 98.0);
        let head = 6.0;
        let expected = 0.7 * 10.0 * (2.0 * G_METRIC * head).sqrt();
        assert!((q_full - expected).abs() < 1e-5);
    }

    #[test]
    fn center_width_resolves_to_left_right() {
        let user = DeckVentUserInput {
            centers: Some(vec![25.0]),
            widths: Some(vec![10.0]),
            invert_elevations: Some(vec![100.0]),
            soffit_elevations: Some(vec![102.0]),
            ..Default::default()
        };
        let vents = resolve_deck_vents(&user, 1.0, UnitSystem::Metric, 0.8);
        assert_eq!(vents.len(), 1);
        assert!((vents[0].left_station_m - 20.0).abs() < 1e-9);
        assert!((vents[0].right_station_m - 30.0).abs() < 1e-9);
    }

    #[test]
    fn partial_submergence_area_scales_to_soffit() {
        let vent = ResolvedDeckVent {
            left_station_m: 0.0,
            right_station_m: 2.0,
            invert_m: 5.2,
            soffit_m: 5.9,
            discharge_coeff: 0.8,
            vent_type: DeckVentType::Orifice,
            flow_width_m: 2.0,
            slot_height_m: 0.7,
        };
        assert!((vent.submerged_area_m2(5.0)).abs() < 1e-9);
        assert!((vent.submerged_area_m2(5.55) - 0.7).abs() < 1e-9);
        assert!((vent.submerged_area_m2(6.5) - 1.4).abs() < 1e-9);
        let q_partial = vent.discharge_m3s(5.55, 5.8, 5.3);
        let head = 0.5;
        let expected = 0.8 * 0.7 * (2.0 * G_METRIC * head).sqrt();
        assert!((q_partial - expected).abs() < 1e-5);
    }

    #[test]
    fn left_right_wins_over_center() {
        let user = DeckVentUserInput {
            left_stations: Some(vec![1.0]),
            right_stations: Some(vec![3.0]),
            centers: Some(vec![25.0]),
            widths: Some(vec![10.0]),
            invert_elevations: Some(vec![100.0]),
            soffit_elevations: Some(vec![101.0]),
            ..Default::default()
        };
        let vents = resolve_deck_vents(&user, 1.0, UnitSystem::Metric, 0.8);
        assert!((vents[0].flow_width_m - 2.0).abs() < 1e-9);
    }
}
