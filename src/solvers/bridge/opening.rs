use crate::geometry::{
    flow_area_for_row, row_at_elevation, CrossSection, GeometryRow, GeometryTable, GuideBanks,
    IneffectiveFlowAreas,
};
use crate::solvers::pier_geometry::{
    evenly_spaced_pier_stations, resolve_pier_width_specs, ResolvedPier,
};
use crate::utils::G_METRIC;

use super::section::BridgeFrictionWeighting;

use super::geometry::{
    apply_opening_blockage, capped_ice_thickness_m, effective_deck_crest_m,
    effective_scalar_high_chord_m, effective_z_bed_m, interpolate_profile,
    opening_station_bounds_from_deck, scale_base_area_for_ice, BridgeGeometry,
};

pub(crate) fn pier_floating_debris_obstruction_m2(geom: &BridgeGeometry, wsel: f64, z_bed: f64) -> f64 {
    let z_eff = effective_z_bed_m(z_bed, geom);
    let depth = (wsel - z_eff).max(0.0);
    if depth < 1e-9 {
        return 0.0;
    }
    let (s_min, s_max) = opening_station_bounds_m(geom);
    let cos = geom.skew_cos.max(1e-6);
    let mut total = 0.0;
    for (i, pier) in active_resolved_piers(geom).iter().enumerate() {
        let w_debris = geom.pier_debris_widths_m.get(i).copied().unwrap_or(0.0);
        let h_debris = geom.pier_debris_heights_m.get(i).copied().unwrap_or(0.0);
        if w_debris < 1e-9 || h_debris < 1e-9 {
            continue;
        }
        let pier_w = pier.spec.width_perp_at(wsel).max(0.0) / cos;
        let w = w_debris.max(pier_w);
        let half = w * 0.5;
        let s_lo = (pier.station_m - half).max(s_min);
        let s_hi = (pier.station_m + half).min(s_max);
        if s_hi <= s_lo + 1e-9 {
            continue;
        }
        let h = h_debris.min(depth);
        let gross = (s_hi - s_lo) * h;
        let pier_a = pier.submerged_area_m2(wsel, z_eff) / cos;
        total += (gross - pier_a).max(0.0);
    }
    total
}

pub(crate) fn ineffective_for_side(geom: &BridgeGeometry, is_upstream: bool) -> Option<&IneffectiveFlowAreas> {
    if is_upstream {
        geom.ineffective_up.as_ref()
    } else {
        geom.ineffective_down.as_ref()
    }
}
#[derive(Debug, Copy, Clone)]
pub(crate) struct ObstructedHydraulics {
    pub(crate) a_eff: f64,
    pub(crate) area_moment: f64,
    pub(crate) top_width: f64,
}

pub(crate) fn lookup_row(
    table: &GeometryTable,
    xs: Option<&CrossSection>,
    ineffective: Option<&IneffectiveFlowAreas>,
    guide_banks: Option<&GuideBanks>,
    wsel: f64,
) -> GeometryRow {
    if let Some(xs) = xs {
        row_at_elevation(table, xs, wsel, ineffective, guide_banks)
    } else {
        let row = table.interpolate(wsel);
        GeometryRow {
            active_area: row.area,
            active_channel_area: row.channel_area,
            ..row
        }
    }
}

pub(crate) fn base_flow_area(
    row: &GeometryRow,
    ineffective: Option<&IneffectiveFlowAreas>,
    guide_banks: Option<&GuideBanks>,
) -> f64 {
    let has_ineffective = ineffective.filter(|i| i.is_configured()).is_some()
        || row.active_area + 1e-6 < row.area;
    let has_guide = guide_banks.filter(|g| g.is_configured()).is_some();
    if has_ineffective && !has_guide {
        return flow_area_for_row(row);
    }
    let clip_active = has_ineffective || has_guide;
    if row.channel_area > 1e-6 {
        if clip_active {
            row.active_channel_area
        } else {
            row.channel_area
        }
    } else if clip_active {
        row.active_area
    } else {
        row.area
    }
}

pub(crate) fn ineffective_on_cut(xs: Option<&CrossSection>) -> Option<&IneffectiveFlowAreas> {
    xs.and_then(|x| x.ineffective_flow_areas.as_ref())
        .filter(|i| i.is_configured())
}

