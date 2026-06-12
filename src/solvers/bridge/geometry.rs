use crate::geometry::{
    CrossSection, GeometryTable, GuideBanks,
    IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::{resolve_abutments, BridgeAbutments};
use crate::solvers::deck_vent_geometry::{resolve_deck_vents, ResolvedDeckVent};
use crate::solvers::pier_geometry::{
    evenly_spaced_pier_stations, resolve_pier_width_specs, PierAttachmentsUserInput,
    PierWidthUserInput, ResolvedPier,
};
use crate::utils::{UnitSystem, FT_TO_M};

use super::ice_debris::clamp_opening_blockage_factor;
use super::section::{
    apply_bridge_skew, BridgeFrictionLengths, BridgeFrictionWeighting, BridgeSectionContext,
};
use super::types::{BridgeCouplingParams, HighFlowMethod, LowFlowMethod, PierShape};


/// Piecewise-linear deck geometry across the bridge opening (HEC-RAS deck/roadway profiles).
#[derive(Debug, Clone, Default)]
pub struct BridgeDeckProfile {
    /// Horizontal stations across the opening (metric, monotonic increasing).
    pub stations_m: Vec<f64>,
    /// Low chord elevation at each station.
    pub low_elevations_m: Vec<f64>,
    /// High chord (roadway crest) elevation at each station.
    pub high_elevations_m: Vec<f64>,
}

impl BridgeDeckProfile {
    pub fn is_valid(&self) -> bool {
        let n = self.stations_m.len();
        n >= 2
            && n == self.low_elevations_m.len()
            && n == self.high_elevations_m.len()
            && self.stations_m.windows(2).all(|w| w[1] > w[0])
    }

    /// Shift opening-frame deck stations to reach XS coordinates (metric).
    pub fn remap_stations_to_reach(&mut self, origin_user: f64, units: UnitSystem) {
        let origin_m = if units == UnitSystem::USCustomary {
            origin_user * FT_TO_M
        } else {
            origin_user
        };
        for s in &mut self.stations_m {
            *s += origin_m;
        }
    }
}

/// Build a deck profile from optional per-point arrays; returns `None` when fewer than two points.
pub fn build_bridge_deck_profile(
    _scalar_low: f64,
    _scalar_high: f64,
    stations: Option<&[f64]>,
    low_elevs: Option<&[f64]>,
    high_elevs: Option<&[f64]>,
    units: UnitSystem,
) -> Option<BridgeDeckProfile> {
    let st = stations?;
    let lo = low_elevs?;
    let hi = high_elevs?;
    if st.len() < 2 || st.len() != lo.len() || st.len() != hi.len() {
        return None;
    }
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let profile = BridgeDeckProfile {
        stations_m: st.iter().map(|s| to_m(*s)).collect(),
        low_elevations_m: lo.iter().map(|e| to_m(*e)).collect(),
        high_elevations_m: hi.iter().map(|e| to_m(*e)).collect(),
    };
    if profile.is_valid() {
        Some(profile)
    } else {
        None
    }
}

pub(crate) fn interpolate_profile(stations: &[f64], elevations: &[f64], station: f64) -> f64 {
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

pub(crate) fn deck_extrema(
    scalar_low: f64,
    scalar_high: f64,
    deck: Option<&BridgeDeckProfile>,
    units: UnitSystem,
) -> (f64, f64, f64, f64) {
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let low_m = to_m(scalar_low);
    let high_m = to_m(scalar_high);
    if let Some(d) = deck.filter(|p| p.is_valid()) {
        let low_min = d
            .low_elevations_m
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let low_max = d
            .low_elevations_m
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let high_min = d
            .high_elevations_m
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let high_max = d
            .high_elevations_m
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        (low_min, low_max, high_min, high_max)
    } else {
        (low_m, low_m, high_m, high_m)
    }
}

pub(crate) fn profile_opening_area_factor(geom: &BridgeGeometry) -> f64 {
    let deck = match &geom.deck {
        Some(d) if d.is_valid() => d,
        _ => return 1.0,
    };
    let z = geom.z_up_m;
    let mut trap_area = 0.0;
    let mut width = 0.0;
    for i in 0..deck.stations_m.len() - 1 {
        let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
        if w <= 0.0 {
            continue;
        }
        let h0 = (deck.low_elevations_m[i] - z).max(0.0);
        let h1 = (deck.low_elevations_m[i + 1] - z).max(0.0);
        trap_area += 0.5 * (h0 + h1) * w;
        width += w;
    }
    let h_min = (geom.low_chord_m - z).max(0.0);
    let rect_area = h_min * width.max(1e-6);
    if rect_area > 1e-6 {
        (trap_area / rect_area).clamp(0.05, 2.0)
    } else {
        1.0
    }
}

pub(crate) fn effective_weir_length_m(geom: &BridgeGeometry, e_upstream: f64, fallback: f64) -> f64 {
    let deck = match &geom.deck {
        Some(d) if d.is_valid() => d,
        _ => return fallback,
    };
    let mut len = 0.0;
    for i in 0..deck.stations_m.len() - 1 {
        let w = (deck.stations_m[i + 1] - deck.stations_m[i]) * geom.skew_cos;
        if w <= 0.0 {
            continue;
        }
        let s_mid = 0.5 * (deck.stations_m[i] + deck.stations_m[i + 1]);
        let high_mid = effective_deck_crest_m(
            geom,
            interpolate_profile(
                &deck.stations_m,
                &deck.high_elevations_m,
                s_mid,
            ),
        );
        if e_upstream > high_mid {
            len += w;
        }
    }
    if len > 1e-3 {
        len
    } else {
        fallback.max(1e-3)
    }
}

/// Bridge geometry and coefficients in metric units.
#[derive(Debug, Clone)]
pub struct BridgeGeometry {
    /// Minimum low chord (deck-soffit) elevation — free-flow limit.
    pub low_chord_m: f64,
    /// Maximum low chord elevation — HEC-RAS pressure-flow EGL trigger.
    pub low_chord_max_m: f64,
    /// Minimum high chord (roadway crest) — weir overtopping begins.
    pub high_chord_m: f64,
    /// Maximum high chord elevation across the deck profile.
    pub high_chord_max_m: f64,
    pub pier_width_m: f64,
    pub num_piers: i32,
    /// Explicit pier centerline stations (metric); empty → evenly spaced across opening.
    pub pier_stations_m: Vec<f64>,
    /// Resolved per-pier width specs (metric). Empty → synthesize legacy constant prisms at solve time.
    pub pier_specs: Vec<ResolvedPier>,
    pub skew_deg: f64,
    pub skew_cos: f64,
    pub pier_shape: PierShape,
    pub abutments: BridgeAbutments,
    pub weir_coeff_m: f64,
    pub orifice_coeff: f64,
    pub z_up_m: f64,
    pub z_down_m: f64,
    pub low_flow_method: LowFlowMethod,
    pub high_flow_method: HighFlowMethod,
    pub length_m: f64,
    pub friction_weighting: BridgeFrictionWeighting,
    pub friction_opening_m: f64,
    pub friction_approach_m: f64,
    pub friction_departure_m: f64,
    pub wspro_coeff_c: f64,
    pub coeff_contraction: f64,
    pub coeff_expansion: f64,
    pub pressure_coeff_inlet: f64,
    pub pressure_coeff_submerged: f64,
    pub max_weir_submergence: f64,
    pub deck: Option<BridgeDeckProfile>,
    pub ineffective_up: Option<IneffectiveFlowAreas>,
    pub ineffective_down: Option<IneffectiveFlowAreas>,
    pub xs_up: Option<CrossSection>,
    pub xs_down: Option<CrossSection>,
    /// Approach (section 4) cut for contraction / WSPRO approach area.
    pub xs_approach: Option<CrossSection>,
    /// Departure cut for expansion area.
    pub xs_departure: Option<CrossSection>,
    pub guide_banks_approach: Option<GuideBanks>,
    pub guide_banks_departure: Option<GuideBanks>,
    pub table_approach: Option<GeometryTable>,
    pub table_departure: Option<GeometryTable>,
    /// Supplemental pressure-flow paths through deck vents / slots (metric).
    pub deck_vents: Vec<ResolvedDeckVent>,
    /// Interior cut tables (US → DS) for sub-segment opening friction.
    pub internal_opening_tables: Vec<GeometryTable>,
    /// Skew-adjusted segment lengths between consecutive opening nodes (BU → … → BD).
    pub internal_opening_segment_lengths_m: Vec<f64>,
    /// Bed elevation (metric) at each interior opening cut.
    pub internal_opening_z_m: Vec<f64>,
    /// Final multiplier on net opening area after debris / ice (§A).
    pub opening_blockage_factor: f64,
    /// Active constant ice thickness under deck (metric).
    pub ice_thickness_m: f64,
    /// `0` = none, `1` = constant thickness.
    pub ice_mode: u8,
    /// Roadway ice lowering weir crest (metric).
    pub deck_ice_thickness_m: f64,
    /// Resolved per-pier debris widths (metric, opening coordinates).
    pub pier_debris_widths_m: Vec<f64>,
    /// Resolved per-pier debris heights below WSEL (metric).
    pub pier_debris_heights_m: Vec<f64>,
}

impl Default for BridgeGeometry {
    fn default() -> Self {
        Self {
            low_chord_m: 5.0,
            low_chord_max_m: 5.0,
            high_chord_m: 7.0,
            high_chord_max_m: 7.0,
            pier_width_m: 0.0,
            num_piers: 0,
            pier_stations_m: Vec::new(),
            pier_specs: Vec::new(),
            skew_deg: 0.0,
            skew_cos: 1.0,
            pier_shape: PierShape::Square,
            abutments: BridgeAbutments::default(),
            weir_coeff_m: 1.44,
            orifice_coeff: 0.8,
            z_up_m: 0.0,
            z_down_m: 0.0,
            low_flow_method: LowFlowMethod::Auto,
            high_flow_method: HighFlowMethod::PressureWeir,
            length_m: 10.0,
            friction_weighting: BridgeFrictionWeighting::OpeningOnly,
            friction_opening_m: 10.0,
            friction_approach_m: 0.0,
            friction_departure_m: 0.0,
            wspro_coeff_c: 0.8,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3,
            pressure_coeff_inlet: 0.0,
            pressure_coeff_submerged: 0.8,
            max_weir_submergence: 0.98,
            deck: None,
            ineffective_up: None,
            ineffective_down: None,
            xs_up: None,
            xs_down: None,
            xs_approach: None,
            xs_departure: None,
            guide_banks_approach: None,
            guide_banks_departure: None,
            table_approach: None,
            table_departure: None,
            deck_vents: Vec::new(),
            internal_opening_tables: Vec::new(),
            internal_opening_segment_lengths_m: Vec::new(),
            internal_opening_z_m: Vec::new(),
            opening_blockage_factor: 1.0,
            ice_thickness_m: 0.0,
            ice_mode: 0,
            deck_ice_thickness_m: 0.0,
            pier_debris_widths_m: Vec::new(),
            pier_debris_heights_m: Vec::new(),
        }
    }
}

pub(crate) fn ice_thickness_active_m(geom: &BridgeGeometry) -> f64 {
    if geom.ice_mode == 1 {
        geom.ice_thickness_m.max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn capped_ice_thickness_m(geom: &BridgeGeometry, z_bed: f64) -> f64 {
    let t = ice_thickness_active_m(geom);
    if t <= 1e-9 {
        return 0.0;
    }
    let max_ice = (geom.low_chord_m - z_bed - 1e-3).max(0.0);
    t.min(max_ice)
}

pub(crate) fn effective_z_bed_m(z_bed: f64, geom: &BridgeGeometry) -> f64 {
    z_bed + capped_ice_thickness_m(geom, z_bed)
}

pub(crate) fn effective_deck_crest_m(geom: &BridgeGeometry, crest_m: f64) -> f64 {
    crest_m - geom.deck_ice_thickness_m.max(0.0)
}

pub(crate) fn effective_scalar_high_chord_m(geom: &BridgeGeometry) -> f64 {
    effective_deck_crest_m(geom, geom.high_chord_m)
}

pub(crate) fn apply_opening_blockage(a_eff: f64, geom: &BridgeGeometry) -> f64 {
    a_eff * clamp_opening_blockage_factor(geom.opening_blockage_factor)
}

pub(crate) fn scale_base_area_for_ice(a_base: f64, wsel: f64, z_bed: f64, geom: &BridgeGeometry) -> f64 {
    let ice = capped_ice_thickness_m(geom, z_bed);
    if ice <= 1e-9 || wsel <= z_bed + 1e-9 {
        return a_base;
    }
    let h_total = wsel - z_bed;
    let h_flow = (wsel - effective_z_bed_m(z_bed, geom)).max(0.0);
    a_base * (h_flow / h_total).min(1.0)
}
pub(crate) fn pier_width_user_to_metric(user: &PierWidthUserInput, units: UnitSystem) -> PierWidthUserInput {
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let to_m_vec = |v: &Option<Vec<f64>>| v.as_ref().map(|xs| xs.iter().map(|x| to_m(*x)).collect());
    let to_m_mat = |v: &Option<Vec<Vec<f64>>>| {
        v.as_ref().map(|rows| {
            rows.iter()
                .map(|row| row.iter().map(|x| to_m(*x)).collect())
                .collect()
        })
    };
    PierWidthUserInput {
        top_widths: to_m_vec(&user.top_widths),
        bottom_widths: to_m_vec(&user.bottom_widths),
        width_elevations: to_m_mat(&user.width_elevations),
        width_values: to_m_mat(&user.width_values),
        top_elevations: to_m_vec(&user.top_elevations),
        base_elevations: to_m_vec(&user.base_elevations),
    }
}

pub(crate) fn pier_attachments_user_to_metric(
    user: &PierAttachmentsUserInput,
    units: UnitSystem,
) -> PierAttachmentsUserInput {
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let to_m_vec = |v: &Option<Vec<f64>>| v.as_ref().map(|xs| xs.iter().map(|x| to_m(*x)).collect());
    PierAttachmentsUserInput {
        footing_top_elevations: to_m_vec(&user.footing_top_elevations),
        footing_widths: to_m_vec(&user.footing_widths),
        footing_bottom_elevations: to_m_vec(&user.footing_bottom_elevations),
        nosing_lengths: to_m_vec(&user.nosing_lengths),
        nosing_widths: to_m_vec(&user.nosing_widths),
    }
}

pub(crate) fn resolve_ice_debris_geometry_fields(
    coupling: &BridgeCouplingParams,
    units: UnitSystem,
) -> (f64, f64, u8, f64, Vec<f64>, Vec<f64>) {
    let to_m = |v: f64| {
        if units == UnitSystem::USCustomary {
            v * FT_TO_M
        } else {
            v
        }
    };
    let id = &coupling.ice_debris;
    (
        clamp_opening_blockage_factor(id.opening_blockage_factor),
        to_m(id.ice_thickness),
        id.ice_mode,
        to_m(id.deck_ice_thickness),
        id.pier_debris_widths.iter().map(|&w| to_m(w)).collect(),
        id.pier_debris_heights.iter().map(|&h| to_m(h)).collect(),
    )
}

pub(crate) fn build_bridge_geometry(
    low_chord: f64,
    high_chord: f64,
    pier_width: f64,
    num_piers: i32,
    pier_shape_type: i32,
    weir_coeff: f64,
    orifice_coeff: f64,
    z_down: f64,
    z_up: f64,
    units: UnitSystem,
    coupling: &BridgeCouplingParams,
    interval_length: f64,
    deck: Option<&BridgeDeckProfile>,
    sections: Option<&BridgeSectionContext>,
) -> BridgeGeometry {
    let (low_min, low_max, high_min, high_max) = deck_extrema(low_chord, high_chord, deck, units);
    let length_base_m = sections
        .map(|s| s.friction_length_m)
        .filter(|&l| l > 1e-3)
        .unwrap_or_else(|| {
            if interval_length > 1e-3 {
                interval_length
            } else if coupling.length > 1e-6 {
                if units == UnitSystem::USCustomary {
                    coupling.length * FT_TO_M
                } else {
                    coupling.length
                }
            } else {
                10.0
            }
        });
    let skew_deg = sections.map(|s| s.skew_deg).unwrap_or(0.0);
    let friction_lengths = sections
        .map(|s| s.friction_lengths)
        .unwrap_or(BridgeFrictionLengths {
            weighting: coupling.friction_weighting,
            opening_m: length_base_m,
            approach_m: 0.0,
            departure_m: 0.0,
        });
    let opening_base_m = if friction_lengths.opening_m > 1e-3 {
        friction_lengths.opening_m
    } else {
        length_base_m
    };
    let (_, length_m) = apply_bridge_skew(skew_deg, 1.0, opening_base_m);
    let (_, friction_opening_m) = apply_bridge_skew(skew_deg, 1.0, opening_base_m);
    let (_, friction_approach_m) = apply_bridge_skew(skew_deg, 1.0, friction_lengths.approach_m);
    let (_, friction_departure_m) =
        apply_bridge_skew(skew_deg, 1.0, friction_lengths.departure_m);
    let friction_weighting = coupling.friction_weighting;
    let skew_cos = {
        let deg = skew_deg.clamp(0.0, 59.0);
        deg.to_radians().cos().max(0.52)
    };

    let pier_stations_m = sections
        .and_then(|s| s.pier_stations.as_ref())
        .map(|st| {
            st.iter()
                .map(|x| {
                    if units == UnitSystem::USCustomary {
                        x * FT_TO_M
                    } else {
                        *x
                    }
                })
                .collect::<Vec<f64>>()
        })
        .unwrap_or_default();
    let num_piers = if !pier_stations_m.is_empty() {
        pier_stations_m.len() as i32
    } else {
        num_piers
    };

    let submerged_c = if orifice_coeff > 1e-6 {
        orifice_coeff
    } else {
        coupling.pressure_coeff_submerged
    };

    let opening_origin_user = sections.and_then(|s| s.opening_reach_station_origin);
    let deck_owned = deck.cloned().map(|mut d| {
        if let Some(origin) = opening_origin_user {
            d.remap_stations_to_reach(origin, units);
        }
        d
    });
    let abutment_input = opening_origin_user
        .map(|origin| {
            crate::solvers::bridge_abutment::remap_abutment_input_to_reach(&coupling.abutment, origin)
        })
        .unwrap_or_else(|| coupling.abutment.clone());
    let to_metric_ineffective = |i: &IneffectiveFlowAreas| {
        if units == UnitSystem::USCustomary {
            i.to_metric(UnitSystem::USCustomary)
        } else {
            i.clone()
        }
    };
    let ineffective_up = sections
        .and_then(|s| s.ineffective_up.as_ref())
        .filter(|i| i.is_configured())
        .map(to_metric_ineffective);
    let ineffective_down = sections
        .and_then(|s| s.ineffective_down.as_ref())
        .filter(|i| i.is_configured())
        .map(to_metric_ineffective);
    let xs_up = sections.and_then(|s| s.xs_up.clone()).map(|xs| {
        if units == UnitSystem::USCustomary {
            xs.to_metric()
        } else {
            xs
        }
    });
    let xs_down = sections.and_then(|s| s.xs_down.clone()).map(|xs| {
        if units == UnitSystem::USCustomary {
            xs.to_metric()
        } else {
            xs
        }
    });
    let internal_metric: Vec<CrossSection> = sections
        .map(|s| {
            s.internal_xs
                .iter()
                .map(|xs| {
                    if units == UnitSystem::USCustomary {
                        xs.to_metric()
                    } else {
                        xs.clone()
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let (internal_opening_tables, internal_opening_segment_lengths_m, internal_opening_z_m) =
        internal_opening_friction_segments(
            xs_up.as_ref(),
            &internal_metric,
            xs_down.as_ref(),
            skew_deg,
        );
    let to_metric_xs = |xs: CrossSection| {
        if units == UnitSystem::USCustomary {
            xs.to_metric()
        } else {
            xs
        }
    };
    let to_metric_guide = |gb: &GuideBanks| {
        if units == UnitSystem::USCustomary {
            gb.to_metric(UnitSystem::USCustomary)
        } else {
            gb.clone()
        }
    };
    let xs_approach = sections
        .and_then(|s| s.xs_approach.clone())
        .map(to_metric_xs);
    let xs_departure = sections
        .and_then(|s| s.xs_departure.clone())
        .map(to_metric_xs);
    let guide_banks_approach = sections
        .and_then(|s| s.guide_banks_approach.as_ref())
        .filter(|g| g.is_configured())
        .map(to_metric_guide);
    let guide_banks_departure = sections
        .and_then(|s| s.guide_banks_departure.as_ref())
        .filter(|g| g.is_configured())
        .map(to_metric_guide);
    let table_approach = xs_approach
        .as_ref()
        .map(|xs| xs.generate_lookup_table(APPROACH_DEPARTURE_TABLE_SLICES));
    let table_departure = xs_departure
        .as_ref()
        .map(|xs| xs.generate_lookup_table(APPROACH_DEPARTURE_TABLE_SLICES));

    let (opening_s_min, opening_s_max) = opening_station_bounds_from_deck(deck_owned.as_ref());
    let abutments = resolve_abutments(&abutment_input, opening_s_min, opening_s_max, skew_cos, units);

    let pier_width_perp_m = if units == UnitSystem::USCustomary {
        pier_width * FT_TO_M
    } else {
        pier_width
    };
    let z_up_m = if units == UnitSystem::USCustomary {
        z_up * FT_TO_M
    } else {
        z_up
    };
    let z_down_m = if units == UnitSystem::USCustomary {
        z_down * FT_TO_M
    } else {
        z_down
    };
    let pier_width_user = sections
        .and_then(|s| s.pier_widths.as_ref())
        .map(|u| pier_width_user_to_metric(u, units));
    let pier_attachments_user = sections
        .and_then(|s| s.pier_attachments.as_ref())
        .map(|u| pier_attachments_user_to_metric(u, units));
    let inset = pier_width_perp_m.max(0.0) * 0.5;
    let pier_station_list = if !pier_stations_m.is_empty() {
        pier_stations_m.clone()
    } else {
        evenly_spaced_pier_stations(num_piers, opening_s_min, opening_s_max, inset)
    };
    let z_bed_m = z_up_m.min(z_down_m);
    let z_top_defaults: Vec<f64> = pier_station_list
        .iter()
        .map(|&s| {
            deck_owned
                .as_ref()
                .map(|d| interpolate_profile(&d.stations_m, &d.low_elevations_m, s))
                .unwrap_or(low_min)
        })
        .collect();
    let pier_specs = resolve_pier_width_specs(
        pier_width_perp_m,
        &pier_station_list,
        z_bed_m,
        &z_top_defaults,
        pier_width_user.as_ref(),
        pier_attachments_user.as_ref(),
    );

    let deck_vents = sections
        .and_then(|s| s.deck_vents.as_ref())
        .map(|u| resolve_deck_vents(u, skew_cos, units, submerged_c))
        .unwrap_or_default();
    let (
        opening_blockage_factor,
        ice_thickness_m,
        ice_mode,
        deck_ice_thickness_m,
        pier_debris_widths_m,
        pier_debris_heights_m,
    ) = resolve_ice_debris_geometry_fields(coupling, units);

    if units == UnitSystem::USCustomary {
        BridgeGeometry {
            low_chord_m: low_min,
            low_chord_max_m: low_max,
            high_chord_m: high_min,
            high_chord_max_m: high_max,
            pier_width_m: pier_width_perp_m,
            num_piers,
            pier_stations_m: pier_stations_m.clone(),
            pier_specs: pier_specs.clone(),
            skew_deg,
            skew_cos,
            pier_shape: PierShape::from_i32(pier_shape_type),
            abutments: abutments.clone(),
            weir_coeff_m: weir_coeff / 1.8113,
            orifice_coeff: submerged_c,
            z_up_m,
            z_down_m,
            low_flow_method: LowFlowMethod::from_i32(coupling.low_flow_method),
            high_flow_method: HighFlowMethod::from_i32(coupling.high_flow_method),
            length_m,
            friction_weighting,
            friction_opening_m,
            friction_approach_m,
            friction_departure_m,
            wspro_coeff_c: coupling.wspro_coeff,
            coeff_contraction: coupling.coeff_contraction,
            coeff_expansion: coupling.coeff_expansion,
            pressure_coeff_inlet: coupling.pressure_coeff_inlet,
            pressure_coeff_submerged: submerged_c,
            max_weir_submergence: coupling.max_weir_submergence,
            deck: deck_owned.clone(),
            ineffective_up: ineffective_up.clone(),
            ineffective_down: ineffective_down.clone(),
            xs_up: xs_up.clone(),
            xs_down: xs_down.clone(),
            xs_approach: xs_approach.clone(),
            xs_departure: xs_departure.clone(),
            guide_banks_approach: guide_banks_approach.clone(),
            guide_banks_departure: guide_banks_departure.clone(),
            table_approach: table_approach.clone(),
            table_departure: table_departure.clone(),
            deck_vents: deck_vents.clone(),
            internal_opening_tables: internal_opening_tables.clone(),
            internal_opening_segment_lengths_m: internal_opening_segment_lengths_m.clone(),
            internal_opening_z_m: internal_opening_z_m.clone(),
            opening_blockage_factor,
            ice_thickness_m,
            ice_mode,
            deck_ice_thickness_m,
            pier_debris_widths_m: pier_debris_widths_m.clone(),
            pier_debris_heights_m: pier_debris_heights_m.clone(),
        }
    } else {
        BridgeGeometry {
            low_chord_m: low_min,
            low_chord_max_m: low_max,
            high_chord_m: high_min,
            high_chord_max_m: high_max,
            pier_width_m: pier_width_perp_m,
            num_piers,
            pier_stations_m,
            pier_specs,
            skew_deg,
            skew_cos,
            pier_shape: PierShape::from_i32(pier_shape_type),
            abutments,
            weir_coeff_m: weir_coeff,
            orifice_coeff: submerged_c,
            z_up_m,
            z_down_m,
            low_flow_method: LowFlowMethod::from_i32(coupling.low_flow_method),
            high_flow_method: HighFlowMethod::from_i32(coupling.high_flow_method),
            length_m,
            friction_weighting,
            friction_opening_m,
            friction_approach_m,
            friction_departure_m,
            wspro_coeff_c: coupling.wspro_coeff,
            coeff_contraction: coupling.coeff_contraction,
            coeff_expansion: coupling.coeff_expansion,
            pressure_coeff_inlet: coupling.pressure_coeff_inlet,
            pressure_coeff_submerged: submerged_c,
            max_weir_submergence: coupling.max_weir_submergence,
            deck: deck_owned,
            ineffective_up,
            ineffective_down,
            xs_up,
            xs_down,
            xs_approach,
            xs_departure,
            guide_banks_approach,
            guide_banks_departure,
            table_approach,
            table_departure,
            deck_vents,
            internal_opening_tables,
            internal_opening_segment_lengths_m,
            internal_opening_z_m,
            opening_blockage_factor,
            ice_thickness_m,
            ice_mode,
            deck_ice_thickness_m,
            pier_debris_widths_m,
            pier_debris_heights_m,
        }
    }
}

pub(crate) const APPROACH_DEPARTURE_TABLE_SLICES: usize = 50;

pub(crate) fn opening_station_bounds_from_deck(deck: Option<&BridgeDeckProfile>) -> (f64, f64) {
    if let Some(deck) = deck.filter(|d| d.is_valid()) {
        return (
            deck.stations_m.first().copied().unwrap_or(0.0),
            deck.stations_m.last().copied().unwrap_or(0.0),
        );
    }
    (0.0, 10.0)
}


/// Skew-adjusted segment lengths along explicit BU → internal → BD river stations (metric).
pub(crate) fn internal_opening_friction_segments(
    xs_up: Option<&CrossSection>,
    internal: &[CrossSection],
    xs_down: Option<&CrossSection>,
    skew_deg: f64,
) -> (Vec<GeometryTable>, Vec<f64>, Vec<f64>) {
    if internal.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let mut nodes: Vec<&CrossSection> = Vec::new();
    if let Some(bu) = xs_up {
        nodes.push(bu);
    }
    for xs in internal {
        nodes.push(xs);
    }
    if let Some(bd) = xs_down {
        nodes.push(bd);
    }
    if nodes.len() < 3 {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    let mut stations: Vec<f64> = nodes.iter().map(|xs| xs.station).collect();
    stations.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let segment_lengths_m: Vec<f64> = stations
        .windows(2)
        .map(|w| {
            let seg = (w[0] - w[1]).abs();
            apply_bridge_skew(skew_deg, 1.0, seg).1
        })
        .collect();

    let mut tables = Vec::with_capacity(internal.len());
    let mut z_m = Vec::with_capacity(internal.len());
    for xs in internal {
        tables.push(xs.generate_lookup_table(APPROACH_DEPARTURE_TABLE_SLICES));
        z_m.push(crate::solvers::bridge_interior::cross_section_min_bed(xs));
    }
    (tables, segment_lengths_m, z_m)
}


