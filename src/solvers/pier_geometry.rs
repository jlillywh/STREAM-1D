//! Per-pier width specifications and submerged plan-area integration.

/// Optional per-pier width overrides (user units before metric conversion in `build_bridge_geometry`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PierWidthUserInput {
    pub top_widths: Option<Vec<f64>>,
    pub bottom_widths: Option<Vec<f64>>,
    pub width_elevations: Option<Vec<Vec<f64>>>,
    pub width_values: Option<Vec<Vec<f64>>>,
    pub top_elevations: Option<Vec<f64>>,
    pub base_elevations: Option<Vec<f64>>,
}

/// Optional per-pier footing and nosing (user units before metric conversion).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PierAttachmentsUserInput {
    pub footing_top_elevations: Option<Vec<f64>>,
    pub footing_widths: Option<Vec<f64>>,
    pub footing_bottom_elevations: Option<Vec<f64>>,
    pub nosing_lengths: Option<Vec<f64>>,
    pub nosing_widths: Option<Vec<f64>>,
}

/// Flow-normal upstream nosing extension (metric).
#[derive(Debug, Clone)]
pub struct ResolvedPierNosing {
    pub length_perp_m: f64,
    /// When `None`, use shaft perpendicular width at each elevation.
    pub width_perp_m: Option<f64>,
}

/// Resolved pier width definition in metric units (perpendicular to flow).
#[derive(Debug, Clone)]
pub enum PierWidthSpec {
    /// Legacy constant prism: one perpendicular width from bed to WSEL.
    Constant { width_perp_m: f64 },
    /// Linear taper between base and cap elevations.
    Tapered {
        top_width_perp_m: f64,
        bottom_width_perp_m: f64,
        z_top_m: f64,
        z_base_m: f64,
    },
    /// Piecewise-linear width vs absolute elevation (≥ 2 points).
    Profile {
        elevations_m: Vec<f64>,
        widths_perp_m: Vec<f64>,
    },
}

/// One pier with centerline station and resolved width spec.
#[derive(Debug, Clone)]
pub struct ResolvedPier {
    pub station_m: f64,
    pub spec: PierWidthSpec,
    pub nosing: Option<ResolvedPierNosing>,
}

impl ResolvedPier {
    fn wetted_vertical_limits(&self, wsel_m: f64, z_bed_m: f64) -> (f64, f64) {
        let z_lo = self.spec.z_base_m(z_bed_m).max(z_bed_m);
        let z_hi = wsel_m.min(self.spec.z_top_m());
        (z_lo, z_hi)
    }

    /// Shaft plan area only (footing merged into `spec` when composed).
    pub fn shaft_submerged_area_m2(&self, wsel_m: f64, z_bed_m: f64) -> f64 {
        self.spec.submerged_area_m2(wsel_m, z_bed_m)
    }

    /// Upstream nosing plan area: $L_\perp \times W_{nose} \times h_{wet}$.
    pub fn nosing_submerged_area_m2(&self, wsel_m: f64, z_bed_m: f64) -> f64 {
        let Some(n) = &self.nosing else {
            return 0.0;
        };
        if n.length_perp_m <= 1e-9 {
            return 0.0;
        }
        let (z_lo, z_hi) = self.wetted_vertical_limits(wsel_m, z_bed_m);
        let h = z_hi - z_lo;
        if h <= 1e-9 {
            return 0.0;
        }
        let w_nose = n
            .width_perp_m
            .unwrap_or_else(|| self.spec.width_perp_at_wsel(wsel_m, z_bed_m));
        n.length_perp_m * w_nose.max(0.0) * h
    }

    /// Total submerged opening-plane pier area including nosing.
    pub fn submerged_area_m2(&self, wsel_m: f64, z_bed_m: f64) -> f64 {
        self.shaft_submerged_area_m2(wsel_m, z_bed_m) + self.nosing_submerged_area_m2(wsel_m, z_bed_m)
    }

    /// Opening-plane top width at WSEL (shaft + nosing length when wet).
    pub fn flow_width_at_wsel_opening_m(&self, wsel_m: f64, z_bed_m: f64, skew_cos: f64) -> f64 {
        let cos = skew_cos.max(1e-6);
        let w_shaft = self.spec.width_perp_at_wsel(wsel_m, z_bed_m) / cos;
        if w_shaft <= 1e-9 {
            return 0.0;
        }
        let l_nose = self
            .nosing
            .as_ref()
            .filter(|n| n.length_perp_m > 1e-9)
            .map(|n| n.length_perp_m / cos)
            .unwrap_or(0.0);
        w_shaft + l_nose
    }
}

fn interpolate_profile(stations: &[f64], values: &[f64], x: f64) -> f64 {
    if stations.is_empty() {
        return 0.0;
    }
    if x <= stations[0] {
        return values[0];
    }
    if x >= stations[stations.len() - 1] {
        return values[values.len() - 1];
    }
    for i in 0..stations.len() - 1 {
        if x <= stations[i + 1] {
            let t = (x - stations[i]) / (stations[i + 1] - stations[i]);
            return values[i] + t * (values[i + 1] - values[i]);
        }
    }
    values[values.len() - 1]
}