pub(crate) fn guide_banks_configured_on_side(geom: &BridgeGeometry, is_approach: bool) -> bool {
    if is_approach {
        geom.guide_banks_approach
            .as_ref()
            .is_some_and(|g| g.is_configured())
    } else {
        geom.guide_banks_departure
            .as_ref()
            .is_some_and(|g| g.is_configured())
    }
}

pub(crate) fn approach_departure_cut_modifiers_active(geom: &BridgeGeometry, is_approach: bool) -> bool {
    if guide_banks_configured_on_side(geom, is_approach) {
        return true;
    }
    let xs = if is_approach {
        geom.xs_approach.as_ref()
    } else {
        geom.xs_departure.as_ref()
    };
    ineffective_on_cut(xs).is_some()
}

/// Active flow area on approach or departure cut (guide banks and/or ineffective on that cut).
pub(crate) fn reach_cut_flow_area(geom: &BridgeGeometry, is_approach: bool, wsel: f64) -> Option<f64> {
    if !approach_departure_cut_modifiers_active(geom, is_approach) {
        return None;
    }
    let (xs, table, guide_banks) = if is_approach {
        (
            geom.xs_approach.as_ref(),
            geom.table_approach.as_ref(),
            geom.guide_banks_approach.as_ref(),
        )
    } else {
        (
            geom.xs_departure.as_ref(),
            geom.table_departure.as_ref(),
            geom.guide_banks_departure.as_ref(),
        )
    };
    let xs = xs?;
    let table = table?;
    let ineffective = ineffective_on_cut(Some(xs));
    let guide_banks = guide_banks.filter(|g| g.is_configured());
    let row = lookup_row(table, Some(xs), ineffective, guide_banks, wsel);
    Some(base_flow_area(&row, ineffective, guide_banks))
}

pub(crate) fn section_xs<'a>(geom: &'a BridgeGeometry, is_upstream: bool) -> Option<&'a CrossSection> {
    if is_upstream {
        geom.xs_up.as_ref()
    } else {
        geom.xs_down.as_ref()
    }
}

pub(crate) fn opening_station_bounds_m(geom: &BridgeGeometry) -> (f64, f64) {
    opening_station_bounds_from_deck(geom.deck.as_ref())
}

pub(crate) fn gross_projected_opening_width_m(geom: &BridgeGeometry) -> f64 {
    let (s0, s1) = opening_station_bounds_m(geom);
    (s1 - s0).max(0.0) * geom.skew_cos
}

pub(crate) fn legacy_resolved_piers(geom: &BridgeGeometry) -> Vec<ResolvedPier> {
    let (s_min, s_max) = opening_station_bounds_m(geom);
    let inset = geom.pier_width_m.max(0.0) * 0.5;
    let stations = if !geom.pier_stations_m.is_empty() {
        geom.pier_stations_m.clone()
    } else {
        evenly_spaced_pier_stations(geom.num_piers, s_min, s_max, inset)
    };
    let z_bed = geom.z_up_m.min(geom.z_down_m);
    let z_tops: Vec<f64> = stations
        .iter()
        .map(|&s| {
            geom.deck
                .as_ref()
                .map(|d| interpolate_profile(&d.stations_m, &d.low_elevations_m, s))
                .unwrap_or(geom.low_chord_m)
        })
        .collect();
    resolve_pier_width_specs(
        geom.pier_width_m,
        &stations,
        z_bed,
        &z_tops,
        None,
        None,
    )
}

pub(crate) fn active_resolved_piers(geom: &BridgeGeometry) -> Vec<ResolvedPier> {
    let piers = if geom.pier_specs.is_empty() {
        legacy_resolved_piers(geom)
    } else {
        geom.pier_specs.clone()
    };
    piers
        .into_iter()
        .filter(|p| pier_in_opening_span(geom, p))
        .collect()
}

pub(crate) fn pier_half_width_opening_m(geom: &BridgeGeometry, pier: &ResolvedPier) -> f64 {
    pier.spec.width_perp_at(geom.low_chord_m).max(0.0) / geom.skew_cos * 0.5
}

