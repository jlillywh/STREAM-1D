//! Bi-directional bridge rating curve: Q ∈ [-Qmax, +Qmax] (API v31).
//!
//! Benchmark: `verification/fixtures/bridge_reverse_flow_rating.json`

use stream1d::solvers::bridge::{
    compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams,
};
use stream1d::utils::UnitSystem;

#[derive(serde::Deserialize)]
struct BenchmarkFile {
    tolerance_head_loss_m: f64,
    q_max_cms: f64,
    tw_wsel_m: f64,
    cases: Vec<BenchmarkCase>,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct BenchmarkCase {
    name: String,
    #[allow(dead_code)]
    notes: String,
    q_values_cms: Vec<f64>,
    low_chord_m: f64,
    high_chord_m: f64,
    channel_width_m: f64,
    manning_n: f64,
    low_flow_method: i32,
    expected_head_loss_at_abs_q20_cms: f64,
}

fn rating_inputs(case: &BenchmarkCase, tw_wsel_m: f64) -> BridgeRatingCurveInputs {
    let mut bridge = BridgeSolveParams::default();
    bridge.low_chord = case.low_chord_m;
    bridge.high_chord = case.high_chord_m;
    bridge.z_down = 0.0;
    bridge.z_up = 0.0;
    bridge.tw_wsel = tw_wsel_m;
    bridge.units = UnitSystem::Metric;
    bridge.low_flow_method = case.low_flow_method;
    bridge.channel_width = case.channel_width_m;
    bridge.manning_n = case.manning_n;
    BridgeRatingCurveInputs {
        q_values: case.q_values_cms.clone(),
        bridge,
    }
}

fn head_loss_at_q(curve: &stream1d::solvers::bridge::BridgeRatingCurveResult, q: f64) -> f64 {
    let idx = curve
        .q
        .iter()
        .position(|&sample| (sample - q).abs() < 1e-6)
        .unwrap_or_else(|| panic!("missing Q={q} in rating curve"));
    curve.head_losses[idx]
}

#[test]
fn bridge_reverse_flow_rating_bidirectional_sweep() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/verification/fixtures/bridge_reverse_flow_rating.json"
    );
    let file: BenchmarkFile =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read fixture")).expect("parse");

    for case in &file.cases {
        let curve = compute_bridge_rating_curve(&rating_inputs(case, file.tw_wsel_m));
        assert_eq!(
            curve.q.len(),
            case.q_values_cms.len(),
            "{}: all non-zero samples returned",
            case.name
        );
        assert!(
            curve.q.iter().all(|q| q.abs() <= file.q_max_cms + 1e-6),
            "{}: samples within ±Qmax",
            case.name
        );

        for i in 0..curve.q.len() {
            assert!(
                curve.wsel[i] > curve.wsel_down[i],
                "{}: HW > TW at q={}",
                case.name,
                curve.q[i]
            );
            assert!(
                (curve.wsel[i] - curve.wsel_down[i] - curve.head_losses[i]).abs() < 1e-4,
                "{}: head_loss consistency at q={}",
                case.name,
                curve.q[i]
            );
        }

        let mut pos: Vec<usize> = (0..curve.q.len()).filter(|&i| curve.q[i] > 0.0).collect();
        pos.sort_by(|&a, &b| curve.q[a].partial_cmp(&curve.q[b]).unwrap());
        for w in pos.windows(2) {
            assert!(
                curve.wsel[w[1]] > curve.wsel[w[0]],
                "{}: forward monotonicity",
                case.name
            );
        }

        let mut neg: Vec<usize> = (0..curve.q.len()).filter(|&i| curve.q[i] < 0.0).collect();
        neg.sort_by(|&a, &b| curve.q[b].partial_cmp(&curve.q[a]).unwrap());
        for w in neg.windows(2) {
            assert!(
                curve.wsel[w[1]] > curve.wsel[w[0]],
                "{}: reverse monotonicity in |Q|",
                case.name
            );
        }

        let hl_pos_20 = head_loss_at_q(&curve, 20.0);
        let hl_neg_20 = head_loss_at_q(&curve, -20.0);
        assert!(
            (hl_pos_20 - case.expected_head_loss_at_abs_q20_cms).abs() < file.tolerance_head_loss_m,
            "{}: |Q|=20 forward head loss",
            case.name
        );
        assert!(
            (hl_neg_20 - case.expected_head_loss_at_abs_q20_cms).abs() < file.tolerance_head_loss_m,
            "{}: |Q|=20 reverse head loss",
            case.name
        );
        assert!(
            (hl_pos_20 - hl_neg_20).abs() < file.tolerance_head_loss_m,
            "{}: symmetric |Q|=20",
            case.name
        );
    }
}