fn integrate_width_profile(elevations: &[f64], widths: &[f64], z_lo: f64, z_hi: f64) -> f64 {
    if z_hi <= z_lo + 1e-9 || elevations.len() < 2 || elevations.len() != widths.len() {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..elevations.len() - 1 {
        let z0 = elevations[i];
        let z1 = elevations[i + 1];
        if z1 <= z_lo || z0 >= z_hi {
            continue;
        }
        let seg_lo = z0.max(z_lo);
        let seg_hi = z1.min(z_hi);
        if seg_hi <= seg_lo + 1e-9 {
            continue;
        }
        let w0 = widths[i];
        let w1 = widths[i + 1];
        let dz = z1 - z0;
        let t_lo = if dz.abs() > 1e-9 {
            (seg_lo - z0) / dz
        } else {
            0.0
        };
        let t_hi = if dz.abs() > 1e-9 {
            (seg_hi - z0) / dz
        } else {
            0.0
        };
        let w_lo = w0 + t_lo * (w1 - w0);
        let w_hi = w0 + t_hi * (w1 - w0);
        area += 0.5 * (w_lo + w_hi) * (seg_hi - seg_lo);
    }
    area
}

impl PierWidthSpec {
    pub(crate) fn z_top_m(&self) -> f64 {
        match self {
            PierWidthSpec::Constant { .. } => f64::INFINITY,
            PierWidthSpec::Tapered { z_top_m, .. } => *z_top_m,
            PierWidthSpec::Profile { elevations_m, .. } => {
                elevations_m.last().copied().unwrap_or(f64::INFINITY)
            }
        }
    }

    pub(crate) fn z_base_m(&self, z_bed_m: f64) -> f64 {
        match self {
            PierWidthSpec::Constant { .. } => z_bed_m,
            PierWidthSpec::Tapered { z_base_m, .. } => *z_base_m,
            PierWidthSpec::Profile { elevations_m, .. } => {
                elevations_m.first().copied().unwrap_or(z_bed_m)
            }
        }
    }

    /// Perpendicular width at absolute elevation `z` (clamped to pier extent).
    pub fn width_perp_at(&self, z_m: f64) -> f64 {
        match self {
            PierWidthSpec::Constant { width_perp_m } => *width_perp_m,
            PierWidthSpec::Tapered {
                top_width_perp_m,
                bottom_width_perp_m,
                z_top_m,
                z_base_m,
            } => {
                if z_m <= *z_base_m {
                    return *bottom_width_perp_m;
                }
                if z_m >= *z_top_m {
                    return *top_width_perp_m;
                }
                interpolate_profile(
                    &[*z_base_m, *z_top_m],
                    &[*bottom_width_perp_m, *top_width_perp_m],
                    z_m,
                )
            }
            PierWidthSpec::Profile {
                elevations_m,
                widths_perp_m,
            } => interpolate_profile(elevations_m, widths_perp_m, z_m),
        }
    }

    fn wetted_vertical_limits(&self, wsel_m: f64, z_bed_m: f64) -> (f64, f64) {
        let z_lo = self.z_base_m(z_bed_m).max(z_bed_m);
        let z_hi = wsel_m.min(self.z_top_m());
        (z_lo, z_hi)
    }

    /// Submerged plan area using perpendicular widths: ∫ w_perp(z) dz from bed to WSEL.
    pub fn submerged_area_m2(&self, wsel_m: f64, z_bed_m: f64) -> f64 {
        let (z_lo, z_hi) = self.wetted_vertical_limits(wsel_m, z_bed_m);
        if z_hi <= z_lo + 1e-9 {
            return 0.0;
        }
        match self {
            PierWidthSpec::Constant { width_perp_m } => width_perp_m * (z_hi - z_lo),
            PierWidthSpec::Tapered {
                top_width_perp_m,
                bottom_width_perp_m,
                z_top_m,
                z_base_m,
            } => integrate_width_profile(
                &[*z_base_m, *z_top_m],
                &[*bottom_width_perp_m, *top_width_perp_m],
                z_lo,
                z_hi,
            ),
            PierWidthSpec::Profile {
                elevations_m,
                widths_perp_m,
            } => integrate_width_profile(elevations_m, widths_perp_m, z_lo, z_hi),
        }
    }

    /// Perpendicular width at the water surface (0 when pier top is above WSEL).
    pub fn width_perp_at_wsel(&self, wsel_m: f64, z_bed_m: f64) -> f64 {
        let (z_lo, z_hi) = self.wetted_vertical_limits(wsel_m, z_bed_m);
        if z_hi <= z_lo + 1e-9 {
            return 0.0;
        }
        self.width_perp_at(wsel_m)
    }
}

pub fn evenly_spaced_pier_stations(
    num_piers: i32,
    opening_s_min: f64,
    opening_s_max: f64,
    inset_m: f64,
) -> Vec<f64> {
    let n = num_piers.max(0);
    if n == 0 {
        return vec![];
    }
    let span = (opening_s_max - opening_s_min).max(1e-3);
    let usable = (span - 2.0 * inset_m).max(inset_m);
    (0..n)
        .map(|i| opening_s_min + inset_m + usable * (i as f64 + 1.0) / (n as f64 + 1.0))
        .collect()
}

fn spec_to_profile_points(spec: &PierWidthSpec, z_bed_m: f64, z_cap_m: f64) -> (Vec<f64>, Vec<f64>) {
    match spec {
        PierWidthSpec::Constant { width_perp_m } => (
            vec![z_bed_m, z_cap_m.max(z_bed_m + 1e-9)],
            vec![*width_perp_m, *width_perp_m],
        ),
        PierWidthSpec::Tapered {
            top_width_perp_m,
            bottom_width_perp_m,
            z_top_m,
            z_base_m,
        } => (
            vec![*z_base_m, *z_top_m],
            vec![*bottom_width_perp_m, *top_width_perp_m],
        ),
        PierWidthSpec::Profile {
            elevations_m,
            widths_perp_m,
        } => (elevations_m.clone(), widths_perp_m.clone()),
    }
}

fn merge_footing_profile(
    elevs: &[f64],
    widths: &[f64],
    z_bed_m: f64,
    footing_top_m: f64,
    footing_bottom_m: f64,
    footing_width_m: f64,
) -> (Vec<f64>, Vec<f64>) {
    // Profile wins only when it already extends below the footing top (shaft base).
    if !elevs.is_empty() && elevs[0] < footing_top_m - 1e-9 {
        return (elevs.to_vec(), widths.to_vec());
    }
    let z_bot = footing_bottom_m.min(footing_top_m - 1e-6).max(z_bed_m);
    let mut out_e = vec![z_bot, footing_top_m];
    let mut out_w = vec![footing_width_m, footing_width_m];
    let w_shaft = widths[0];
    if (elevs[0] - footing_top_m).abs() <= 1e-9 {
        // Shaft base coincides with footing top — one point, shaft width above.
        *out_w.last_mut().unwrap() = w_shaft;
    } else if elevs[0] > footing_top_m + 1e-9 {
        out_e.push(elevs[0]);
        out_w.push(w_shaft);
    } else if (w_shaft - footing_width_m).abs() > 1e-9 {
        out_e.push(footing_top_m);
        out_w.push(w_shaft);
    }
    for i in 1..elevs.len() {
        if out_e.last().map(|&z| (z - elevs[i]).abs() < 1e-9).unwrap_or(false) {
            *out_w.last_mut().unwrap() = widths[i];
        } else {
            out_e.push(elevs[i]);
            out_w.push(widths[i]);
        }
    }
    (out_e, out_w)
}

fn apply_footing_to_spec(
    spec: PierWidthSpec,
    z_bed_m: f64,
    z_cap_m: f64,
    footing_top_m: f64,
    footing_bottom_m: f64,
    footing_width_m: f64,
) -> PierWidthSpec {
    let (elevs, widths) = spec_to_profile_points(&spec, z_bed_m, z_cap_m);
    let (elevs, widths) = merge_footing_profile(
        &elevs,
        &widths,
        z_bed_m,
        footing_top_m,
        footing_bottom_m,
        footing_width_m,
    );
    if elevs.len() >= 2 && valid_profile(&elevs, &widths) {
        PierWidthSpec::Profile {
            elevations_m: elevs,
            widths_perp_m: widths,
        }
    } else {
        spec
    }
}

fn resolve_nosing(
    pier_idx: usize,
    user: Option<&PierAttachmentsUserInput>,
) -> Option<ResolvedPierNosing> {
    let u = user?;
    let length = u.nosing_lengths.as_ref()?.get(pier_idx).copied()?;
    if length <= 1e-9 {
        return None;
    }
    let width = u.nosing_widths.as_ref().and_then(|v| v.get(pier_idx)).copied();
    Some(ResolvedPierNosing {
        length_perp_m: length,
        width_perp_m: width.filter(|&w| w > 1e-9),
    })
}

fn resolve_footing_for_pier(
    pier_idx: usize,
    z_bed_m: f64,
    _z_cap_m: f64,
    user: Option<&PierAttachmentsUserInput>,
) -> Option<(f64, f64, f64)> {
    let u = user?;
    let z_top = u.footing_top_elevations.as_ref()?.get(pier_idx).copied()?;
    let width = u.footing_widths.as_ref()?.get(pier_idx).copied()?;
    if width <= 1e-9 {
        return None;
    }
    let z_bot = u
        .footing_bottom_elevations
        .as_ref()
        .and_then(|v| v.get(pier_idx))
        .copied()
        .unwrap_or(z_bed_m.min(z_top - 1e-3));
    Some((z_top, z_bot, width))
}

fn valid_profile(elevations: &[f64], widths: &[f64]) -> bool {
    if elevations.len() < 2 || elevations.len() != widths.len() {
        return false;
    }
    if widths.iter().any(|&w| w <= 1e-6) {
        return false;
    }
    elevations
        .windows(2)
        .all(|w| w[1] > w[0] + 1e-9)
}

fn resolve_one_pier(
    pier_idx: usize,
    legacy_width_perp_m: f64,
    z_bed_m: f64,
    z_top_default_m: f64,
    width_user: Option<&PierWidthUserInput>,
    attachments_user: Option<&PierAttachmentsUserInput>,
) -> ResolvedPier {
    let mut spec = resolve_one_pier_spec(
        pier_idx,
        legacy_width_perp_m,
        z_bed_m,
        z_top_default_m,
        width_user,
    );
    if let Some((z_foot_top, z_foot_bot, w_foot)) =
        resolve_footing_for_pier(pier_idx, z_bed_m, z_top_default_m, attachments_user)
    {
        spec = apply_footing_to_spec(spec, z_bed_m, z_top_default_m, z_foot_top, z_foot_bot, w_foot);
    }
    ResolvedPier {
        station_m: 0.0,
        spec,
        nosing: resolve_nosing(pier_idx, attachments_user),
    }
}

fn resolve_one_pier_spec(
    pier_idx: usize,
    legacy_width_perp_m: f64,
    z_bed_m: f64,
    z_top_default_m: f64,
    user: Option<&PierWidthUserInput>,
) -> PierWidthSpec {

    if let Some(u) = user {
        if let (Some(elevs), Some(widths)) = (
            u.width_elevations.as_ref().and_then(|v| v.get(pier_idx)),
            u.width_values.as_ref().and_then(|v| v.get(pier_idx)),
        ) {
            if valid_profile(elevs, widths) {
                return PierWidthSpec::Profile {
                    elevations_m: elevs.clone(),
                    widths_perp_m: widths.clone(),
                };
            }
        }

        let top_w = u.top_widths.as_ref().and_then(|v| v.get(pier_idx)).copied();
        let bot_w = u
            .bottom_widths
            .as_ref()
            .and_then(|v| v.get(pier_idx))
            .copied();
        if let (Some(top), Some(bot)) = (top_w, bot_w) {
            if top > 1e-6 && bot > 1e-6 {
                let z_top = u
                    .top_elevations
                    .as_ref()
                    .and_then(|v| v.get(pier_idx))
                    .copied()
                    .unwrap_or(z_top_default_m);
                let z_base = u
                    .base_elevations
                    .as_ref()
                    .and_then(|v| v.get(pier_idx))
                    .copied()
                    .unwrap_or(z_bed_m);
                return PierWidthSpec::Tapered {
                    top_width_perp_m: top,
                    bottom_width_perp_m: bot,
                    z_top_m: z_top,
                    z_base_m: z_base,
                };
            }
        }
    }

    PierWidthSpec::Constant {
        width_perp_m: legacy_width_perp_m,
    }
}

/// Build per-pier width specs for a bridge opening (all values in metric).
pub fn resolve_pier_width_specs(
    legacy_width_perp_m: f64,
    pier_stations_m: &[f64],
    z_bed_m: f64,
    z_top_defaults_m: &[f64],
    width_user: Option<&PierWidthUserInput>,
    attachments_user: Option<&PierAttachmentsUserInput>,
) -> Vec<ResolvedPier> {
    pier_stations_m
        .iter()
        .enumerate()
        .map(|(i, &station_m)| {
            let z_top = z_top_defaults_m.get(i).copied().unwrap_or(z_bed_m);
            let mut pier = resolve_one_pier(
                i,
                legacy_width_perp_m,
                z_bed_m,
                z_top,
                width_user,
                attachments_user,
            );
            pier.station_m = station_m;
            pier
        })
        .collect()
}

/// Sum submerged opening-plane pier area at WSEL (widths perpendicular → opening plane via skew).
pub fn total_submerged_pier_area_m2(
    piers: &[ResolvedPier],
    wsel_m: f64,
    z_bed_m: f64,
    skew_cos: f64,
) -> f64 {
    let cos = skew_cos.max(1e-6);
    piers
        .iter()
        .map(|p| p.submerged_area_m2(wsel_m, z_bed_m) / cos)
        .sum()
}

fn nested_bridge_row<T: Clone>(rows: &Option<Vec<Vec<T>>>, b_idx: usize) -> Option<Vec<T>> {
    rows.as_ref().and_then(|r| r.get(b_idx)).cloned()
}

fn nested_pier_profile_rows(rows: &Option<Vec<Vec<Vec<f64>>>>, b_idx: usize) -> Option<Vec<Vec<f64>>> {
    rows.as_ref().and_then(|r| r.get(b_idx)).cloned()
}

/// Per-bridge pier width overrides from steady/unsteady flat arrays (`[bridge][pier]` or `[bridge][pier][point]`).
pub fn pier_width_user_for_bridge_index(
    top_widths: &Option<Vec<Vec<f64>>>,
    bottom_widths: &Option<Vec<Vec<f64>>>,
    width_elevations: &Option<Vec<Vec<Vec<f64>>>>,
    width_values: &Option<Vec<Vec<Vec<f64>>>>,
    top_elevations: &Option<Vec<Vec<f64>>>,
    base_elevations: &Option<Vec<Vec<f64>>>,
    b_idx: usize,
) -> Option<PierWidthUserInput> {
    let input = PierWidthUserInput {
        top_widths: nested_bridge_row(top_widths, b_idx),
        bottom_widths: nested_bridge_row(bottom_widths, b_idx),
        width_elevations: nested_pier_profile_rows(width_elevations, b_idx),
        width_values: nested_pier_profile_rows(width_values, b_idx),
        top_elevations: nested_bridge_row(top_elevations, b_idx),
        base_elevations: nested_bridge_row(base_elevations, b_idx),
    };
    if input.top_widths.is_some()
        || input.bottom_widths.is_some()
        || input.width_elevations.is_some()
        || input.width_values.is_some()
        || input.top_elevations.is_some()
        || input.base_elevations.is_some()
    {
        Some(input)
    } else {
        None
    }
}

/// Pier width overrides for standalone bridge / rating curve (`BridgeSolveParams`, single opening).
pub fn pier_width_user_from_rating_params(
    top_widths: &Option<Vec<f64>>,
    bottom_widths: &Option<Vec<f64>>,
    width_elevations: &Option<Vec<Vec<f64>>>,
    width_values: &Option<Vec<Vec<f64>>>,
    top_elevations: &Option<Vec<f64>>,
    base_elevations: &Option<Vec<f64>>,
) -> Option<PierWidthUserInput> {
    let input = PierWidthUserInput {
        top_widths: top_widths.clone(),
        bottom_widths: bottom_widths.clone(),
        width_elevations: width_elevations.clone(),
        width_values: width_values.clone(),
        top_elevations: top_elevations.clone(),
        base_elevations: base_elevations.clone(),
    };
    if input.top_widths.is_some()
        || input.bottom_widths.is_some()
        || input.width_elevations.is_some()
        || input.width_values.is_some()
        || input.top_elevations.is_some()
        || input.base_elevations.is_some()
    {
        Some(input)
    } else {
        None
    }
}

/// Per-bridge pier footing/nosing from steady/unsteady flat arrays.
pub fn pier_attachments_user_for_bridge_index(
    footing_top_elevations: &Option<Vec<Vec<f64>>>,
    footing_widths: &Option<Vec<Vec<f64>>>,
    footing_bottom_elevations: &Option<Vec<Vec<f64>>>,
    nosing_lengths: &Option<Vec<Vec<f64>>>,
    nosing_widths: &Option<Vec<Vec<f64>>>,
    b_idx: usize,
) -> Option<PierAttachmentsUserInput> {
    let input = PierAttachmentsUserInput {
        footing_top_elevations: nested_bridge_row(footing_top_elevations, b_idx),
        footing_widths: nested_bridge_row(footing_widths, b_idx),
        footing_bottom_elevations: nested_bridge_row(footing_bottom_elevations, b_idx),
        nosing_lengths: nested_bridge_row(nosing_lengths, b_idx),
        nosing_widths: nested_bridge_row(nosing_widths, b_idx),
    };
    if input.footing_top_elevations.is_some()
        || input.footing_widths.is_some()
        || input.footing_bottom_elevations.is_some()
        || input.nosing_lengths.is_some()
        || input.nosing_widths.is_some()
    {
        Some(input)
    } else {
        None
    }
}

/// Pier attachments for standalone bridge / rating curve.
pub fn pier_attachments_from_rating_params(
    footing_top_elevations: &Option<Vec<f64>>,
    footing_widths: &Option<Vec<f64>>,
    footing_bottom_elevations: &Option<Vec<f64>>,
    nosing_lengths: &Option<Vec<f64>>,
    nosing_widths: &Option<Vec<f64>>,
) -> Option<PierAttachmentsUserInput> {
    let input = PierAttachmentsUserInput {
        footing_top_elevations: footing_top_elevations.clone(),
        footing_widths: footing_widths.clone(),
        footing_bottom_elevations: footing_bottom_elevations.clone(),
        nosing_lengths: nosing_lengths.clone(),
        nosing_widths: nosing_widths.clone(),
    };
    if input.footing_top_elevations.is_some()
        || input.footing_widths.is_some()
        || input.footing_bottom_elevations.is_some()
        || input.nosing_lengths.is_some()
        || input.nosing_widths.is_some()
    {
        Some(input)
    } else {
        None
    }
}

/// Sum opening-plane pier top widths at WSEL.
pub fn total_pier_flow_width_at_wsel_m(
    piers: &[ResolvedPier],
    wsel_m: f64,
    z_bed_m: f64,
    skew_cos: f64,
) -> f64 {
    piers
        .iter()
        .map(|p| p.flow_width_at_wsel_opening_m(wsel_m, z_bed_m, skew_cos))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tapered_trapezoid_area_matches_hand_calc() {
        let spec = PierWidthSpec::Tapered {
            top_width_perp_m: 1.0,
            bottom_width_perp_m: 2.0,
            z_base_m: 100.0,
            z_top_m: 104.0,
        };
        // Fully submerged 4 m: area = 0.5 * (1 + 2) * 4 = 6 m²
        let area = spec.submerged_area_m2(104.0, 100.0);
        assert!((area - 6.0).abs() < 1e-9, "area={area}");

        // Partial submergence 2 m: area = 0.5 * (2 + 1.5) * 2 = 3.5 m²
        let partial = spec.submerged_area_m2(102.0, 100.0);
        assert!((partial - 3.5).abs() < 1e-9, "partial={partial}");
    }

    #[test]
    fn constant_prism_matches_width_times_depth() {
        let spec = PierWidthSpec::Constant { width_perp_m: 1.5 };
        assert!((spec.submerged_area_m2(5.0, 2.0) - 4.5).abs() < 1e-9);
        assert!((spec.width_perp_at_wsel(5.0, 2.0) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn profile_piecewise_integrates_segments() {
        let spec = PierWidthSpec::Profile {
            elevations_m: vec![0.0, 2.0, 4.0],
            widths_perp_m: vec![2.0, 1.5, 1.0],
        };
        // 0→2: avg (2+1.5)/2 * 2 = 3.5; 2→4: avg (1.5+1)/2 * 2 = 2.5 → total 6
        assert!((spec.submerged_area_m2(4.0, 0.0) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn tapered_exceeds_rectangular_at_mid_depth() {
        let tapered = PierWidthSpec::Tapered {
            top_width_perp_m: 1.0,
            bottom_width_perp_m: 2.0,
            z_base_m: 0.0,
            z_top_m: 4.0,
        };
        let rectangular = PierWidthSpec::Constant { width_perp_m: 1.0 };
        let wsel = 2.0;
        assert!(tapered.submerged_area_m2(wsel, 0.0) > rectangular.submerged_area_m2(wsel, 0.0));
    }

    /// Linear taper (top=1, bottom=2) has the same mean width (1.5) as a constant prism.
    #[test]
    fn tapered_matches_mean_constant_area_when_fully_submerged() {
        let tapered = PierWidthSpec::Tapered {
            top_width_perp_m: 1.0,
            bottom_width_perp_m: 2.0,
            z_base_m: 0.0,
            z_top_m: 4.0,
        };
        let mean_constant = PierWidthSpec::Constant { width_perp_m: 1.5 };
        let wsel = 4.0;
        let a_taper = tapered.submerged_area_m2(wsel, 0.0);
        let a_mean = mean_constant.submerged_area_m2(wsel, 0.0);
        assert!((a_taper - 6.0).abs() < 1e-9, "taper full: {a_taper}");
        assert!((a_mean - 6.0).abs() < 1e-9, "mean constant full: {a_mean}");
        assert!((a_taper - a_mean).abs() < 1e-9);
    }

    #[test]
    fn tapered_exceeds_mean_constant_area_when_partly_submerged() {
        let tapered = PierWidthSpec::Tapered {
            top_width_perp_m: 1.0,
            bottom_width_perp_m: 2.0,
            z_base_m: 0.0,
            z_top_m: 4.0,
        };
        let mean_constant = PierWidthSpec::Constant { width_perp_m: 1.5 };
        let wsel = 2.5;
        let a_taper = tapered.submerged_area_m2(wsel, 0.0);
        let a_mean = mean_constant.submerged_area_m2(wsel, 0.0);
        // 0.5 * (2 + 1.375) * 2.5 = 4.21875 vs 1.5 * 2.5 = 3.75
        assert!((a_taper - 4.218_75).abs() < 1e-6, "taper partial: {a_taper}");
        assert!((a_mean - 3.75).abs() < 1e-6, "mean constant partial: {a_mean}");
        assert!(a_taper > a_mean);
    }

    #[test]
    fn tapered_same_mean_as_constant_equal_surface_width_at_mid_height() {
        let tapered = PierWidthSpec::Tapered {
            top_width_perp_m: 1.0,
            bottom_width_perp_m: 2.0,
            z_base_m: 0.0,
            z_top_m: 4.0,
        };
        let mean_constant = PierWidthSpec::Constant { width_perp_m: 1.5 };
        // Linear taper equals mean width only at mid-pier elevation (z = 2 m).
        let wsel = 2.0;
        let w_taper = tapered.width_perp_at_wsel(wsel, 0.0);
        let w_mean = mean_constant.width_perp_at_wsel(wsel, 0.0);
        assert!((w_taper - 1.5).abs() < 1e-9);
        assert!((w_taper - w_mean).abs() < 1e-9);
        // Below mid-height taper is wider at the water surface.
        assert!(tapered.width_perp_at_wsel(1.0, 0.0) > mean_constant.width_perp_at_wsel(1.0, 0.0));
    }

    #[test]
    fn pier_width_user_for_bridge_index_extracts_row() {
        let top = Some(vec![vec![1.0, 1.0]]);
        let bottom = Some(vec![vec![2.0, 2.0]]);
        let user = pier_width_user_for_bridge_index(
            &top,
            &bottom,
            &None,
            &None,
            &None,
            &None,
            0,
        )
        .expect("pier width user");
        assert_eq!(user.top_widths.as_deref(), Some([1.0, 1.0].as_ref()));
        assert_eq!(user.bottom_widths.as_deref(), Some([2.0, 2.0].as_ref()));
        assert!(pier_width_user_for_bridge_index(
            &top, &bottom, &None, &None, &None, &None, 1
        )
        .is_none());
    }

    #[test]
    fn pier_width_user_input_serde_round_trip() {
        let input = PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![2.0]),
            ..Default::default()
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: PierWidthUserInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.top_widths, input.top_widths);
    }

    #[test]
    fn opening_plane_area_applies_skew() {
        let piers = vec![ResolvedPier {
            station_m: 5.0,
            spec: PierWidthSpec::Constant { width_perp_m: 1.0 },
            nosing: None,
        }];
        let area = total_submerged_pier_area_m2(&piers, 3.0, 0.0, 0.5);
        assert!((area - 6.0).abs() < 1e-9);
    }

    #[test]
    fn resolve_pier_width_specs_legacy_constant() {
        let specs = resolve_pier_width_specs(1.5, &[5.0, 8.0], 0.0, &[4.0, 4.0], None, None);
        assert_eq!(specs.len(), 2);
        for p in &specs {
            match &p.spec {
                PierWidthSpec::Constant { width_perp_m } => {
                    assert!((width_perp_m - 1.5).abs() < 1e-9)
                }
                _ => panic!("expected constant legacy spec"),
            }
        }
    }

    #[test]
    fn resolve_pier_width_specs_tapered_pair() {
        let user = PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![2.0]),
            ..Default::default()
        };
        let specs = resolve_pier_width_specs(1.5, &[5.0], 0.0, &[4.0], Some(&user), None);
        match &specs[0].spec {
            PierWidthSpec::Tapered {
                top_width_perp_m,
                bottom_width_perp_m,
                z_top_m,
                z_base_m,
            } => {
                assert!((top_width_perp_m - 1.0).abs() < 1e-9);
                assert!((bottom_width_perp_m - 2.0).abs() < 1e-9);
                assert!((z_top_m - 4.0).abs() < 1e-9);
                assert!((z_base_m - 0.0).abs() < 1e-9);
            }
            _ => panic!("expected tapered spec"),
        }
    }

    #[test]
    fn resolve_pier_width_specs_profile_precedence_over_top_bottom() {
        let user = PierWidthUserInput {
            top_widths: Some(vec![99.0]),
            bottom_widths: Some(vec![99.0]),
            width_elevations: Some(vec![vec![0.0, 4.0]]),
            width_values: Some(vec![vec![2.0, 1.0]]),
            ..Default::default()
        };
        let specs = resolve_pier_width_specs(1.5, &[5.0], 0.0, &[4.0], Some(&user), None);
        match &specs[0].spec {
            PierWidthSpec::Profile {
                elevations_m,
                widths_perp_m,
            } => {
                assert_eq!(elevations_m, &vec![0.0, 4.0]);
                assert_eq!(widths_perp_m, &vec![2.0, 1.0]);
            }
            _ => panic!("profile should win over top/bottom pair"),
        }
    }

    #[test]
    fn resolve_pier_width_specs_custom_top_and_base_elevations() {
        let user = PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![2.0]),
            top_elevations: Some(vec![103.0]),
            base_elevations: Some(vec![100.0]),
            ..Default::default()
        };
        let specs = resolve_pier_width_specs(1.5, &[5.0], 99.0, &[104.0], Some(&user), None);
        match &specs[0].spec {
            PierWidthSpec::Tapered { z_top_m, z_base_m, .. } => {
                assert!((z_top_m - 103.0).abs() < 1e-9);
                assert!((z_base_m - 100.0).abs() < 1e-9);
            }
            _ => panic!("expected tapered with custom elevations"),
        }
    }

    #[test]
    fn resolve_pier_width_specs_mixed_piers_in_one_opening() {
        let user = PierWidthUserInput {
            width_elevations: Some(vec![vec![0.0, 4.0]]),
            width_values: Some(vec![vec![2.0, 1.0]]),
            ..Default::default()
        };
        let specs = resolve_pier_width_specs(1.0, &[3.0, 7.0], 0.0, &[4.0, 4.0], Some(&user), None);
        assert!(matches!(specs[0].spec, PierWidthSpec::Profile { .. }));
        match &specs[1].spec {
            PierWidthSpec::Constant { width_perp_m } => assert!((width_perp_m - 1.0).abs() < 1e-9),
            _ => panic!("second pier should use legacy constant width"),
        }
    }

    #[test]
    fn pier_width_user_from_rating_params_empty_is_none() {
        assert!(pier_width_user_from_rating_params(
            &None, &None, &None, &None, &None, &None
        )
        .is_none());
    }

    #[test]
    fn pier_width_user_from_rating_params_extracts_flat_arrays() {
        let user = pier_width_user_from_rating_params(
            &Some(vec![1.0]),
            &Some(vec![2.0]),
            &None,
            &None,
            &None,
            &None,
        )
        .expect("rating pier width user");
        assert_eq!(user.top_widths.as_deref(), Some([1.0].as_ref()));
        assert_eq!(user.bottom_widths.as_deref(), Some([2.0].as_ref()));
    }

    #[test]
    fn footing_adds_area_below_shaft_base() {
        let width_user = PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![2.0]),
            base_elevations: Some(vec![0.0]),
            ..Default::default()
        };
        let attachments = PierAttachmentsUserInput {
            footing_top_elevations: Some(vec![0.0]),
            footing_widths: Some(vec![3.0]),
            footing_bottom_elevations: Some(vec![-1.0]),
            ..Default::default()
        };
        let shaft_only =
            resolve_pier_width_specs(1.5, &[5.0], -1.0, &[4.0], Some(&width_user), None);
        let with_footing = resolve_pier_width_specs(
            1.5,
            &[5.0],
            -1.0,
            &[4.0],
            Some(&width_user),
            Some(&attachments),
        );
        let wsel = 2.0;
        let a_shaft = shaft_only[0].shaft_submerged_area_m2(wsel, -1.0);
        let a_total = with_footing[0].shaft_submerged_area_m2(wsel, -1.0);
        // Footing -1→0 widens below shaft base at z=0 (≈2.5 m² extra vs tapered shaft alone).
        assert!(a_total > a_shaft + 2.0, "footing={a_total} shaft={a_shaft}");
    }

    #[test]
    fn nosing_adds_submerged_area_and_flow_width() {
        let attachments = PierAttachmentsUserInput {
            nosing_lengths: Some(vec![0.5]),
            ..Default::default()
        };
        let piers = resolve_pier_width_specs(1.0, &[5.0], 0.0, &[4.0], None, Some(&attachments));
        let wsel = 2.0;
        let pier = &piers[0];
        assert!((pier.nosing_submerged_area_m2(wsel, 0.0) - 1.0).abs() < 1e-9);
        assert!((pier.submerged_area_m2(wsel, 0.0) - 3.0).abs() < 1e-9);
        let w_flow = pier.flow_width_at_wsel_opening_m(wsel, 0.0, 1.0);
        assert!((w_flow - 1.5).abs() < 1e-9);
    }

    #[test]
    fn pier_attachments_user_for_bridge_index_extracts_row() {
        let footing = Some(vec![vec![1.0], vec![2.0]]);
        let nosing = Some(vec![vec![0.5]]);
        let user = pier_attachments_user_for_bridge_index(
            &footing,
            &None,
            &None,
            &nosing,
            &None,
            0,
        )
        .expect("attachments user");
        assert_eq!(user.footing_top_elevations.as_deref(), Some([1.0].as_ref()));
        assert_eq!(user.nosing_lengths.as_deref(), Some([0.5].as_ref()));
        let user1 = pier_attachments_user_for_bridge_index(
            &footing, &None, &None, &nosing, &None, 1,
        )
        .expect("second bridge footing");
        assert_eq!(user1.footing_top_elevations.as_deref(), Some([2.0].as_ref()));
        assert!(user1.nosing_lengths.is_none());
        assert!(pier_attachments_user_for_bridge_index(
            &footing, &None, &None, &nosing, &None, 2,
        )
        .is_none());
    }

    #[test]
    fn pier_attachments_from_rating_params_extracts_flat_arrays() {
        let user = pier_attachments_from_rating_params(
            &Some(vec![1.0]),
            &Some(vec![3.0]),
            &None,
            &Some(vec![0.5]),
            &None,
        )
        .expect("rating attachments");
        assert_eq!(user.footing_top_elevations.as_deref(), Some([1.0].as_ref()));
        assert_eq!(user.footing_widths.as_deref(), Some([3.0].as_ref()));
        assert_eq!(user.nosing_lengths.as_deref(), Some([0.5].as_ref()));
        assert!(pier_attachments_from_rating_params(
            &None, &None, &None, &None, &None
        )
        .is_none());
    }

    #[test]
    fn pier_attachments_user_input_serde_round_trip() {
        let input = PierAttachmentsUserInput {
            footing_top_elevations: Some(vec![1.0]),
            footing_widths: Some(vec![3.0]),
            nosing_lengths: Some(vec![0.5]),
            ..Default::default()
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: PierAttachmentsUserInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.footing_widths, input.footing_widths);
        assert_eq!(back.nosing_lengths, input.nosing_lengths);
    }

    #[test]
    fn nosing_explicit_width_adds_submerged_area() {
        let attachments = PierAttachmentsUserInput {
            nosing_lengths: Some(vec![0.5]),
            nosing_widths: Some(vec![2.0]),
            ..Default::default()
        };
        let piers = resolve_pier_width_specs(1.0, &[5.0], 0.0, &[4.0], None, Some(&attachments));
        let wsel = 2.0;
        // 0.5 × 2.0 × 2.0 m wet depth = 2.0 m²
        assert!((piers[0].nosing_submerged_area_m2(wsel, 0.0) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn nosing_flow_width_applies_skew() {
        let attachments = PierAttachmentsUserInput {
            nosing_lengths: Some(vec![0.5]),
            ..Default::default()
        };
        let piers = resolve_pier_width_specs(1.0, &[5.0], 0.0, &[4.0], None, Some(&attachments));
        let wsel = 2.0;
        let w_normal = total_pier_flow_width_at_wsel_m(&piers, wsel, 0.0, 1.0);
        let w_skew = total_pier_flow_width_at_wsel_m(&piers, wsel, 0.0, 0.5);
        assert!((w_normal - 1.5).abs() < 1e-9);
        assert!((w_skew - 3.0).abs() < 1e-9);
    }

    #[test]
    fn footing_profile_wins_when_already_covers_footing_band() {
        let width_user = PierWidthUserInput {
            width_elevations: Some(vec![vec![-1.0, 0.0, 4.0]]),
            width_values: Some(vec![vec![3.0, 2.0, 1.0]]),
            ..Default::default()
        };
        let attachments = PierAttachmentsUserInput {
            footing_top_elevations: Some(vec![0.0]),
            footing_widths: Some(vec![99.0]),
            ..Default::default()
        };
        let specs = resolve_pier_width_specs(
            1.0,
            &[5.0],
            -1.0,
            &[4.0],
            Some(&width_user),
            Some(&attachments),
        );
        match &specs[0].spec {
            PierWidthSpec::Profile { widths_perp_m, .. } => {
                assert!((widths_perp_m[0] - 3.0).abs() < 1e-9);
                assert!(widths_perp_m[0] < 50.0);
            }
            _ => panic!("expected profile"),
        }
    }
}