pub(crate) fn pier_in_opening_span(geom: &BridgeGeometry, pier: &ResolvedPier) -> bool {
    let (s_min, s_max) = opening_station_bounds_m(geom);
    let half = pier_half_width_opening_m(geom, pier);
    pier.station_m + half > s_min && pier.station_m - half < s_max
}

pub(crate) fn total_pier_flow_width_at_wsel_m(geom: &BridgeGeometry, wsel: f64, z_bed: f64) -> f64 {
    let z_eff = effective_z_bed_m(z_bed, geom);
    let piers = active_resolved_piers(geom);
    crate::solvers::pier_geometry::total_pier_flow_width_at_wsel_m(
        &piers,
        wsel,
        z_eff,
        geom.skew_cos,
    )
}

pub(crate) fn pier_submerged_area_at_wsel(geom: &BridgeGeometry, wsel: f64, z_bed: f64) -> f64 {
    let z_eff = effective_z_bed_m(z_bed, geom);
    let piers = active_resolved_piers(geom);
    crate::solvers::pier_geometry::total_submerged_pier_area_m2(
        &piers,
        wsel,
        z_eff,
        geom.skew_cos,
    )
}

/// Downstream flow area for Yarnell: base area minus per-side abutments, before pier blockage.
pub(crate) fn yarnell_downstream_flow_area_m2(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
) -> f64 {
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, false, false);
    (props.a_eff + pier_submerged_area_at_wsel(geom, wsel, z_bed)).max(1e-5)
}

/// HEC-RAS weighting: use the more constricted of BU and BD at a common water-surface elevation.
pub(crate) fn obstructed_opening_at_wsel(
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
    wsel: f64,
) -> (ObstructedHydraulics, bool) {
    let up = obstructed_hydraulics(table_up, wsel, geom.z_up_m, geom, true, false);
    let down = obstructed_hydraulics(table_down, wsel, geom.z_down_m, geom, false, false);
    if up.a_eff <= down.a_eff {
        (up, true)
    } else {
        (down, false)
    }
}

/// Vertical opening below the low chord (minimum of BU and BD invert depths).
pub(crate) fn opening_height_below_deck_m(geom: &BridgeGeometry) -> f64 {
    let ice_up = capped_ice_thickness_m(geom, geom.z_up_m);
    let ice_down = capped_ice_thickness_m(geom, geom.z_down_m);
    let h_up = (geom.low_chord_m - geom.z_up_m - ice_up).max(0.0);
    let h_down = (geom.low_chord_m - geom.z_down_m - ice_down).max(0.0);
    h_up.min(h_down).max(1e-3)
}

pub(crate) fn deck_obstructed_area_at_wsel(geom: &BridgeGeometry, wsel: f64) -> f64 {
    if let Some(deck) = geom.deck.as_ref().filter(|d| d.is_valid()) {
        let mut area = 0.0;
        for i in 0..deck.stations_m.len().saturating_sub(1) {
            let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
            if w <= 0.0 {
                continue;
            }
            let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
            let lc = interpolate_profile(&deck.stations_m, &deck.low_elevations_m, s_mid);
            let hc = effective_deck_crest_m(
                geom,
                interpolate_profile(&deck.stations_m, &deck.high_elevations_m, s_mid),
            );
            if wsel <= lc {
                continue;
            } else if wsel <= hc {
                area += w * (wsel - lc);
            } else {
                area += w * (hc - lc);
            }
        }
        area
    } else {
        let lc = geom.low_chord_m;
        let hc = effective_scalar_high_chord_m(geom);
        let w = gross_projected_opening_width_m(geom);
        if wsel <= lc {
            0.0
        } else if wsel <= hc {
            w * (wsel - lc)
        } else {
            w * (hc - lc)
        }
    }
}

