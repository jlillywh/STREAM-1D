use crate::geometry::GeometryTable;
use crate::solvers::deck_vent_geometry::total_deck_vent_discharge_m3s;
use crate::utils::G_METRIC;

use super::geometry::{
    effective_deck_crest_m, effective_scalar_high_chord_m, effective_weir_length_m,
    interpolate_profile, BridgeGeometry,
};
use super::low_flow::{
    net_opening_area_at_low_chord, solve_high_flow_energy, solve_high_flow_energy_fallback,
    upstream_energy_grade,
};
use super::opening::{obstructed_hydraulics, opening_height_below_deck_m, velocity_head};
use super::types::{BridgeFlowRegimeKind, BridgeHeadwaterSolve, HighFlowMethod};

/// Bradley (1978) trapezoidal-weir submergence curve (HEC-RAS Fig. 5-8): percent submergence → flow factor.
const BRADLEY_SUBMERGENCE_PCT: [f64; 12] = [
    0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 95.0, 98.0,
];
const BRADLEY_FLOW_FACTOR: [f64; 12] = [
    1.0, 1.0, 0.99, 0.97, 0.94, 0.90, 0.84, 0.75, 0.62, 0.40, 0.22, 0.08,
];
pub(crate) fn sluice_gate_discharge_coeff(y3_over_z: f64, user_coeff: f64) -> f64 {
    if user_coeff > 1e-6 {
        return user_coeff;
    }
    let r = y3_over_z.clamp(0.0, 1.0);
    0.27 + 0.23 * r
}

pub(crate) fn bradley_weir_submergence_factor(submergence_ratio: f64) -> f64 {
    if submergence_ratio <= 0.0 {
        return 1.0;
    }
    let pct = (submergence_ratio * 100.0).clamp(0.0, 98.0);
    for i in 1..BRADLEY_SUBMERGENCE_PCT.len() {
        if pct <= BRADLEY_SUBMERGENCE_PCT[i] {
            let t = (pct - BRADLEY_SUBMERGENCE_PCT[i - 1])
                / (BRADLEY_SUBMERGENCE_PCT[i] - BRADLEY_SUBMERGENCE_PCT[i - 1]);
            return BRADLEY_FLOW_FACTOR[i - 1]
                + t * (BRADLEY_FLOW_FACTOR[i] - BRADLEY_FLOW_FACTOR[i - 1]);
        }
    }
    BRADLEY_FLOW_FACTOR[BRADLEY_FLOW_FACTOR.len() - 1]
}

pub(crate) fn weir_submergence_ratio(tw_m: f64, e_upstream: f64, crest_m: f64) -> f64 {
    let tail_above = (tw_m - crest_m).max(0.0);
    let head_above = (e_upstream - crest_m).max(1e-6);
    (tail_above / head_above).clamp(0.0, 1.0)
}

/// Maximum Bradley submergence ratio over deck segments where $E_{up}$ clears the local crest.
pub(crate) fn max_active_weir_submergence_ratio(
    tw_m: f64,
    e_upstream: f64,
    geom: &BridgeGeometry,
) -> f64 {
    if let Some(deck) = geom.deck.as_ref().filter(|d| d.is_valid()) {
        let mut max_ratio = 0.0_f64;
        for i in 0..deck.stations_m.len().saturating_sub(1) {
            let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
            if w <= 0.0 {
                continue;
            }
            let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
            let crest = effective_deck_crest_m(
                geom,
                interpolate_profile(&deck.stations_m, &deck.high_elevations_m, s_mid),
            );
            if e_upstream > crest + 1e-6 {
                max_ratio = max_ratio.max(weir_submergence_ratio(tw_m, e_upstream, crest));
            }
        }
        max_ratio
    } else if e_upstream > effective_scalar_high_chord_m(geom) + 1e-6 {
        weir_submergence_ratio(tw_m, e_upstream, effective_scalar_high_chord_m(geom))
    } else {
        0.0
    }
}

pub(crate) fn weir_submergence_exceeds_cap(
    tw_m: f64,
    e_upstream: f64,
    geom: &BridgeGeometry,
) -> bool {
    max_active_weir_submergence_ratio(tw_m, e_upstream, geom) >= geom.max_weir_submergence
}

