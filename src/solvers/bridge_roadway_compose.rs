//! Unified roadway embankment input — composes deck, abutment, ineffective, and blocked profiles.

use crate::geometry::BlockedObstruction;
use crate::solvers::bridge::BridgeSolveParams;
use crate::solvers::steady::SteadyInputs;

/// Piecewise polyline in opening coordinates (stations monotonic increasing, ≥ 2 points).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EmbankmentPolyline {
    pub stations: Vec<f64>,
    pub elevations: Vec<f64>,
}

impl EmbankmentPolyline {
    pub fn is_valid(&self) -> bool {
        let n = self.stations.len();
        n >= 2 && n == self.elevations.len() && self.stations.windows(2).all(|w| w[1] > w[0])
    }
}

/// Deck low/high chord profile (opening frame).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BridgeDeckInput {
    pub stations: Vec<f64>,
    pub low_elevations: Vec<f64>,
    pub high_elevations: Vec<f64>,
}

impl BridgeDeckInput {
    pub fn is_valid(&self) -> bool {
        let n = self.stations.len();
        n >= 2
            && n == self.low_elevations.len()
            && n == self.high_elevations.len()
            && self.stations.windows(2).all(|w| w[1] > w[0])
    }

    pub fn low_min(&self) -> f64 {
        self.low_elevations
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
    }