pub(crate) fn deck_obstructed_width_at_wsel(geom: &BridgeGeometry, wsel: f64) -> f64 {
    if let Some(deck) = geom.deck.as_ref().filter(|d| d.is_valid()) {
        let mut width = 0.0;
        for i in 0..deck.stations_m.len().saturating_sub(1) {
            let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
            if w <= 0.0 {
                continue;
            }
            let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
            let lc = interpolate_profile(&deck.stations_m, &deck.low_elevations_m, s_mid);
            let hc = effective_deck_crest_m(
                geom,
                interpolate_profile(&deck.stations_m, &deck.high_elevations_m, s_mid),
            );
            if wsel > lc && wsel <= hc {
                width += w;
            }
        }
        width
    } else {
        let lc = geom.low_chord_m;
        let hc = effective_scalar_high_chord_m(geom);
        let w = gross_projected_opening_width_m(geom);
        if wsel > lc && wsel <= hc {
            w
        } else {
            0.0
        }
    }
}

pub(crate) fn obstructed_hydraulics(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
    subtract_deck: bool,
) -> ObstructedHydraulics {
    let ineffective = ineffective_for_side(geom, is_upstream);
    let row = lookup_row(
        table,
        section_xs(geom, is_upstream),
        ineffective,
        None,
        wsel,
    );
    let a_base = scale_base_area_for_ice(
        base_flow_area(&row, ineffective, None),
        wsel,
        z_bed,
        geom,
    );
    let z_eff = effective_z_bed_m(z_bed, geom);
    let depth = (wsel - z_eff).max(0.0);
    let a_piers = pier_submerged_area_at_wsel(geom, wsel, z_bed);
    let a_abut = geom.abutments.submerged_area_m2(wsel, z_eff);
    let a_debris = pier_floating_debris_obstruction_m2(geom, wsel, z_bed);
    let a_deck = if subtract_deck {
        deck_obstructed_area_at_wsel(geom, wsel)
    } else {
        0.0
    };
    let a_eff = apply_opening_blockage(
        (a_base - a_piers - a_abut - a_debris - a_deck).max(1e-5),
        geom,
    );

    let full_moment = table.calculate_area_moment(wsel);
    let area_moment = if a_base > 1e-5 {
        full_moment * (a_eff / a_base)
    } else {
        a_eff * depth * 0.5
    };

    let t_base = if row.channel_area > 1e-6 {
        row.top_width.min(a_base / depth.max(1e-3))
    } else {
        row.top_width
    };
    let abut_width_at_wsel = geom.abutments.submerged_width_at_wsel_m(wsel, z_eff);
    let w_deck = if subtract_deck {
        deck_obstructed_width_at_wsel(geom, wsel)
    } else {
        0.0
    };
    let top_width = (t_base
        - total_pier_flow_width_at_wsel_m(geom, wsel, z_bed)
        - abut_width_at_wsel
        - w_deck)
        .max(1e-3);

    ObstructedHydraulics {
        a_eff,
        area_moment,
        top_width,
    }
}

pub(crate) fn specific_force(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    q: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> f64 {
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, is_upstream, false);
    if props.a_eff < 1e-6 {
        return f64::INFINITY;
    }
    (q * q) / (G_METRIC * props.a_eff) + props.area_moment
}

pub(crate) fn obstructed_conveyance(
    table: &GeometryTable,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> f64 {
    let ineffective = ineffective_for_side(geom, is_upstream);
    let row = lookup_row(table, section_xs(geom, is_upstream), ineffective, None, wsel);
    let a_base = base_flow_area(&row, ineffective, None);
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, is_upstream, false);
    if a_base > 1e-6 {
        row.conveyance * (props.a_eff / a_base)
    } else {
        0.0
    }
}

pub(crate) fn velocity_head(q: f64, area: f64) -> f64 {
    if area < 1e-6 {
        return 0.0;
    }
    (q * q) / (2.0 * G_METRIC * area * area)
}

pub(crate) fn friction_loss(q: f64, k1: f64, k2: f64, length: f64) -> f64 {
    let k_avg = 0.5 * (k1 + k2).max(1e-6);
    length * (q / k_avg).powi(2)
}

