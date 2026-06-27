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

fn interpolate_opt_coeff(up: Option<f64>, down: Option<f64>, t: f64) -> Option<f64> {
    match (up, down) {
        (Some(u), Some(d)) => Some((1.0 - t) * u + t * d),
        (Some(u), None) => Some(u),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    }
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
            .map(|&st| (1.0 - t) * upstream.get_manning_n(st) + t * downstream.get_manning_n(st))
            .collect();
        let coeff_contraction =
            interpolate_opt_coeff(upstream.coeff_contraction, downstream.coeff_contraction, t);
        let coeff_expansion =
            interpolate_opt_coeff(upstream.coeff_expansion, downstream.coeff_expansion, t);
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
            coeff_contraction,
            coeff_expansion,
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
        .map(|&st| (1.0 - t) * upstream.get_manning_n(st) + t * downstream.get_manning_n(st))
        .collect();

    let coeff_contraction =
        interpolate_opt_coeff(upstream.coeff_contraction, downstream.coeff_contraction, t);
    let coeff_expansion =
        interpolate_opt_coeff(upstream.coeff_expansion, downstream.coeff_expansion, t);

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
        coeff_contraction,
        coeff_expansion,
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
        let (table, z) =
            interpolate_geometry_table(table_up, z_up, table_down, z_down, t, num_slices);
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
            coeff_contraction: None,
            coeff_expansion: None,
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
        up.ineffective_flow_areas =
            Some(IneffectiveFlowAreas::from_block_pairs(&[], &[], &[8.0], &[3.0]).unwrap());
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
    fn interior_blocked_obstruction_matches_upstream_no_gap() {
        use crate::geometry::geometry_row_at_elevation;
        use crate::geometry::BlockedObstruction;

        let block = vec![BlockedObstruction {
            stations: vec![8.0, 12.0],
            elevations: vec![1.5, 1.5],
        }];
        let mut up = rect(200.0, 0.0, 20.0);
        let mut down = rect(0.0, 0.0, 20.0);
        up.blocked_obstructions = Some(block.clone());
        down.blocked_obstructions = Some(block);

        let table_up = up.generate_lookup_table(40);
        let table_down = down.generate_lookup_table(40);
        let (table_mid, _, xs) = densify_interior_node(
            &up,
            &down,
            &table_up,
            min_bed(&up),
            &table_down,
            min_bed(&down),
            100.0,
            0.5,
            40,
            DensifyReachModifierPolicy::Upstream,
        );
        let interior = xs.expect("upstream policy materializes interior xs");
        assert_eq!(
            interior.blocked_obstructions.as_ref().unwrap()[0].stations,
            up.blocked_obstructions.as_ref().unwrap()[0].stations
        );

        let wsel = 2.0;
        let row_up = geometry_row_at_elevation(&table_up, Some(&up), wsel, None, None);
        let row_mid = geometry_row_at_elevation(&table_mid, Some(&interior), wsel, None, None);
        assert!(
            (row_mid.active_area - row_up.active_area).abs() < 1e-3,
            "interior active area {} should match upstream {} (no obstruction gap)",
            row_mid.active_area,
            row_up.active_area
        );
        assert!((row_mid.area - row_up.area).abs() < 1e-3);
    }

    #[test]
    fn upstream_only_blocked_copied_to_interior_no_gap() {
        use crate::geometry::geometry_row_at_elevation;
        use crate::geometry::BlockedObstruction;

        let mut up = rect(200.0, 0.0, 20.0);
        up.blocked_obstructions = Some(vec![BlockedObstruction {
            stations: vec![8.0, 12.0],
            elevations: vec![1.5, 1.5],
        }]);
        let down = rect(0.0, 0.0, 20.0);

        let table_up = up.generate_lookup_table(40);
        let table_down = down.generate_lookup_table(40);
        let (table_mid, _, xs) = densify_interior_node(
            &up,
            &down,
            &table_up,
            min_bed(&up),
            &table_down,
            min_bed(&down),
            100.0,
            0.5,
            40,
            DensifyReachModifierPolicy::Upstream,
        );
        let interior = xs.expect("interior xs");
        let wsel = 2.0;
        let row_up = geometry_row_at_elevation(&table_up, Some(&up), wsel, None, None);
        let row_down = geometry_row_at_elevation(&table_down, Some(&down), wsel, None, None);
        let row_mid = geometry_row_at_elevation(&table_mid, Some(&interior), wsel, None, None);
        assert!(
            (row_mid.active_area - row_up.active_area).abs() < 1e-3,
            "interior must inherit upstream blockage, not open downstream"
        );
        assert!(
            row_mid.active_area + 1e-3 < row_down.active_area,
            "downstream is unobstructed; interior should stay blocked"
        );
    }

    #[test]
    fn none_policy_table_blend_no_obstruction_gap_between_identical_parents() {
        use crate::geometry::BlockedObstruction;

        let block = vec![BlockedObstruction {
            stations: vec![8.0, 12.0],
            elevations: vec![1.5, 1.5],
        }];
        let mut up = rect(200.0, 0.0, 20.0);
        let mut down = rect(0.0, 0.0, 20.0);
        up.blocked_obstructions = Some(block.clone());
        down.blocked_obstructions = Some(block);
        let open = rect(200.0, 0.0, 20.0);

        let table_up = up.generate_lookup_table(40);
        let table_down = down.generate_lookup_table(40);
        let (table_mid, _, xs) = densify_interior_node(
            &up,
            &down,
            &table_up,
            min_bed(&up),
            &table_down,
            min_bed(&down),
            100.0,
            0.5,
            40,
            DensifyReachModifierPolicy::None,
        );
        assert!(xs.is_none());

        let wsel = 2.0;
        let row_up = table_up.interpolate(wsel);
        let row_mid = table_mid.interpolate(wsel);
        let row_open = open.generate_lookup_table(40).interpolate(wsel);
        assert!(
            (row_mid.area - row_up.area).abs() < 1e-4,
            "blended table should match identical blocked parents"
        );
        assert!(
            row_mid.area < row_open.area - 0.5,
            "interior must not revert to unobstructed area between blocked user XS"
        );
    }

    #[test]
    fn interior_ineffective_conveyance_matches_upstream_no_gap() {
        use crate::geometry::geometry_row_at_elevation;

        let mut up = rect(200.0, 0.0, 20.0);
        up.ineffective_flow_areas =
            Some(IneffectiveFlowAreas::from_block_pairs(&[], &[], &[18.0], &[3.0]).unwrap());
        let mut down = rect(0.0, 0.0, 20.0);
        down.ineffective_flow_areas = up.ineffective_flow_areas.clone();

        let table_up = up.generate_lookup_table(40);
        let table_down = down.generate_lookup_table(40);
        let (table_mid, _, xs) = densify_interior_node(
            &up,
            &down,
            &table_up,
            min_bed(&up),
            &table_down,
            min_bed(&down),
            100.0,
            0.5,
            40,
            DensifyReachModifierPolicy::Upstream,
        );
        let interior = xs.expect("interior xs");
        let wsel = 2.5;
        let row_up = geometry_row_at_elevation(&table_up, Some(&up), wsel, None, None);
        let row_mid = geometry_row_at_elevation(&table_mid, Some(&interior), wsel, None, None);
        assert!(
            (row_mid.conveyance - row_up.conveyance).abs() < 1e-2,
            "ineffective conveyance at interior {} vs upstream {}",
            row_mid.conveyance,
            row_up.conveyance
        );
    }

    #[test]
    fn densify_reach_modifier_policy_from_u8_and_option() {
        assert_eq!(
            DensifyReachModifierPolicy::from_u8(1),
            DensifyReachModifierPolicy::Upstream
        );
        assert_eq!(
            DensifyReachModifierPolicy::from_u8(2),
            DensifyReachModifierPolicy::Downstream
        );
        assert_eq!(
            DensifyReachModifierPolicy::from_u8(3),
            DensifyReachModifierPolicy::Nearest
        );
        assert_eq!(
            DensifyReachModifierPolicy::from_u8(99),
            DensifyReachModifierPolicy::None
        );
        assert_eq!(
            DensifyReachModifierPolicy::from_option(None),
            DensifyReachModifierPolicy::None
        );
        assert_eq!(
            DensifyReachModifierPolicy::from_option(Some(2)),
            DensifyReachModifierPolicy::Downstream
        );
    }

    #[test]
    fn downstream_policy_copies_modifiers_from_downstream_parent() {
        use crate::geometry::BlockedObstruction;

        let up = rect(200.0, 0.0, 20.0);
        let mut down = rect(0.0, 0.0, 20.0);
        down.blocked_obstructions = Some(vec![BlockedObstruction {
            stations: vec![4.0, 6.0],
            elevations: vec![1.0, 1.0],
        }]);
        let table_up = up.generate_lookup_table(30);
        let table_down = down.generate_lookup_table(30);
        let (_, _, xs) = densify_interior_node(
            &up,
            &down,
            &table_up,
            min_bed(&up),
            &table_down,
            min_bed(&down),
            100.0,
            0.5,
            30,
            DensifyReachModifierPolicy::Downstream,
        );
        let interior = xs.expect("downstream policy materializes xs");
        assert_eq!(
            interior.blocked_obstructions.as_ref().unwrap()[0].stations,
            down.blocked_obstructions.as_ref().unwrap()[0].stations
        );
    }

    #[test]
    fn nearest_policy_uses_downstream_when_t_above_half() {
        use crate::geometry::IneffectiveFlowAreas;

        let mut up = rect(200.0, 0.0, 20.0);
        up.ineffective_flow_areas =
            Some(IneffectiveFlowAreas::from_block_pairs(&[], &[], &[18.0], &[3.0]).unwrap());
        let mut down = rect(0.0, 0.0, 20.0);
        down.ineffective_flow_areas =
            Some(IneffectiveFlowAreas::from_block_pairs(&[], &[], &[2.0], &[3.0]).unwrap());
        let table_up = up.generate_lookup_table(30);
        let table_down = down.generate_lookup_table(30);
        let (_, _, xs_near) = densify_interior_node(
            &up,
            &down,
            &table_up,
            min_bed(&up),
            &table_down,
            min_bed(&down),
            40.0,
            0.6,
            30,
            DensifyReachModifierPolicy::Nearest,
        );
        let interior = xs_near.expect("nearest policy xs");
        let right = interior
            .ineffective_flow_areas
            .as_ref()
            .and_then(|i| i.right_blocks.first())
            .expect("downstream ineffective");
        assert!((right.station - 2.0).abs() < 1e-9);
    }

    #[test]
    fn upstream_policy_copies_guide_banks() {
        use crate::geometry::{GuideBankPolyline, GuideBanks};

        let mut up = rect(200.0, 0.0, 20.0);
        up.guide_banks = Some(GuideBanks {
            left_polylines: vec![GuideBankPolyline {
                stations: vec![1.0, 5.0],
                elevations: vec![2.0, 2.5],
            }],
            ..Default::default()
        });
        let down = rect(0.0, 0.0, 20.0);
        let table_up = up.generate_lookup_table(30);
        let table_down = down.generate_lookup_table(30);
        let (_, _, xs) = densify_interior_node(
            &up,
            &down,
            &table_up,
            min_bed(&up),
            &table_down,
            min_bed(&down),
            100.0,
            0.5,
            30,
            DensifyReachModifierPolicy::Upstream,
        );
        let interior = xs.expect("guide banks copied");
        assert!(interior
            .guide_banks
            .as_ref()
            .is_some_and(|g| g.is_configured()));
    }

    #[test]
    fn interpolate_cross_section_merged_lateral_grid() {
        let up = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 1.0, 1.0, 5.0],
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
        let down = CrossSection {
            station: 0.0,
            x: vec![5.0, 5.0, 15.0, 15.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.04],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            coeff_contraction: None,
            coeff_expansion: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let mid = interpolate_cross_section(&up, &down, 0.5, 50.0);
        assert!(
            mid.x.len() >= 4,
            "merged lateral grid should union parent stations"
        );
        assert!((min_bed(&mid) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn none_policy_leaves_modifiers_empty_on_synthetic_xs() {
        let mut up = rect(100.0, 0.0, 10.0);
        up.ineffective_flow_areas =
            Some(IneffectiveFlowAreas::from_block_pairs(&[], &[], &[8.0], &[3.0]).unwrap());
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

    #[test]
    fn elevation_at_x_edge_cases_and_opt_coeff_branches() {
        let one_pt = CrossSection {
            station: 0.0,
            x: vec![0.0],
            y: vec![1.0],
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
        assert!(elevation_at_x(&one_pt, 0.0).is_none());

        let flat = CrossSection {
            station: 0.0,
            x: vec![0.0, 5.0],
            y: vec![2.0, 2.0],
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
        assert!((elevation_at_x(&flat, 2.5).unwrap() - 2.0).abs() < 1e-9);
        assert!((elevation_at_x(&flat, 0.0).unwrap() - 2.0).abs() < 1e-9);
        assert!((elevation_at_x(&flat, 5.0).unwrap() - 2.0).abs() < 1e-9);

        assert!((interpolate_opt_coeff(Some(0.1), Some(0.3), 0.5).unwrap() - 0.2).abs() < 1e-9);
        assert_eq!(interpolate_opt_coeff(Some(0.1), None, 0.5), Some(0.1));
        assert_eq!(interpolate_opt_coeff(None, Some(0.3), 0.5), Some(0.3));
        assert_eq!(interpolate_opt_coeff(None, None, 0.5), None);
    }

    #[test]
    fn apply_reach_modifier_policy_none_is_noop() {
        let up = rect(100.0, 0.0, 10.0);
        let down = rect(0.0, 0.0, 10.0);
        let mut synthetic = rect(50.0, 0.0, 10.0);
        synthetic.ineffective_flow_areas =
            Some(IneffectiveFlowAreas::from_block_pairs(&[], &[], &[8.0], &[3.0]).unwrap());
        apply_reach_modifier_policy(
            &mut synthetic,
            &up,
            &down,
            0.5,
            DensifyReachModifierPolicy::None,
        );
        assert!(synthetic.ineffective_flow_areas.is_some());
    }
}