/// Segment-wise Bradley weir overtopping (HEC-RAS effective length per crest segment).
pub(crate) fn segment_weir_discharge_m3s(tw_m: f64, e_upstream: f64, geom: &BridgeGeometry) -> f64 {
    if let Some(deck) = geom.deck.as_ref().filter(|d| d.is_valid()) {
        let mut q = 0.0;
        for i in 0..deck.stations_m.len().saturating_sub(1) {
            let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
            if w <= 0.0 {
                continue;
            }
            let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
            let crest = effective_deck_crest_m(
                geom,
                interpolate_profile(&deck.stations_m, &deck.high_elevations_m, s_mid),
            );
            let h = (e_upstream - crest).max(0.0);
            if h <= 1e-6 {
                continue;
            }
            let sub_ratio = weir_submergence_ratio(tw_m, e_upstream, crest);
            let factor = bradley_weir_submergence_factor(sub_ratio);
            q += geom.weir_coeff_m * factor * w * h.powf(1.5);
        }
        q
    } else {
        0.0
    }
}

pub(crate) fn main_pressure_flow_discharge(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    a_net: f64,
) -> f64 {
    if wsel_up <= geom.low_chord_m + 1e-6 {
        return 0.0;
    }

    if tw_m >= geom.low_chord_max_m {
        let e_up = upstream_energy_grade(wsel_up, q_metric, geom, table_up, geom.z_up_m, true);
        let head = (e_up - tw_m).max(0.0);
        geom.pressure_coeff_submerged * a_net * (2.0 * G_METRIC * head).sqrt()
    } else {
        let z = opening_height_below_deck_m(geom);
        let y3 = (wsel_up - geom.z_up_m).max(1e-3);
        let cd = sluice_gate_discharge_coeff(y3 / z, geom.pressure_coeff_inlet);
        let props = obstructed_hydraulics(table_up, wsel_up, geom.z_up_m, geom, true, false);
        let v_head = velocity_head(q_metric, props.a_eff);
        let drive = (y3 - 0.5 * z + v_head).max(0.0);
        cd * a_net * (2.0 * G_METRIC * drive).sqrt()
    }
}

pub(crate) fn deck_vents_active_at_wsel(geom: &BridgeGeometry, wsel_m: f64) -> bool {
    geom.deck_vents.iter().any(|v| wsel_m > v.invert_m + 1e-9)
}

pub(crate) fn deck_vent_pressure_discharge_m3s(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
) -> f64 {
    if !deck_vents_active_at_wsel(geom, wsel_up) {
        return 0.0;
    }
    let e_up = upstream_energy_grade(wsel_up, q_metric, geom, table_up, geom.z_up_m, true);
    total_deck_vent_discharge_m3s(&geom.deck_vents, wsel_up, e_up, tw_m)
}

/// High-flow discharge split: main opening under low chord, deck vents/slots, roadway weir.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct HighFlowDischargeComponents {
    pub q_opening_m3s: f64,
    pub q_vents_m3s: f64,
    pub q_weir_m3s: f64,
}

impl HighFlowDischargeComponents {
    pub fn total_m3s(self) -> f64 {
        self.q_opening_m3s + self.q_vents_m3s + self.q_weir_m3s
    }

    pub fn pressure_paths_m3s(self) -> f64 {
        self.q_opening_m3s + self.q_vents_m3s
    }
}

/// Combined high flow: $Q = Q_{opening} + Q_{vents} + Q_{weir}$.
///
/// Pass `weir_length_m = None` for pressure paths only (opening + vents).
pub(crate) fn combined_high_flow_discharge(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    a_net: f64,
    weir_length_m: Option<f64>,
) -> HighFlowDischargeComponents {
    let q_opening = if wsel_up > geom.low_chord_m + 1e-6 {
        main_pressure_flow_discharge(wsel_up, tw_m, q_metric, geom, table_up, a_net)
    } else {
        0.0
    };
    let q_vents = deck_vent_pressure_discharge_m3s(wsel_up, tw_m, q_metric, geom, table_up);
    let q_weir = weir_length_m
        .filter(|&l| l > 1e-6)
        .map(|l| weir_flow_discharge(wsel_up, tw_m, q_metric, geom, table_up, l))
        .unwrap_or(0.0);
    HighFlowDischargeComponents {
        q_opening_m3s: q_opening,
        q_vents_m3s: q_vents,
        q_weir_m3s: q_weir,
    }
}

/// Main opening pressure flow plus parallel vent/slot paths when the deck blocks the primary opening.
pub(crate) fn pressure_flow_discharge(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    a_net: f64,
) -> f64 {
    combined_high_flow_discharge(wsel_up, tw_m, q_metric, geom, table_up, a_net, None)
        .pressure_paths_m3s()
}