    pub fn high_max(&self) -> f64 {
        self.high_elevations
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn interpolate_high(&self, station: f64) -> f64 {
        interpolate_profile(&self.stations, &self.high_elevations, station)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct IneffectiveBlockPoint {
    pub station: f64,
    pub elevation: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RoadwayAbutmentInput {
    #[serde(default)]
    pub outer_station: Option<f64>,
    pub width: f64,
    #[serde(default)]
    pub top_elevation: Option<f64>,
    #[serde(default)]
    pub top_profile: Option<EmbankmentPolyline>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RoadwayEmbankmentSide {
    /// Grade line (toe → crest) in opening frame. Drives ineffective activation and blocked top when derived.
    #[serde(default)]
    pub embankment_profile: Option<EmbankmentPolyline>,
    #[serde(default)]
    pub ineffective_blocks: Option<Vec<IneffectiveBlockPoint>>,
    #[serde(default)]
    pub abutment: Option<RoadwayAbutmentInput>,
    #[serde(default)]
    pub derive_ineffective: Option<bool>,
    #[serde(default)]
    pub derive_blocked: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BridgeIneffectiveFaceOverride {
    #[serde(default)]
    pub left_blocks: Option<Vec<IneffectiveBlockPoint>>,
    #[serde(default)]
    pub right_blocks: Option<Vec<IneffectiveBlockPoint>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BridgeIneffectiveFacesOverride {
    #[serde(default)]
    pub upstream: Option<BridgeIneffectiveFaceOverride>,
    #[serde(default)]
    pub downstream: Option<BridgeIneffectiveFaceOverride>,
}

/// Unified bridge opening geometry (API v26). Composes flat `bridge_*` fields before hydraulics.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BridgeRoadwayEmbankment {
    pub deck: BridgeDeckInput,
    #[serde(default)]
    pub left: Option<RoadwayEmbankmentSide>,
    #[serde(default)]
    pub right: Option<RoadwayEmbankmentSide>,
    #[serde(default)]
    pub ineffective_faces: Option<BridgeIneffectiveFacesOverride>,
    #[serde(default)]
    pub derive_ineffective: Option<bool>,
}

pub fn interpolate_profile(stations: &[f64], elevations: &[f64], station: f64) -> f64 {
    if stations.is_empty() {
        return 0.0;
    }
    if station <= stations[0] {
        return elevations[0];
    }
    if station >= stations[stations.len() - 1] {
        return elevations[elevations.len() - 1];
    }
    for i in 0..stations.len() - 1 {
        if station <= stations[i + 1] {
            let t = (station - stations[i]) / (stations[i + 1] - stations[i]);
            return elevations[i] + t * (elevations[i + 1] - elevations[i]);
        }
    }
    elevations[elevations.len() - 1]
}

fn blocks_from_points(points: &[IneffectiveBlockPoint]) -> (Vec<f64>, Vec<f64>) {
    let stations: Vec<f64> = points.iter().map(|p| p.station).collect();
    let elevations: Vec<f64> = points.iter().map(|p| p.elevation).collect();
    (stations, elevations)
}

fn ineffective_points_from_side(
    side: Option<&RoadwayEmbankmentSide>,
    deck: &BridgeDeckInput,
    deck_edge_station: f64,
) -> Vec<IneffectiveBlockPoint> {
    let Some(side) = side else {
        return vec![];
    };
    if side.derive_ineffective == Some(false) {
        return vec![];
    }
    if let Some(blocks) = side.ineffective_blocks.as_ref().filter(|b| !b.is_empty()) {
        return blocks.clone();
    }
    if let Some(profile) = side.embankment_profile.as_ref().filter(|p| p.is_valid()) {
        return profile
            .stations
            .iter()
            .zip(profile.elevations.iter())
            .map(|(&station, &elevation)| IneffectiveBlockPoint { station, elevation })
            .collect();
    }
    if side.abutment.is_some() || deck.is_valid() {
        return vec![IneffectiveBlockPoint {
            station: deck_edge_station,
            elevation: deck.interpolate_high(deck_edge_station),
        }];
    }
    vec![]
}

fn blocked_profile_from_side(side: Option<&RoadwayEmbankmentSide>) -> Option<EmbankmentPolyline> {
    let side = side?;
    if side.derive_blocked == Some(false) {
        return None;
    }
    side.embankment_profile
        .as_ref()
        .filter(|p| p.is_valid())
        .cloned()
}

fn face_override_blocks(
    face: Option<&BridgeIneffectiveFaceOverride>,
    side: Option<&str>,
) -> Option<Vec<IneffectiveBlockPoint>> {
    let face = face?;
    match side {
        Some("left") => face.left_blocks.clone().filter(|b| !b.is_empty()),
        Some("right") => face.right_blocks.clone().filter(|b| !b.is_empty()),
        _ => None,
    }
}

fn split_side_ineffective(
    left_pts: &[IneffectiveBlockPoint],
    right_pts: &[IneffectiveBlockPoint],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let (ls, le) = blocks_from_points(left_pts);
    let (rs, re) = blocks_from_points(right_pts);
    (ls, le, rs, re)
}

fn compose_ineffective_for_face_full(
    emb: &BridgeRoadwayEmbankment,
    face: &str,
    deck: &BridgeDeckInput,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let derive = emb.derive_ineffective != Some(false);
    if !derive {
        return (vec![], vec![], vec![], vec![]);
    }

    let override_face = match face {
        "upstream" => emb
            .ineffective_faces
            .as_ref()
            .and_then(|f| f.upstream.as_ref()),
        "downstream" => emb
            .ineffective_faces
            .as_ref()
            .and_then(|f| f.downstream.as_ref()),
        _ => None,
    };

    let s_left = deck.stations.first().copied().unwrap_or(0.0);
    let s_right = deck.stations.last().copied().unwrap_or(s_left);

    let left_pts = face_override_blocks(override_face, Some("left"))
        .unwrap_or_else(|| ineffective_points_from_side(emb.left.as_ref(), deck, s_left));
    let right_pts = face_override_blocks(override_face, Some("right"))
        .unwrap_or_else(|| ineffective_points_from_side(emb.right.as_ref(), deck, s_right));

    split_side_ineffective(&left_pts, &right_pts)
}

fn ensure_bridge_index<T: Clone>(vec: &mut Vec<T>, b_idx: usize, fill: T) {
    while vec.len() <= b_idx {
        vec.push(fill.clone());
    }
}

fn flat_deck_present(
    stations: &Option<Vec<Vec<f64>>>,
    low: &Option<Vec<Vec<f64>>>,
    high: &Option<Vec<Vec<f64>>>,
    b_idx: usize,
) -> bool {
    let Some(st) = stations.as_ref().and_then(|v| v.get(b_idx)) else {
        return false;
    };
    let Some(lo) = low.as_ref().and_then(|v| v.get(b_idx)) else {
        return false;
    };
    let Some(hi) = high.as_ref().and_then(|v| v.get(b_idx)) else {
        return false;
    };
    st.len() >= 2 && st.len() == lo.len() && st.len() == hi.len()
}

fn flat_ineffective_face_present(
    left_st: &Option<Vec<Vec<f64>>>,
    left_el: &Option<Vec<Vec<f64>>>,
    right_st: &Option<Vec<Vec<f64>>>,
    right_el: &Option<Vec<Vec<f64>>>,
    legacy_left_st: &Option<Vec<Vec<f64>>>,
    legacy_left_el: &Option<Vec<Vec<f64>>>,
    legacy_right_st: &Option<Vec<Vec<f64>>>,
    legacy_right_el: &Option<Vec<Vec<f64>>>,
    b_idx: usize,
) -> bool {
    let face_has = |lst: &Option<Vec<Vec<f64>>>,
                    lel: &Option<Vec<Vec<f64>>>,
                    rst: &Option<Vec<Vec<f64>>>,
                    rel: &Option<Vec<Vec<f64>>>| {
        let left = lst
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .filter(|b| !b.is_empty())
            .zip(
                lel.as_ref()
                    .and_then(|v| v.get(b_idx))
                    .filter(|b| !b.is_empty()),
            );
        let right = rst
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .filter(|b| !b.is_empty())
            .zip(
                rel.as_ref()
                    .and_then(|v| v.get(b_idx))
                    .filter(|b| !b.is_empty()),
            );
        left.is_some() || right.is_some()
    };
    face_has(left_st, left_el, right_st, right_el)
        || face_has(
            legacy_left_st,
            legacy_left_el,
            legacy_right_st,
            legacy_right_el,
        )
}

fn set_nested_bridge_blocks(target: &mut Option<Vec<Vec<f64>>>, b_idx: usize, values: Vec<f64>) {
    if values.is_empty() {
        return;
    }
    let vec = target.get_or_insert_with(Vec::new);
    ensure_bridge_index(vec, b_idx, vec![]);
    vec[b_idx] = values;
}

fn compose_abutment_side(
    side: Option<&RoadwayEmbankmentSide>,
    deck_edge: f64,
    width_out: &mut Option<Vec<f64>>,
    station_out: &mut Option<Vec<f64>>,
    top_out: &mut Option<Vec<f64>>,
    profile_st_out: &mut Option<Vec<Vec<f64>>>,
    profile_el_out: &mut Option<Vec<Vec<f64>>>,
    b_idx: usize,
) {
    let Some(side) = side else {
        return;
    };
    let Some(abut) = side.abutment.as_ref() else {
        return;
    };
    if abut.width <= 1e-9 {
        return;
    }

    let widths = width_out.get_or_insert_with(Vec::new);
    ensure_bridge_index(widths, b_idx, 0.0);
    if widths[b_idx] <= 1e-9 {
        widths[b_idx] = abut.width;
    }

    let outer = abut.outer_station.unwrap_or(deck_edge);
    let stations = station_out.get_or_insert_with(Vec::new);
    ensure_bridge_index(stations, b_idx, 0.0);
    if stations[b_idx].abs() < 1e-12 {
        stations[b_idx] = outer;
    }

    if let Some(profile) = abut.top_profile.as_ref().filter(|p| p.is_valid()) {
        let st = profile_st_out.get_or_insert_with(Vec::new);
        let el = profile_el_out.get_or_insert_with(Vec::new);
        ensure_bridge_index(st, b_idx, vec![]);
        ensure_bridge_index(el, b_idx, vec![]);
        if st[b_idx].is_empty() {
            st[b_idx] = profile.stations.clone();
            el[b_idx] = profile.elevations.clone();
        }
    } else if let Some(top) = abut.top_elevation {
        let tops = top_out.get_or_insert_with(Vec::new);
        ensure_bridge_index(tops, b_idx, 0.0);
        if tops[b_idx].abs() < 1e-12 {
            tops[b_idx] = top;
        }
    }
}

/// Opening-frame blocked top profiles composed from embankment grade lines (per bridge, per side).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ComposedEmbankmentBlocked {
    pub left: Option<EmbankmentPolyline>,
    pub right: Option<EmbankmentPolyline>,
}

pub fn compose_one_bridge(
    emb: &BridgeRoadwayEmbankment,
    b_idx: usize,
    deck_stations: &mut Option<Vec<Vec<f64>>>,
    deck_low: &mut Option<Vec<Vec<f64>>>,
    deck_high: &mut Option<Vec<Vec<f64>>>,
    low_chords: &mut Option<Vec<f64>>,
    high_chords: &mut Option<Vec<f64>>,
    abutment_left_widths: &mut Option<Vec<f64>>,
    abutment_right_widths: &mut Option<Vec<f64>>,
    abutment_left_stations: &mut Option<Vec<f64>>,
    abutment_right_stations: &mut Option<Vec<f64>>,
    abutment_left_top_elevations: &mut Option<Vec<f64>>,
    abutment_right_top_elevations: &mut Option<Vec<f64>>,
    abutment_left_top_profile_stations: &mut Option<Vec<Vec<f64>>>,
    abutment_left_top_profile_elevations: &mut Option<Vec<Vec<f64>>>,
    abutment_right_top_profile_stations: &mut Option<Vec<Vec<f64>>>,
    abutment_right_top_profile_elevations: &mut Option<Vec<Vec<f64>>>,
    ineffective_left_stations_upstream: &mut Option<Vec<Vec<f64>>>,
    ineffective_left_elevations_upstream: &mut Option<Vec<Vec<f64>>>,
    ineffective_right_stations_upstream: &mut Option<Vec<Vec<f64>>>,
    ineffective_right_elevations_upstream: &mut Option<Vec<Vec<f64>>>,
    ineffective_left_stations_downstream: &mut Option<Vec<Vec<f64>>>,
    ineffective_left_elevations_downstream: &mut Option<Vec<Vec<f64>>>,
    ineffective_right_stations_downstream: &mut Option<Vec<Vec<f64>>>,
    ineffective_right_elevations_downstream: &mut Option<Vec<Vec<f64>>>,
    ineffective_left_stations: &mut Option<Vec<Vec<f64>>>,
    ineffective_left_elevations: &mut Option<Vec<Vec<f64>>>,
    ineffective_right_stations: &mut Option<Vec<Vec<f64>>>,
    ineffective_right_elevations: &mut Option<Vec<Vec<f64>>>,
) -> ComposedEmbankmentBlocked {
    let mut blocked = ComposedEmbankmentBlocked::default();
    if !emb.deck.is_valid() {
        return blocked;
    }

    let deck = &emb.deck;
    let s_left = deck.stations[0];
    let s_right = deck.stations[deck.stations.len() - 1];

    if !flat_deck_present(deck_stations, deck_low, deck_high, b_idx) {
        let st = deck_stations.get_or_insert_with(Vec::new);
        let lo = deck_low.get_or_insert_with(Vec::new);
        let hi = deck_high.get_or_insert_with(Vec::new);
        ensure_bridge_index(st, b_idx, vec![]);
        ensure_bridge_index(lo, b_idx, vec![]);
        ensure_bridge_index(hi, b_idx, vec![]);
        st[b_idx] = deck.stations.clone();
        lo[b_idx] = deck.low_elevations.clone();
        hi[b_idx] = deck.high_elevations.clone();
    }

    let lows = low_chords.get_or_insert_with(Vec::new);
    ensure_bridge_index(lows, b_idx, 0.0);
    if lows[b_idx].abs() < 1e-12 {
        lows[b_idx] = deck.low_min();
    }
    let highs = high_chords.get_or_insert_with(Vec::new);
    ensure_bridge_index(highs, b_idx, 0.0);
    if highs[b_idx].abs() < 1e-12 {
        highs[b_idx] = deck.high_max();
    }

    compose_abutment_side(
        emb.left.as_ref(),
        s_left,
        abutment_left_widths,
        abutment_left_stations,
        abutment_left_top_elevations,
        abutment_left_top_profile_stations,
        abutment_left_top_profile_elevations,
        b_idx,
    );
    compose_abutment_side(
        emb.right.as_ref(),
        s_right,
        abutment_right_widths,
        abutment_right_stations,
        abutment_right_top_elevations,
        abutment_right_top_profile_stations,
        abutment_right_top_profile_elevations,
        b_idx,
    );

    if !flat_ineffective_face_present(
        ineffective_left_stations_upstream,
        ineffective_left_elevations_upstream,
        ineffective_right_stations_upstream,
        ineffective_right_elevations_upstream,
        ineffective_left_stations,
        ineffective_left_elevations,
        ineffective_right_stations,
        ineffective_right_elevations,
        b_idx,
    ) {
        let (ls, le, rs, re) = compose_ineffective_for_face_full(emb, "upstream", deck);
        set_nested_bridge_blocks(ineffective_left_stations_upstream, b_idx, ls);
        set_nested_bridge_blocks(ineffective_left_elevations_upstream, b_idx, le);
        set_nested_bridge_blocks(ineffective_right_stations_upstream, b_idx, rs);
        set_nested_bridge_blocks(ineffective_right_elevations_upstream, b_idx, re);
    }

    if !flat_ineffective_face_present(
        ineffective_left_stations_downstream,
        ineffective_left_elevations_downstream,
        ineffective_right_stations_downstream,
        ineffective_right_elevations_downstream,
        ineffective_left_stations,
        ineffective_left_elevations,
        ineffective_right_stations,
        ineffective_right_elevations,
        b_idx,
    ) {
        let (ls, le, rs, re) = compose_ineffective_for_face_full(emb, "downstream", deck);
        set_nested_bridge_blocks(ineffective_left_stations_downstream, b_idx, ls);
        set_nested_bridge_blocks(ineffective_left_elevations_downstream, b_idx, le);
        set_nested_bridge_blocks(ineffective_right_stations_downstream, b_idx, rs);
        set_nested_bridge_blocks(ineffective_right_elevations_downstream, b_idx, re);
    }

    blocked.left = blocked_profile_from_side(emb.left.as_ref());
    blocked.right = blocked_profile_from_side(emb.right.as_ref());
    blocked
}

pub fn composed_embankment_blocked_for(
    blocked: &Option<Vec<Option<ComposedEmbankmentBlocked>>>,
    b_idx: usize,
) -> Option<&ComposedEmbankmentBlocked> {
    blocked
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .and_then(|o| o.as_ref())
}

/// Opening-frame blocked profiles for bridge `b_idx` after steady compose (if any).
pub fn steady_composed_embankment_blocked(
    inputs: &SteadyInputs,
    b_idx: usize,
) -> Option<ComposedEmbankmentBlocked> {
    composed_embankment_blocked_for(&inputs.bridge_composed_embankment_blocked, b_idx).cloned()
}

/// Opening-frame blocked profiles after rating-curve compose (if any).
pub fn rating_composed_embankment_blocked(
    params: &BridgeSolveParams,
) -> Option<ComposedEmbankmentBlocked> {
    params.composed_embankment_blocked.clone()
}

pub fn needs_roadway_compose_steady(inputs: &SteadyInputs) -> bool {
    inputs
        .bridge_roadway_embankments
        .as_ref()
        .is_some_and(|v| v.iter().any(Option::is_some))
}

pub fn apply_roadway_embankment_compose_steady(inputs: &mut SteadyInputs) {
    let Some(embankments) = inputs.bridge_roadway_embankments.clone() else {
        return;
    };
    let blocked_out = inputs
        .bridge_composed_embankment_blocked
        .get_or_insert_with(Vec::new);
    for (b_idx, emb) in embankments.iter().enumerate() {
        let Some(emb) = emb else { continue };
        let composed = compose_one_bridge(
            emb,
            b_idx,
            &mut inputs.bridge_deck_stations,
            &mut inputs.bridge_deck_low_elevations,
            &mut inputs.bridge_deck_high_elevations,
            &mut inputs.bridge_low_chords,
            &mut inputs.bridge_high_chords,
            &mut inputs.bridge_abutment_left_widths,
            &mut inputs.bridge_abutment_right_widths,
            &mut inputs.bridge_abutment_left_stations,
            &mut inputs.bridge_abutment_right_stations,
            &mut inputs.bridge_abutment_left_top_elevations,
            &mut inputs.bridge_abutment_right_top_elevations,
            &mut inputs.bridge_abutment_left_top_profile_stations,
            &mut inputs.bridge_abutment_left_top_profile_elevations,
            &mut inputs.bridge_abutment_right_top_profile_stations,
            &mut inputs.bridge_abutment_right_top_profile_elevations,
            &mut inputs.bridge_ineffective_left_stations_upstream,
            &mut inputs.bridge_ineffective_left_elevations_upstream,
            &mut inputs.bridge_ineffective_right_stations_upstream,
            &mut inputs.bridge_ineffective_right_elevations_upstream,
            &mut inputs.bridge_ineffective_left_stations_downstream,
            &mut inputs.bridge_ineffective_left_elevations_downstream,
            &mut inputs.bridge_ineffective_right_stations_downstream,
            &mut inputs.bridge_ineffective_right_elevations_downstream,
            &mut inputs.bridge_ineffective_left_stations,
            &mut inputs.bridge_ineffective_left_elevations,
            &mut inputs.bridge_ineffective_right_stations,
            &mut inputs.bridge_ineffective_right_elevations,
        );
        ensure_bridge_index(blocked_out, b_idx, None);
        if composed.left.is_some() || composed.right.is_some() {
            blocked_out[b_idx] = Some(composed);
        }
    }
}

pub fn composed_steady_inputs(inputs: &SteadyInputs) -> SteadyInputs {
    if !needs_roadway_compose_steady(inputs) {
        return inputs.clone();
    }
    let mut c = inputs.clone();
    apply_roadway_embankment_compose_steady(&mut c);
    c
}

fn params_ineffective_nested(
    stations: &Option<Vec<f64>>,
    scalar: Option<f64>,
) -> Option<Vec<Vec<f64>>> {
    if let Some(v) = stations.as_ref().filter(|v| !v.is_empty()) {
        return Some(vec![v.clone()]);
    }
    scalar.map(|s| vec![vec![s]])
}

pub fn apply_roadway_embankment_compose_params(params: &mut BridgeSolveParams) {
    let Some(emb) = params.roadway_embankment.clone() else {
        return;
    };
    let mut deck_stations = params.deck_stations.as_ref().map(|s| vec![s.clone()]);
    let mut deck_low = params.deck_low_elevations.as_ref().map(|s| vec![s.clone()]);
    let mut deck_high = params
        .deck_high_elevations
        .as_ref()
        .map(|s| vec![s.clone()]);
    let mut low_chords = Some(vec![params.low_chord]);
    let mut high_chords = Some(vec![params.high_chord]);
    let mut left_widths = params.abutment_left_width.map(|w| vec![w]);
    let mut right_widths = params.abutment_right_width.map(|w| vec![w]);
    let mut left_stations = params.abutment_left_station.map(|s| vec![s]);
    let mut right_stations = params.abutment_right_station.map(|s| vec![s]);
    let mut left_top = params.abutment_left_top_elevation.map(|e| vec![e]);
    let mut right_top = params.abutment_right_top_elevation.map(|e| vec![e]);
    let mut left_prof_st = params
        .abutment_left_top_profile_stations
        .as_ref()
        .map(|s| vec![s.clone()]);
    let mut left_prof_el = params
        .abutment_left_top_profile_elevations
        .as_ref()
        .map(|s| vec![s.clone()]);
    let mut right_prof_st = params
        .abutment_right_top_profile_stations
        .as_ref()
        .map(|s| vec![s.clone()]);
    let mut right_prof_el = params
        .abutment_right_top_profile_elevations
        .as_ref()
        .map(|s| vec![s.clone()]);

    let mut legacy_left_st = params_ineffective_nested(
        &params.ineffective_left_stations,
        params.ineffective_left_station,
    );
    let mut legacy_left_el = params_ineffective_nested(
        &params.ineffective_left_elevations,
        params.ineffective_left_elevation,
    );
    let mut legacy_right_st = params_ineffective_nested(
        &params.ineffective_right_stations,
        params.ineffective_right_station,
    );
    let mut legacy_right_el = params_ineffective_nested(
        &params.ineffective_right_elevations,
        params.ineffective_right_elevation,
    );
    let mut us_left_st = params_ineffective_nested(
        &params.ineffective_left_stations_upstream,
        params.ineffective_left_station_upstream,
    );
    let mut us_left_el = params_ineffective_nested(
        &params.ineffective_left_elevations_upstream,
        params.ineffective_left_elevation_upstream,
    );
    let mut us_right_st = params_ineffective_nested(
        &params.ineffective_right_stations_upstream,
        params.ineffective_right_station_upstream,
    );
    let mut us_right_el = params_ineffective_nested(
        &params.ineffective_right_elevations_upstream,
        params.ineffective_right_elevation_upstream,
    );
    let mut ds_left_st = params_ineffective_nested(
        &params.ineffective_left_stations_downstream,
        params.ineffective_left_station_downstream,
    );
    let mut ds_left_el = params_ineffective_nested(
        &params.ineffective_left_elevations_downstream,
        params.ineffective_left_elevation_downstream,
    );
    let mut ds_right_st = params_ineffective_nested(
        &params.ineffective_right_stations_downstream,
        params.ineffective_right_station_downstream,
    );
    let mut ds_right_el = params_ineffective_nested(
        &params.ineffective_right_elevations_downstream,
        params.ineffective_right_elevation_downstream,
    );

    let composed = compose_one_bridge(
        &emb,
        0,
        &mut deck_stations,
        &mut deck_low,
        &mut deck_high,
        &mut low_chords,
        &mut high_chords,
        &mut left_widths,
        &mut right_widths,
        &mut left_stations,
        &mut right_stations,
        &mut left_top,
        &mut right_top,
        &mut left_prof_st,
        &mut left_prof_el,
        &mut right_prof_st,
        &mut right_prof_el,
        &mut us_left_st,
        &mut us_left_el,
        &mut us_right_st,
        &mut us_right_el,
        &mut ds_left_st,
        &mut ds_left_el,
        &mut ds_right_st,
        &mut ds_right_el,
        &mut legacy_left_st,
        &mut legacy_left_el,
        &mut legacy_right_st,
        &mut legacy_right_el,
    );

    if let Some(st) = deck_stations.and_then(|v| v.into_iter().next()) {
        params.deck_stations = Some(st);
    }
    if let Some(lo) = deck_low.and_then(|v| v.into_iter().next()) {
        params.deck_low_elevations = Some(lo);
    }
    if let Some(hi) = deck_high.and_then(|v| v.into_iter().next()) {
        params.deck_high_elevations = Some(hi);
    }
    if let Some(lc) = low_chords.and_then(|v| v.into_iter().next()) {
        params.low_chord = lc;
    }
    if let Some(hc) = high_chords.and_then(|v| v.into_iter().next()) {
        params.high_chord = hc;
    }
    if let Some(w) = left_widths.and_then(|v| v.into_iter().next()) {
        params.abutment_left_width = Some(w);
    }
    if let Some(w) = right_widths.and_then(|v| v.into_iter().next()) {
        params.abutment_right_width = Some(w);
    }
    if let Some(s) = left_stations.and_then(|v| v.into_iter().next()) {
        params.abutment_left_station = Some(s);
    }
    if let Some(s) = right_stations.and_then(|v| v.into_iter().next()) {
        params.abutment_right_station = Some(s);
    }
    if let Some(e) = left_top.and_then(|v| v.into_iter().next()) {
        params.abutment_left_top_elevation = Some(e);
    }
    if let Some(e) = right_top.and_then(|v| v.into_iter().next()) {
        params.abutment_right_top_elevation = Some(e);
    }
    if let Some(st) = left_prof_st.and_then(|v| v.into_iter().next()) {
        params.abutment_left_top_profile_stations = Some(st);
    }
    if let Some(el) = left_prof_el.and_then(|v| v.into_iter().next()) {
        params.abutment_left_top_profile_elevations = Some(el);
    }
    if let Some(st) = right_prof_st.and_then(|v| v.into_iter().next()) {
        params.abutment_right_top_profile_stations = Some(st);
    }
    if let Some(el) = right_prof_el.and_then(|v| v.into_iter().next()) {
        params.abutment_right_top_profile_elevations = Some(el);
    }

    apply_face_to_params(
        params,
        &us_left_st,
        &us_left_el,
        &us_right_st,
        &us_right_el,
        true,
    );
    apply_face_to_params(
        params,
        &ds_left_st,
        &ds_left_el,
        &ds_right_st,
        &ds_right_el,
        false,
    );

    if composed.left.is_some() || composed.right.is_some() {
        params.composed_embankment_blocked = Some(composed);
    }
}

fn apply_face_to_params(
    params: &mut BridgeSolveParams,
    left_st: &Option<Vec<Vec<f64>>>,
    left_el: &Option<Vec<Vec<f64>>>,
    right_st: &Option<Vec<Vec<f64>>>,
    right_el: &Option<Vec<Vec<f64>>>,
    upstream: bool,
) {
    let b = 0;
    let left_s = left_st.as_ref().and_then(|v| v.get(b)).cloned();
    let left_e = left_el.as_ref().and_then(|v| v.get(b)).cloned();
    let right_s = right_st.as_ref().and_then(|v| v.get(b)).cloned();
    let right_e = right_el.as_ref().and_then(|v| v.get(b)).cloned();
    if upstream {
        if let (Some(ls), Some(le)) = (left_s, left_e) {
            params.ineffective_left_stations_upstream = nested_to_option(&ls);
            params.ineffective_left_elevations_upstream = nested_to_option(&le);
        }
        if let (Some(rs), Some(re)) = (right_s, right_e) {
            params.ineffective_right_stations_upstream = nested_to_option(&rs);
            params.ineffective_right_elevations_upstream = nested_to_option(&re);
        }
    } else if let (Some(ls), Some(le)) = (left_s, left_e) {
        params.ineffective_left_stations_downstream = nested_to_option(&ls);
        params.ineffective_left_elevations_downstream = nested_to_option(&le);
        if let (Some(rs), Some(re)) = (right_s, right_e) {
            params.ineffective_right_stations_downstream = nested_to_option(&rs);
            params.ineffective_right_elevations_downstream = nested_to_option(&re);
        }
    }
}

fn nested_to_option(blocks: &[f64]) -> Option<Vec<f64>> {
    if blocks.is_empty() {
        None
    } else {
        Some(blocks.to_vec())
    }
}

/// Remap opening-frame embankment blocked polylines onto reach lateral `x` and merge into a cut.
pub fn merge_embankment_blocked_into_section(
    section: &mut crate::geometry::CrossSection,
    left: Option<&EmbankmentPolyline>,
    right: Option<&EmbankmentPolyline>,
    opening_origin: Option<f64>,
) {
    let origin = opening_origin.unwrap_or(0.0);
    let mut blocks = section.blocked_obstructions.take().unwrap_or_default();

    for profile in [left, right].into_iter().flatten() {
        if !profile.is_valid() {
            continue;
        }
        let stations: Vec<f64> = profile.stations.iter().map(|s| s + origin).collect();
        let obs = BlockedObstruction {
            stations,
            elevations: profile.elevations.clone(),
        };
        if obs.is_valid() {
            blocks.push(obs);
        }
    }

    if !blocks.is_empty() {
        section.blocked_obstructions = Some(blocks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_embankment() -> BridgeRoadwayEmbankment {
        BridgeRoadwayEmbankment {
            deck: BridgeDeckInput {
                stations: vec![0.0, 10.0],
                low_elevations: vec![5.0, 5.0],
                high_elevations: vec![7.0, 7.0],
            },
            left: Some(RoadwayEmbankmentSide {
                embankment_profile: Some(EmbankmentPolyline {
                    stations: vec![-5.0, 0.0],
                    elevations: vec![6.5, 7.0],
                }),
                abutment: Some(RoadwayAbutmentInput {
                    outer_station: None,
                    width: 1.0,
                    top_elevation: Some(0.0),
                    top_profile: None,
                }),
                ..Default::default()
            }),
            right: Some(RoadwayEmbankmentSide {
                embankment_profile: Some(EmbankmentPolyline {
                    stations: vec![10.0, 15.0],
                    elevations: vec![7.0, 6.5],
                }),
                abutment: Some(RoadwayAbutmentInput {
                    outer_station: None,
                    width: 4.0,
                    top_elevation: Some(2.5),
                    top_profile: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn embankment_profile_drives_ineffective_and_blocked() {
        let emb = sample_embankment();
        let mut deck_st = None;
        let mut deck_lo = None;
        let mut deck_hi = None;
        let mut low = None;
        let mut high = None;
        let mut alw = None;
        let mut arw = None;
        let mut als = None;
        let mut ars = None;
        let mut alt = None;
        let mut art = None;
        let mut alps = None;
        let mut alpe = None;
        let mut arps = None;
        let mut arpe = None;
        let mut us_ls = None;
        let mut us_le = None;
        let mut us_rs = None;
        let mut us_re = None;
        let mut ds_ls = None;
        let mut ds_le = None;
        let mut ds_rs = None;
        let mut ds_re = None;
        let mut leg_ls = None;
        let mut leg_le = None;
        let mut leg_rs = None;
        let mut leg_re = None;

        let blocked = compose_one_bridge(
            &emb,
            0,
            &mut deck_st,
            &mut deck_lo,
            &mut deck_hi,
            &mut low,
            &mut high,
            &mut alw,
            &mut arw,
            &mut als,
            &mut ars,
            &mut alt,
            &mut art,
            &mut alps,
            &mut alpe,
            &mut arps,
            &mut arpe,
            &mut us_ls,
            &mut us_le,
            &mut us_rs,
            &mut us_re,
            &mut ds_ls,
            &mut ds_le,
            &mut ds_rs,
            &mut ds_re,
            &mut leg_ls,
            &mut leg_le,
            &mut leg_rs,
            &mut leg_re,
        );

        assert_eq!(deck_st.unwrap()[0], vec![0.0, 10.0]);
        assert!((low.unwrap()[0] - 5.0).abs() < 1e-9);
        assert!((high.unwrap()[0] - 7.0).abs() < 1e-9);
        assert!((alw.unwrap()[0] - 1.0).abs() < 1e-9);
        assert!((arw.unwrap()[0] - 4.0).abs() < 1e-9);

        let us_left_st = us_ls.unwrap()[0].clone();
        let us_left_el = us_le.unwrap()[0].clone();
        assert_eq!(us_left_st, vec![-5.0, 0.0]);
        assert_eq!(us_left_el, vec![6.5, 7.0]);

        let us_right_st = us_rs.unwrap()[0].clone();
        let us_right_el = us_re.unwrap()[0].clone();
        assert_eq!(us_right_st, vec![10.0, 15.0]);
        assert_eq!(us_right_el, vec![7.0, 6.5]);

        assert!(blocked.left.is_some());
        assert!(blocked.right.is_some());
    }

    #[test]
    fn steady_compose_round_trip_abutment_case() {
        let mut inputs = SteadyInputs {
            bridge_stations: Some(vec![500.0]),
            bridge_low_flow_methods: Some(vec![4]),
            bridge_lengths: Some(vec![50.0]),
            bridge_roadway_embankments: Some(vec![Some(sample_embankment())]),
            ..Default::default()
        };
        apply_roadway_embankment_compose_steady(&mut inputs);

        assert_eq!(
            inputs.bridge_deck_stations.as_ref().unwrap()[0],
            vec![0.0, 10.0]
        );
        assert!((inputs.bridge_abutment_left_widths.as_ref().unwrap()[0] - 1.0).abs() < 1e-9);
        assert!((inputs.bridge_abutment_right_widths.as_ref().unwrap()[0] - 4.0).abs() < 1e-9);
        assert!(
            (inputs
                .bridge_abutment_right_top_elevations
                .as_ref()
                .unwrap()[0]
                - 2.5)
                .abs()
                < 1e-9
        );
        let blocked = inputs.bridge_composed_embankment_blocked.as_ref().unwrap()[0]
            .as_ref()
            .unwrap();
        assert!(blocked.left.as_ref().unwrap().is_valid());
    }

    #[test]
    fn flat_deck_wins_over_unified() {
        let emb = sample_embankment();
        let mut deck_st = Some(vec![vec![1.0, 2.0, 3.0]]);
        let mut deck_lo = Some(vec![vec![4.0, 4.0, 4.0]]);
        let mut deck_hi = Some(vec![vec![8.0, 8.0, 8.0]]);
        let mut low = None;
        let mut high = None;
        let mut alw = None;
        let mut arw = None;
        let mut als = None;
        let mut ars = None;
        let mut alt = None;
        let mut art = None;
        let mut alps = None;
        let mut alpe = None;
        let mut arps = None;
        let mut arpe = None;
        let mut us_ls = None;
        let mut us_le = None;
        let mut us_rs = None;
        let mut us_re = None;
        let mut ds_ls = None;
        let mut ds_le = None;
        let mut ds_rs = None;
        let mut ds_re = None;
        let mut leg_ls = None;
        let mut leg_le = None;
        let mut leg_rs = None;
        let mut leg_re = None;

        compose_one_bridge(
            &emb,
            0,
            &mut deck_st,
            &mut deck_lo,
            &mut deck_hi,
            &mut low,
            &mut high,
            &mut alw,
            &mut arw,
            &mut als,
            &mut ars,
            &mut alt,
            &mut art,
            &mut alps,
            &mut alpe,
            &mut arps,
            &mut arpe,
            &mut us_ls,
            &mut us_le,
            &mut us_rs,
            &mut us_re,
            &mut ds_ls,
            &mut ds_le,
            &mut ds_rs,
            &mut ds_re,
            &mut leg_ls,
            &mut leg_le,
            &mut leg_rs,
            &mut leg_re,
        );

        assert_eq!(deck_st.unwrap()[0], vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn merge_blocked_remaps_opening_origin() {
        use crate::geometry::CrossSection;
        use crate::utils::UnitSystem;

        let mut xs = CrossSection {
            station: 0.0,
            x: vec![80.0, 100.0, 120.0],
            y: vec![0.0, 0.0, 0.0],
            n_stations: vec![80.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            coeff_contraction: None,
            coeff_expansion: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let profile = EmbankmentPolyline {
            stations: vec![0.0, 5.0],
            elevations: vec![6.0, 6.5],
        };
        merge_embankment_blocked_into_section(&mut xs, Some(&profile), None, Some(95.0));
        let blocks = xs.blocked_obstructions.unwrap();
        assert_eq!(blocks.len(), 1);
        assert!((blocks[0].stations[0] - 95.0).abs() < 1e-9);
        assert!((blocks[0].stations[1] - 100.0).abs() < 1e-9);
    }

    #[test]
    fn typical_roadway_fill_reduces_bu_active_area_without_manual_polylines() {
        use crate::geometry::CrossSection;
        use crate::solvers::bridge_interior::{
            interior_from_steady, resolve_bridge_face_solve_geometry,
        };
        use crate::solvers::steady::{
            bridge_ineffective_downstream_for, bridge_ineffective_upstream_for, SteadyInputs,
        };
        use crate::utils::UnitSystem;

        fn face(station: f64) -> CrossSection {
            CrossSection {
                station,
                x: vec![0.0, 0.0, 30.0, 30.0],
                y: vec![8.0, 0.0, 0.0, 8.0],
                n_stations: vec![0.0],
                n_values: vec![0.03],
                unit_system: UnitSystem::Metric,
                is_overbank: None,
                coeff_contraction: None,
                coeff_expansion: None,
                blocked_obstructions: None,
                ineffective_flow_areas: None,
                guide_banks: None,
            }
        }

        let bu = face(52.0);
        assert!(bu.blocked_obstructions.is_none());

        let inputs = SteadyInputs {
            cross_sections: vec![face(200.0), face(100.0), face(0.0)],
            flow_rate: 15.0,
            bridge_stations: Some(vec![50.0]),
            bridge_lengths: Some(vec![4.0]),
            bridge_low_chords: Some(vec![5.0]),
            bridge_high_chords: Some(vec![6.5]),
            bridge_low_flow_methods: Some(vec![3]),
            bridge_opening_reach_station_origins: Some(vec![10.0]),
            bridge_upstream_cross_sections: Some(vec![bu]),
            bridge_downstream_cross_sections: Some(vec![face(48.0)]),
            bridge_roadway_embankments: Some(vec![Some(sample_embankment())]),
            ..Default::default()
        };
        let composed = composed_steady_inputs(&inputs);
        let blocked = composed
            .bridge_composed_embankment_blocked
            .as_ref()
            .and_then(|v| v.first())
            .and_then(|o| o.as_ref())
            .expect("blocked profiles");

        let interior = interior_from_steady(&composed, 0);
        let reach = face(100.0);
        let table = reach.generate_lookup_table(50);
        let geo = resolve_bridge_face_solve_geometry(
            crate::solvers::bridge_interior::BridgeFaceSolveParams {
                interior: &interior,
                reach_xs_up: Some(&reach),
                reach_xs_down: Some(&reach),
                reach_table_up: &table,
                reach_table_down: &table,
                raw_units: UnitSystem::Metric,
                num_slices: 50,
                ineffective_up: bridge_ineffective_upstream_for(&composed, 0),
                ineffective_down: bridge_ineffective_downstream_for(&composed, 0),
                interval_length_m: 4.0,
                embankment_blocked: Some(blocked),
                ..crate::solvers::bridge_interior::BridgeFaceSolveParams::new(
                    &interior, &table, &table,
                )
            },
        );

        assert!(
            geo.sections
                .xs_up
                .as_ref()
                .and_then(|xs| xs.blocked_obstructions.as_ref())
                .is_some_and(|b| !b.is_empty()),
            "BU should gain blocked tops from embankment profile"
        );
        let tw = 2.5;
        assert!(
            geo.table_up.interpolate(tw).active_area < table.interpolate(tw).active_area,
            "fill should reduce conveyance at tailwater"
        );
    }

    #[test]
    fn rating_curve_params_compose() {
        use crate::solvers::bridge::BridgeSolveParams;
        use crate::utils::UnitSystem;

        let mut params = BridgeSolveParams {
            low_chord: 0.0,
            high_chord: 0.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 2.5,
            units: UnitSystem::Metric,
            low_flow_method: 4,
            roadway_embankment: Some(sample_embankment()),
            ..Default::default()
        };
        apply_roadway_embankment_compose_params(&mut params);

        assert_eq!(params.deck_stations.as_deref(), Some(&[0.0, 10.0][..]));
        assert!((params.low_chord - 5.0).abs() < 1e-9);
        assert!((params.abutment_left_width.unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(
            params.ineffective_left_stations_upstream.as_deref(),
            Some(&[-5.0, 0.0][..])
        );
        assert!(params.composed_embankment_blocked.is_some());
    }

    #[test]
    fn flat_ineffective_wins_over_embankment_profile() {
        let emb = sample_embankment();
        let mut us_ls = Some(vec![vec![99.0]]);
        let mut us_le = Some(vec![vec![4.0]]);
        let mut us_rs = None;
        let mut us_re = None;
        let mut ds_ls = None;
        let mut ds_le = None;
        let mut ds_rs = None;
        let mut ds_re = None;
        let mut leg_ls = None;
        let mut leg_le = None;
        let mut leg_rs = None;
        let mut leg_re = None;

        compose_one_bridge(
            &emb,
            0,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut us_ls,
            &mut us_le,
            &mut us_rs,
            &mut us_re,
            &mut ds_ls,
            &mut ds_le,
            &mut ds_rs,
            &mut ds_re,
            &mut leg_ls,
            &mut leg_le,
            &mut leg_rs,
            &mut leg_re,
        );

        assert_eq!(us_ls.unwrap()[0], vec![99.0]);
        assert_eq!(us_le.unwrap()[0], vec![4.0]);
    }

    #[test]
    fn derive_ineffective_false_skips_profile_blocks() {
        let mut emb = sample_embankment();
        emb.derive_ineffective = Some(false);

        let mut us_ls = None;
        let mut us_le = None;
        let mut us_rs = None;
        let mut us_re = None;
        let mut ds_ls = None;
        let mut ds_le = None;
        let mut ds_rs = None;
        let mut ds_re = None;
        let mut leg_ls = None;
        let mut leg_le = None;
        let mut leg_rs = None;
        let mut leg_re = None;

        let blocked = compose_one_bridge(
            &emb,
            0,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut us_ls,
            &mut us_le,
            &mut us_rs,
            &mut us_re,
            &mut ds_ls,
            &mut ds_le,
            &mut ds_rs,
            &mut ds_re,
            &mut leg_ls,
            &mut leg_le,
            &mut leg_rs,
            &mut leg_re,
        );

        assert!(us_ls.is_none());
        assert!(blocked.left.is_some(), "blocked still derived from profile");
    }

    #[test]
    fn derive_blocked_false_omits_blocked_profiles() {
        let mut emb = sample_embankment();
        emb.left.as_mut().unwrap().derive_blocked = Some(false);

        let mut us_ls = None;
        let mut us_le = None;
        let mut us_rs = None;
        let mut us_re = None;
        let mut ds_ls = None;
        let mut ds_le = None;
        let mut ds_rs = None;
        let mut ds_re = None;
        let mut leg_ls = None;
        let mut leg_le = None;
        let mut leg_rs = None;
        let mut leg_re = None;

        let blocked = compose_one_bridge(
            &emb,
            0,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut us_ls,
            &mut us_le,
            &mut us_rs,
            &mut us_re,
            &mut ds_ls,
            &mut ds_le,
            &mut ds_rs,
            &mut ds_re,
            &mut leg_ls,
            &mut leg_le,
            &mut leg_rs,
            &mut leg_re,
        );

        assert!(blocked.left.is_none());
        assert!(blocked.right.is_some());
    }

    #[test]
    fn ineffective_faces_upstream_override() {
        let emb = BridgeRoadwayEmbankment {
            ineffective_faces: Some(BridgeIneffectiveFacesOverride {
                upstream: Some(BridgeIneffectiveFaceOverride {
                    left_blocks: Some(vec![IneffectiveBlockPoint {
                        station: 1.5,
                        elevation: 3.0,
                    }]),
                    right_blocks: Some(vec![IneffectiveBlockPoint {
                        station: 8.5,
                        elevation: 3.5,
                    }]),
                }),
                downstream: None,
            }),
            ..sample_embankment()
        };

        let mut us_ls = None;
        let mut us_le = None;
        let mut us_rs = None;
        let mut us_re = None;
        let mut ds_ls = None;
        let mut ds_le = None;
        let mut ds_rs = None;
        let mut ds_re = None;
        let mut leg_ls = None;
        let mut leg_le = None;
        let mut leg_rs = None;
        let mut leg_re = None;

        compose_one_bridge(
            &emb,
            0,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut None,
            &mut us_ls,
            &mut us_le,
            &mut us_rs,
            &mut us_re,
            &mut ds_ls,
            &mut ds_le,
            &mut ds_rs,
            &mut ds_re,
            &mut leg_ls,
            &mut leg_le,
            &mut leg_rs,
            &mut leg_re,
        );

        assert_eq!(us_ls.unwrap()[0], vec![1.5]);
        assert_eq!(us_le.unwrap()[0], vec![3.0]);
        assert_eq!(us_rs.unwrap()[0], vec![8.5]);
        assert_eq!(
            ds_ls.unwrap()[0],
            vec![-5.0, 0.0],
            "downstream uses profile"
        );
    }

    #[test]
    fn rating_curve_unified_matches_decomposed_flat_params() {
        use crate::solvers::bridge::{
            compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams,
        };
        use crate::utils::UnitSystem;

        let emb = sample_embankment();
        let mut unified = BridgeSolveParams {
            q: 15.0,
            low_chord: 0.0,
            high_chord: 0.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 2.5,
            units: UnitSystem::Metric,
            low_flow_method: 4,
            channel_width: 30.0,
            manning_n: 0.03,
            opening_reach_station_origin: Some(10.0),
            roadway_embankment: Some(emb),
            ..Default::default()
        };
        apply_roadway_embankment_compose_params(&mut unified);
        let blocked = unified.composed_embankment_blocked.clone();
        let roadway_embankment = unified.roadway_embankment.take();

        let mut flat = unified.clone();
        flat.roadway_embankment = None;
        flat.composed_embankment_blocked = blocked;

        let w_unified = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
            q_values: vec![15.0],
            bridge: BridgeSolveParams {
                roadway_embankment,
                ..flat.clone()
            },
        })
        .wsel[0];
        let w_flat = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
            q_values: vec![15.0],
            bridge: flat,
        })
        .wsel[0];

        assert!(
            (w_unified - w_flat).abs() < 1e-6,
            "rating curve unified vs decomposed flat (unified={w_unified:.6}, flat={w_flat:.6})"
        );
    }
}