pub(crate) fn bridge_opening_friction_loss(
    q_metric: f64,
    wsel_bu: f64,
    tw_bd: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let opening_l = if geom.friction_opening_m > 1e-6 {
        geom.friction_opening_m
    } else {
        geom.length_m
    };
    let k_bu = obstructed_conveyance(table_up, wsel_bu, geom.z_up_m, geom, true);
    let k_bd = obstructed_conveyance(table_down, tw_bd, geom.z_down_m, geom, false);
    if geom.internal_opening_segment_lengths_m.is_empty() {
        return friction_loss(q_metric, k_bd, k_bu, opening_l);
    }

    let seg_lens = &geom.internal_opening_segment_lengths_m;
    let total: f64 = seg_lens.iter().sum();
    let mut cum = 0.0;
    let mut conveyances = Vec::with_capacity(seg_lens.len() + 1);
    conveyances.push(k_bu);
    for (i, int_table) in geom.internal_opening_tables.iter().enumerate() {
        cum += seg_lens[i];
        let frac = if total > 1e-6 { cum / total } else { 0.0 };
        let wsel = wsel_bu + frac * (tw_bd - wsel_bu);
        let z = geom
            .internal_opening_z_m
            .get(i)
            .copied()
            .unwrap_or(geom.z_up_m);
        conveyances.push(obstructed_conveyance(int_table, wsel, z, geom, true));
    }
    conveyances.push(k_bd);

    seg_lens
        .iter()
        .enumerate()
        .map(|(i, &l)| friction_loss(q_metric, conveyances[i + 1], conveyances[i], l))
        .sum()
}

pub(crate) fn cut_conveyance_at_wsel(
    xs: &CrossSection,
    table: &GeometryTable,
    guide_banks: Option<&GuideBanks>,
    wsel: f64,
) -> f64 {
    let ineffective = ineffective_on_cut(Some(xs));
    let guide_banks = guide_banks.filter(|g| g.is_configured());
    let row = lookup_row(table, Some(xs), ineffective, guide_banks, wsel);
    row.conveyance.max(0.0)
}

/// Energy / WSPRO friction through the bridge reach (opening-only or HEC-RAS three-segment).
pub(crate) fn bridge_energy_friction_loss(
    q_metric: f64,
    wsel_up: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let hf_opening = bridge_opening_friction_loss(q_metric, wsel_up, tw_m, geom, table_up, table_down);
    if geom.friction_weighting == BridgeFrictionWeighting::OpeningOnly {
        return hf_opening;
    }
    let k_bu = obstructed_conveyance(table_up, wsel_up, geom.z_up_m, geom, true);
    let k_bd = obstructed_conveyance(table_down, tw_m, geom.z_down_m, geom, false);
    let mut hf = hf_opening;
    if geom.friction_approach_m > 1e-6 {
        if let (Some(xs), Some(table)) = (geom.xs_approach.as_ref(), geom.table_approach.as_ref()) {
            let k_ap = cut_conveyance_at_wsel(
                xs,
                table,
                geom.guide_banks_approach.as_ref(),
                wsel_up,
            );
            if k_ap > 1e-6 {
                hf += friction_loss(q_metric, k_bu, k_ap, geom.friction_approach_m);
            }
        }
    }
    if geom.friction_departure_m > 1e-6 {
        if let (Some(xs), Some(table)) = (geom.xs_departure.as_ref(), geom.table_departure.as_ref())
        {
            let z_dep = crate::solvers::bridge_interior::cross_section_min_bed(xs);
            let k_dep = cut_conveyance_at_wsel(
                xs,
                table,
                geom.guide_banks_departure.as_ref(),
                tw_m.max(z_dep + 1e-4),
            );
            if k_dep > 1e-6 {
                hf += friction_loss(q_metric, k_dep, k_bd, geom.friction_departure_m);
            }
        }
    }
    hf
}

/// WSPRO idealized contraction loss (HEC-RAS eq. 10) for approach area A1 and bridge opening A2.
pub(crate) fn wspro_contraction_loss(q: f64, a_approach: f64, a_bridge: f64, c: f64) -> f64 {
    if a_approach < 1e-6 || a_bridge < 1e-6 || c < 1e-6 {
        return 0.0;
    }
    let ratio = a_approach / a_bridge;
    let alpha_2 = 1.0 / (c * c);
    let beta_2 = 1.0 / c;
    let alpha_1 = 1.0;
    let beta_1 = 1.0;
    (q * q) / (2.0 * G_METRIC * a_approach.powi(2))
        * (2.0 * beta_1 - alpha_1 - 2.0 * beta_2 * ratio + alpha_2 * ratio * ratio)
}