pub(crate) fn weir_flow_discharge(
    wsel_up: f64,
    tw_m: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    l_weir: f64,
) -> f64 {
    let e_up = upstream_energy_grade(wsel_up, q_metric, geom, table_up, geom.z_up_m, true);
    if geom.deck.as_ref().is_some_and(|d| d.is_valid()) {
        return segment_weir_discharge_m3s(tw_m, e_up, geom);
    }
    let crest = effective_scalar_high_chord_m(geom);
    let h_weir = (e_up - crest).max(0.0);
    if h_weir <= 1e-6 {
        return 0.0;
    }
    let l = l_weir.max(1e-3);
    let sub_ratio = weir_submergence_ratio(tw_m, e_up, crest);
    let factor = bradley_weir_submergence_factor(sub_ratio);
    geom.weir_coeff_m * factor * l * h_weir.powf(1.5)
}

pub(crate) fn solve_pressure_headwater(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let a_net = net_opening_area_at_low_chord(geom, table_up, table_down);
    let mut low = tw_m.max(geom.z_up_m + 1e-4);
    let mut high = geom.low_chord_m + 30.0;
    let mut best = geom.low_chord_m;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let q_calc = pressure_flow_discharge(mid, tw_m, q_metric, geom, table_up, a_net);
        if (q_calc - q_metric).abs() < 1e-6 {
            return mid;
        }
        if q_calc < q_metric {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

pub(crate) fn weir_head_active_at_energy(e_upstream: f64, geom: &BridgeGeometry) -> bool {
    if let Some(deck) = geom.deck.as_ref().filter(|d| d.is_valid()) {
        for i in 0..deck.stations_m.len().saturating_sub(1) {
            let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
            let crest = effective_deck_crest_m(
                geom,
                interpolate_profile(&deck.stations_m, &deck.high_elevations_m, s_mid),
            );
            if e_upstream > crest + 1e-6 {
                return true;
            }
        }
        false
    } else {
        e_upstream > effective_scalar_high_chord_m(geom) + 1e-6
    }
}

pub(crate) fn solve_high_flow(
    q_metric: f64,
    geom: &BridgeGeometry,
    tw_clamped: f64,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> BridgeHeadwaterSolve {
    if geom.high_flow_method == HighFlowMethod::Energy {
        return BridgeHeadwaterSolve {
            wsel_m: solve_high_flow_energy(q_metric, tw_clamped, geom, table_up, table_down),
            regime: BridgeFlowRegimeKind::Energy,
        };
    }

    let a_net = net_opening_area_at_low_chord(geom, table_up, table_down);
    let fallback_weir_width = table_up.interpolate(geom.high_chord_m).top_width.max(1.0);
    let pressure_only = solve_pressure_headwater(q_metric, tw_clamped, geom, table_up, table_down);
    let e_pressure =
        upstream_energy_grade(pressure_only, q_metric, geom, table_up, geom.z_up_m, true);

    if weir_submergence_exceeds_cap(tw_clamped, e_pressure, geom) {
        return BridgeHeadwaterSolve {
            wsel_m: solve_high_flow_energy_fallback(
                q_metric, tw_clamped, geom, table_up, table_down,
            ),
            regime: BridgeFlowRegimeKind::Energy,
        };
    }

    let q_weir_at_pressure = weir_flow_discharge(
        pressure_only,
        tw_clamped,
        q_metric,
        geom,
        table_up,
        fallback_weir_width,
    );
    if q_weir_at_pressure <= 1e-9 {
        return BridgeHeadwaterSolve {
            wsel_m: pressure_only,
            regime: BridgeFlowRegimeKind::Pressure,
        };
    }

    let combined_q_at = |h_up: f64| -> f64 {
        let e_up = upstream_energy_grade(h_up, q_metric, geom, table_up, geom.z_up_m, true);
        let l_weir = effective_weir_length_m(geom, e_up, fallback_weir_width);
        combined_high_flow_discharge(
            h_up,
            tw_clamped,
            q_metric,
            geom,
            table_up,
            a_net,
            Some(l_weir),
        )
        .total_m3s()
    };

    let mut low = tw_clamped.max(geom.z_up_m + 1e-4);
    let mut high = pressure_only.max(geom.high_chord_m).max(low + 1e-3);
    if combined_q_at(high) < q_metric {
        high = high + 50.0;
    } else if combined_q_at(low) > q_metric {
        // Weir adds capacity below the pressure-only headwater.
        high = pressure_only;
    }

    let residual = |h_up: f64| -> f64 {
        if weir_submergence_exceeds_cap(
            tw_clamped,
            upstream_energy_grade(h_up, q_metric, geom, table_up, geom.z_up_m, true),
            geom,
        ) {
            return -1.0;
        }
        combined_q_at(h_up) - q_metric
    };

    let mut best_h = high;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let e_up = upstream_energy_grade(mid, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_exceeds_cap(tw_clamped, e_up, geom) {
            return BridgeHeadwaterSolve {
                wsel_m: solve_high_flow_energy_fallback(
                    q_metric, tw_clamped, geom, table_up, table_down,
                ),
                regime: BridgeFlowRegimeKind::Energy,
            };
        }
        let res = residual(mid);
        if res.abs() < 1e-8 {
            best_h = mid;
            break;
        }
        if res < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best_h = mid;
    }

    let e_best = upstream_energy_grade(best_h, q_metric, geom, table_up, geom.z_up_m, true);
    if weir_submergence_exceeds_cap(tw_clamped, e_best, geom) {
        return BridgeHeadwaterSolve {
            wsel_m: solve_high_flow_energy_fallback(
                q_metric, tw_clamped, geom, table_up, table_down,
            ),
            regime: BridgeFlowRegimeKind::Energy,
        };
    }

    let l_weir = effective_weir_length_m(geom, e_best, fallback_weir_width);
    let parts = combined_high_flow_discharge(
        best_h,
        tw_clamped,
        q_metric,
        geom,
        table_up,
        a_net,
        Some(l_weir),
    );
    let regime = if parts.q_weir_m3s > 1e-6 {
        BridgeFlowRegimeKind::Weir
    } else {
        BridgeFlowRegimeKind::Pressure
    };
    BridgeHeadwaterSolve {
        wsel_m: best_h,
        regime,
    }
}
pub(crate) fn solve_high_flow_energy_tailwater(
    q_metric: f64,
    hw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let mut low = geom.z_down_m + 1e-4;
    let mut high = hw_m.max(geom.low_chord_m);
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let hw_calc = solve_high_flow_energy(q_metric, mid, geom, table_up, table_down);
        if (hw_calc - hw_m).abs() < 1e-4 {
            return mid;
        }
        if hw_calc < hw_m {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

pub(crate) fn solve_high_flow_tailwater(
    q_metric: f64,
    geom: &BridgeGeometry,
    hw_m: f64,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    if geom.high_flow_method == HighFlowMethod::Energy {
        return solve_high_flow_energy_tailwater(q_metric, hw_m, geom, table_up, table_down);
    }

    let a_net = net_opening_area_at_low_chord(geom, table_up, table_down);
    let fallback_weir_width = table_down.interpolate(geom.high_chord_m).top_width.max(1.0);
    let e_up = upstream_energy_grade(hw_m, q_metric, geom, table_up, geom.z_up_m, true);

    if !weir_head_active_at_energy(e_up, geom) {
        let mut low = geom.z_down_m + 1e-4;
        let mut high = hw_m;
        let mut best = low;
        for _ in 0..50 {
            let mid = 0.5 * (low + high);
            let q_calc = pressure_flow_discharge(hw_m, mid, q_metric, geom, table_up, a_net);
            if (q_calc - q_metric).abs() < 1e-6 {
                return mid;
            }
            if q_calc < q_metric {
                high = mid;
            } else {
                low = mid;
            }
            best = mid;
        }
        return best;
    }

    let residual = |tw: f64| -> f64 {
        let e_up = upstream_energy_grade(hw_m, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_exceeds_cap(tw, e_up, geom) {
            return -1.0;
        }
        let l_weir = effective_weir_length_m(geom, e_up, fallback_weir_width);
        combined_high_flow_discharge(hw_m, tw, q_metric, geom, table_up, a_net, Some(l_weir))
            .total_m3s()
            - q_metric
    };

    // Submerged-orifice tailwater applies for TW ≥ max low chord; sluice Q below that
    // discontinuity is not monotonic with the combined-weir balance.
    let mut low = if hw_m >= geom.low_chord_m {
        geom.low_chord_max_m.max(geom.z_down_m + 1e-4)
    } else {
        geom.z_down_m + 1e-4
    };
    let mut high = hw_m;
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let e_up = upstream_energy_grade(hw_m, q_metric, geom, table_up, geom.z_up_m, true);
        if weir_submergence_exceeds_cap(mid, e_up, geom) {
            return solve_high_flow_energy_fallback(q_metric, mid, geom, table_up, table_down);
        }
        let res = residual(mid);
        if res.abs() < 1e-8 {
            return mid;
        }
        if res > 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}
