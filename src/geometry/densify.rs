//! Reach densification: interpolated interior cross sections and reach modifier inheritance.

use crate::geometry::processor::{interpolate_geometry_table, CrossSection, GeometryTable};

/// How reach modifiers apply on `max_spacing` interior nodes (not BU/BD layout inserts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DensifyReachModifierPolicy {
    /// Interpolated table only; steady uses `densified_xs = None` (blocked hydraulics via table blend).
    #[default]
    None = 0,
    /// Copy ineffective / blocked / guide banks from upstream user section onto synthetic cut.
    Upstream = 1,
    /// Copy from downstream user section.
    Downstream = 2,
    /// Copy from closer parent by interpolation factor `t` (upstream when `t <= 0.5`).
    Nearest = 3,
}

impl DensifyReachModifierPolicy {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Upstream,
            2 => Self::Downstream,
            3 => Self::Nearest,
            _ => Self::None,
        }
    }

    pub fn from_option(v: Option<u8>) -> Self {
        v.map(Self::from_u8).unwrap_or_default()
    }
}

fn merged_lateral_stations(a: &CrossSection, b: &CrossSection) -> Vec<f64> {
    let mut xs: Vec<f64> = a.x.iter().chain(b.x.iter()).copied().collect();
    xs.sort_by(|l, r| l.partial_cmp(r).unwrap_or(std::cmp::Ordering::Equal));
    xs.dedup_by(|l, r| (*l - *r).abs() < 1e-6);
    xs
}

fn elevation_at_x(xs: &CrossSection, x_target: f64) -> Option<f64> {
    let n_pts = xs.x.len();
    if n_pts < 2 {
        return None;
    }
    if x_target <= xs.x[0] {
        return Some(xs.y[0]);
    }
    if x_target >= xs.x[n_pts - 1] {
        return Some(xs.y[n_pts - 1]);
    }
    for i in 0..n_pts - 1 {
        let x1 = xs.x[i];
        let x2 = xs.x[i + 1];
        if x_target >= x1 && x_target <= x2 {
            let dx = x2 - x1;
            if dx.abs() < 1e-9 {
                return Some(xs.y[i]);
            }
            let frac = (x_target - x1) / dx;
            return Some(xs.y[i] + frac * (xs.y[i + 1] - xs.y[i]));
        }
    }
    None
}

fn bed_elevation_at_x(xs: &CrossSection, x_target: f64) -> f64 {
    let tol = 1e-6;
    let mut at_station = false;
    let mut local_min = f64::INFINITY;
    for (&x, &y) in xs.x.iter().zip(xs.y.iter()) {
        if (x - x_target).abs() < tol {
            at_station = true;
            local_min = local_min.min(y);
        }
    }
    if at_station {
        return local_min;
    }
    elevation_at_x(xs, x_target).unwrap_or_else(|| min_bed(xs))
}

fn min_bed(xs: &CrossSection) -> f64 {
    xs.y.iter().cloned().fold(f64::INFINITY, f64::min)
}

/// Linearly interpolate channel polyline and Manning's n between two user cross sections.
/// Reach modifiers are omitted; use [`apply_reach_modifier_policy`].
pub fn interpolate_cross_section(
    upstream: &CrossSection,
    downstream: &CrossSection,
    t: f64,
    station: f64,
) -> CrossSection {
    let t = t.clamp(0.0, 1.0);

    if upstream.x.len() == downstream.x.len()
        && upstream.x.len() >= 2
        && upstream
            .x
            .iter()
            .zip(downstream.x.iter())
            .all(|(a, b)| (a - b).abs() < 1e-6)
    {
        let y: Vec<f64> = upstream
            .y
            .iter()
            .zip(downstream.y.iter())
            .map(|(yu, yd)| (1.0 - t) * yu + t * yd)
            .collect();
        let mut n_breaks: Vec<f64> = upstream
            .n_stations
            .iter()
            .chain(downstream.n_stations.iter())
            .copied()
            .collect();
        n_breaks.sort_by(|l, r| l.partial_cmp(r).unwrap_or(std::cmp::Ordering::Equal));
        n_breaks.dedup_by(|l, r| (*l - *r).abs() < 1e-6);
        let n_values: Vec<f64> = n_breaks
            .iter()
            .map(|&st| {
                (1.0 - t) * upstream.get_manning_n(st) + t * downstream.get_manning_n(st)
            })
            .collect();
        return CrossSection {
            station,
            x: upstream.x.clone(),
            y,
            n_stations: n_breaks,
            n_values,
            unit_system: upstream.unit_system,
            is_overbank: upstream.is_overbank.clone(),
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
    }

    let x = merged_lateral_stations(upstream, downstream);
    let y: Vec<f64> = x
        .iter()
        .map(|&xi| {
            let yu = bed_elevation_at_x(upstream, xi);
            let yd = bed_elevation_at_x(downstream, xi);
            (1.0 - t) * yu + t * yd
        })
        .collect();

    let mut n_breaks: Vec<f64> = upstream
        .n_stations
        .iter()
        .chain(downstream.n_stations.iter())
        .copied()
        .collect();
    n_breaks.sort_by(|l, r| l.partial_cmp(r).unwrap_or(std::cmp::Ordering::Equal));
    n_breaks.dedup_by(|l, r| (*l - *r).abs() < 1e-6);
    let n_values: Vec<f64> = n_breaks
        .iter()
        .map(|&st| {
            (1.0 - t) * upstream.get_manning_n(st) + t * downstream.get_manning_n(st)
        })
        .collect();

    CrossSection {
        station,
        x,
        y,
        n_stations: n_breaks,
        n_values,
        unit_system: upstream.unit_system,
        is_overbank: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    }
}

fn modifier_source<'a>(
    upstream: &'a CrossSection,
    downstream: &'a CrossSection,
    t: f64,
    policy: DensifyReachModifierPolicy,
) -> &'a CrossSection {
    match policy {
        DensifyReachModifierPolicy::Downstream => downstream,
        DensifyReachModifierPolicy::Nearest if t > 0.5 => downstream,
        _ => upstream,
    }
}

