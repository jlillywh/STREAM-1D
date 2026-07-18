//! HEC-RAS verification / regression test for Bald Eagle reach.
//!
//! Benchmarks: `verification/fixtures/baldeagle_regression.json`

use std::collections::HashMap;
use stream1d::solvers::{solve_steady, SteadyInputs};

#[allow(dead_code)]
#[derive(serde::Deserialize, Debug)]
struct HecrasProperties {
    wsel: Option<f64>,
    crit_wsel: Option<f64>,
    eg_elev: Option<f64>,
    eg_slope: Option<f64>,
    velocity: Option<f64>,
    area: Option<f64>,
    top_width: Option<f64>,
    froude: Option<f64>,
}

#[derive(serde::Deserialize)]
struct BaldEagleFixture {
    payload: SteadyInputs,
    rms: Vec<f64>,
    hecras_results: HashMap<String, HecrasProperties>,
}

#[test]
fn test_baldeagle_regression() {
    let fixture_json = include_str!("../verification/fixtures/baldeagle_regression.json");
    let mut fixture: BaldEagleFixture =
        serde_json::from_str(fixture_json).expect("Failed to deserialize Bald Eagle fixture");

    // Disable automatic interpolation for direct comparison
    fixture.payload.max_spacing = None;

    let result = solve_steady(&fixture.payload);



    assert_eq!(
        result.wsel.len(),
        fixture.payload.cross_sections.len(),
        "Result WSEL length must match input cross-sections count"
    );

    // Let's print detailed geometry table diagnostics for the downstream-most two cross sections
    let total_xs = fixture.payload.cross_sections.len();
    let idx_177 = total_xs - 1; // Downstream boundary (RM 659.94)
    let idx_176 = total_xs - 2; // First station upstream (RM 1212.86)

    let xs_177 = &fixture.payload.cross_sections[idx_177];
    let xs_176 = &fixture.payload.cross_sections[idx_176];

    let table_177 = xs_177.to_metric().generate_lookup_table(100);
    let table_176 = xs_176.to_metric().generate_lookup_table(100);

    let ft_to_m = stream1d::utils::FT_TO_M;
    let ft2_to_m2 = ft_to_m * ft_to_m;
    let cfs_to_cms = stream1d::utils::CFS_TO_CMS;

    println!("\n=== GEOMETRY & CONVEYANCE TABLE DIAGNOSTICS ===");
    
    // Station 177 (RM 659.94)
    let wsel_177_calc = result.wsel[idx_177];
    let row_177 = table_177.interpolate(wsel_177_calc * ft_to_m);
    let s1d_area_177_ft = row_177.area / ft2_to_m2;
    let s1d_conv_177_us = row_177.conveyance / (cfs_to_cms / ft_to_m.powf(2.0/3.0));
    println!("Station 177 (RM 659.94) - Solved: {:.4} ft | HEC: 543.35 ft", wsel_177_calc);
    println!("  S1D Area: {:.2} sq ft | HEC Area: 4433.15 sq ft", s1d_area_177_ft);
    println!("  S1D Conv: {:.0} | HEC Conv: {:.0}", s1d_conv_177_us, 20000.0 / (0.001001f64).sqrt());

    // Station 176 (RM 1212.86)
    let wsel_176_calc = result.wsel[idx_176];
    let row_176 = table_176.interpolate(wsel_176_calc * ft_to_m);
    let s1d_area_176_ft = row_176.area / ft2_to_m2;
    let s1d_conv_176_us = row_176.conveyance / (cfs_to_cms / ft_to_m.powf(2.0/3.0));
    println!("Station 176 (RM 1212.86) - Solved: {:.4} ft | HEC: 543.80 ft", wsel_176_calc);
    println!("  S1D Area: {:.2} sq ft | HEC Area: 3571.11 sq ft", s1d_area_176_ft);
    println!("  S1D Conv: {:.0} | HEC Conv: {:.0}", s1d_conv_176_us, 20000.0 / (0.000921f64).sqrt());

    println!("{}", "=".repeat(50));

    let mut pass_count = 0;
    let mut fail_count = 0;
    let max_allowed_difference = 4.5;

    let mut discrepancies = Vec::new();
    let mut differences = Vec::new();
    let mut sum_abs_diff = 0.0;
    let mut max_abs_diff = 0.0;

    for (i, _xs) in fixture.payload.cross_sections.iter().enumerate() {
        let calc_val = result.wsel[i];
        let rm = fixture.rms[i];

        // Find the matching HEC-RAS value from the CSV results
        let expected_props = fixture
            .hecras_results
            .iter()
            .find_map(|(k, v)| {
                let k_f64: f64 = k.parse().ok()?;
                if (k_f64 - rm).abs() < 0.1 {
                    Some(v)
                } else {
                    None
                }
            });

        if let Some(expected) = expected_props {
            let expected_wsel = expected.wsel.unwrap_or(0.0);
            let diff = calc_val - expected_wsel;
            let abs_diff = diff.abs();
            differences.push(abs_diff);
            sum_abs_diff += abs_diff;
            if abs_diff > max_abs_diff {
                max_abs_diff = abs_diff;
            }

            if abs_diff <= max_allowed_difference {
                pass_count += 1;
            } else {
                fail_count += 1;
            }

            discrepancies.push((i, rm, calc_val, expected_wsel, diff, abs_diff));

            if i >= total_xs - 10 {
                println!("Downstream Station {:<3} | RM: {:<9.2} | Solved: {:.4} | HEC: {:.4} | Diff: {:.4} | Status: {}",
                    i, rm, calc_val, expected_wsel, diff,
                    if abs_diff <= max_allowed_difference { "PASS" } else { "FAIL" }
                );
            }
        }
    }

    // Sort discrepancies descending by absolute difference
    discrepancies.sort_by(|a, b| b.5.partial_cmp(&a.5).unwrap());


    println!("\n=== TOP 10 LARGEST PROFILE DISCREPANCIES ===");
    for idx in 0..10.min(discrepancies.len()) {
        let (i, rm, calc, expected, diff, _) = discrepancies[idx];
        println!("Rank {:<2} | Station {:<3} | RM: {:<9.2} | Solved: {:.4} | HEC: {:.4} | Diff: {:.4}",
            idx + 1, i, rm, calc, expected, diff
        );
    }

    let n = differences.len() as f64;
    let mean_abs_diff = if n > 0.0 { sum_abs_diff / n } else { 0.0 };
    let count_01 = differences.iter().filter(|&&d| d <= 0.01).count();
    let count_03 = differences.iter().filter(|&&d| d <= 0.03).count();
    let count_05 = differences.iter().filter(|&&d| d <= 0.05).count();
    let count_10 = differences.iter().filter(|&&d| d <= 0.10).count();
    let count_above_10 = differences.iter().filter(|&&d| d > 0.10).count();

    println!("\n=== PROFILE DIFFERENCE SUMMARY STATISTICS ===");
    println!("Max Absolute WSEL Difference : {:.4} ft", max_abs_diff);
    println!("Mean Absolute WSEL Difference: {:.4} ft", mean_abs_diff);
    println!("Distribution of differences:");
    println!("  <= 0.01 ft: {} / {}", count_01, differences.len());
    println!("  <= 0.03 ft: {} / {}", count_03, differences.len());
    println!("  <= 0.05 ft: {} / {}", count_05, differences.len());
    println!("  <= 0.10 ft: {} / {}", count_10, differences.len());
    println!("   > 0.10 ft: {} / {}", count_above_10, differences.len());
    println!("{}", "=".repeat(45));

    println!(
        "Bald Eagle regression complete. Passed: {}, Failed: {} (Tolerance: ±{} ft)",
        pass_count, fail_count, max_allowed_difference
    );

    assert_eq!(
        fail_count, 0,
        "Bald Eagle regression test failed: {} cross sections exceeded tolerance of ±{} ft",
        fail_count, max_allowed_difference
    );
}