/// Copy reach ineffective, blocked, and guide-bank modifiers from `source` onto `dest`.
pub fn copy_reach_modifiers(dest: &mut CrossSection, source: &CrossSection) {
    dest.ineffective_flow_areas = source.ineffective_flow_areas.clone();
    dest.blocked_obstructions = source.blocked_obstructions.clone();
    dest.guide_banks = source.guide_banks.clone();
}

/// Apply densification modifier policy to a synthetic interior cut.
pub fn apply_reach_modifier_policy(
    synthetic: &mut CrossSection,
    upstream: &CrossSection,
    downstream: &CrossSection,
    t: f64,
    policy: DensifyReachModifierPolicy,
) {
    if policy == DensifyReachModifierPolicy::None {
        return;
    }
    let src = modifier_source(upstream, downstream, t, policy);
    copy_reach_modifiers(synthetic, src);
}

/// Build one interior densified node between two user sections.
pub fn densify_interior_node(
    upstream: &CrossSection,
    downstream: &CrossSection,
    table_up: &GeometryTable,
    z_up: f64,
    table_down: &GeometryTable,
    z_down: f64,
    station: f64,
    t: f64,
    num_slices: usize,
    policy: DensifyReachModifierPolicy,
) -> (GeometryTable, f64, Option<CrossSection>) {
    if policy == DensifyReachModifierPolicy::None {
        let (table, z) = interpolate_geometry_table(table_up, z_up, table_down, z_down, t, num_slices);
        return (table, z, None);
    }

    let mut synthetic = interpolate_cross_section(upstream, downstream, t, station);
    apply_reach_modifier_policy(&mut synthetic, upstream, downstream, t, policy);
    let z = min_bed(&synthetic);
    let table = synthetic.generate_lookup_table(num_slices);
    (table, z, Some(synthetic))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::IneffectiveFlowAreas;
    use crate::utils::UnitSystem;

    fn rect(station: f64, z: f64, width: f64) -> CrossSection {
        CrossSection {
            station,
            x: vec![0.0, 0.0, width, width],
            y: vec![5.0 + z, z, z, 5.0 + z],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        }
    }

    #[test]
    fn interpolate_cross_section_blends_bed_elevation() {
        let up = rect(100.0, 1.0, 10.0);
        let down = rect(0.0, 0.0, 10.0);
        let mid = interpolate_cross_section(&up, &down, 0.5, 50.0);
        assert!((mid.station - 50.0).abs() < 1e-9);
        assert!((min_bed(&mid) - 0.5).abs() < 1e-6);
        assert!(mid.ineffective_flow_areas.is_none());
    }

    #[test]
    fn upstream_policy_copies_ineffective_and_blocked() {
        use crate::geometry::BlockedObstruction;

        let mut up = rect(100.0, 0.0, 10.0);
        up.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[], &[], &[8.0], &[3.0]).unwrap(),
        );
        up.blocked_obstructions = Some(vec![BlockedObstruction {
            stations: vec![2.0, 8.0],
            elevations: vec![1.0, 1.0],
        }]);
        let down = rect(0.0, 0.0, 20.0);

        let table_up = up.generate_lookup_table(20);
        let table_down = down.generate_lookup_table(20);
        let z_up = min_bed(&up);
        let z_down = min_bed(&down);

        let (_, _, xs) = densify_interior_node(
            &up,
            &down,
            &table_up,
            z_up,
            &table_down,
            z_down,
            50.0,
            0.5,
            30,
            DensifyReachModifierPolicy::Upstream,
        );
        let xs = xs.expect("upstream policy materializes xs");
        assert!(xs.ineffective_flow_areas.is_some());
        assert!(xs.blocked_obstructions.is_some());
        assert_eq!(
            xs.blocked_obstructions.as_ref().unwrap()[0].stations,
            up.blocked_obstructions.as_ref().unwrap()[0].stations
        );
    }

    #[test]
    fn none_policy_leaves_modifiers_empty_on_synthetic_xs() {
        let mut up = rect(100.0, 0.0, 10.0);
        up.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[], &[], &[8.0], &[3.0]).unwrap(),
        );
        let down = rect(0.0, 0.0, 10.0);
        let table_up = up.generate_lookup_table(20);
        let table_down = down.generate_lookup_table(20);

        let (_, _, xs) = densify_interior_node(
            &up,
            &down,
            &table_up,
            min_bed(&up),
            &table_down,
            min_bed(&down),
            50.0,
            0.5,
            30,
            DensifyReachModifierPolicy::None,
        );
        assert!(xs.is_none());
    }
}
