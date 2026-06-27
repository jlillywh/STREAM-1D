use super::*;

#[test]
fn test_bridge_rating_curve() {
    let inputs = BridgeRatingCurveInputs {
        q_values: vec![10.0, 20.0, 30.0],
        bridge: BridgeSolveParams {
            low_chord: 5.0,
            high_chord: 7.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 2.5,
            low_flow_method: 3,
            channel_width: 10.0,
            manning_n: 0.03,
            ..Default::default()
        },
    };
    let curve = compute_bridge_rating_curve(&inputs);
    assert_eq!(curve.q.len(), 3);
    assert!(curve.wsel[1] > curve.wsel[0]);
    assert!(curve.wsel[2] > curve.wsel[1]);
    assert_eq!(curve.wsel_down.len(), 3);
    assert_eq!(curve.flow_regimes.len(), 3);
    assert!(!curve.flow_regimes[0].is_empty());
}
#[test]
fn test_bridge_rating_curve_negative_q_symmetric_channel() {
    let base = BridgeSolveParams {
        low_chord: 5.0,
        high_chord: 7.0,
        z_down: 0.0,
        z_up: 0.0,
        tw_wsel: 2.5,
        low_flow_method: 3,
        channel_width: 10.0,
        manning_n: 0.03,
        ..Default::default()
    };
    let forward = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![20.0],
        bridge: base.clone(),
    });
    let reverse = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![-20.0],
        bridge: base,
    });
    assert_eq!(forward.q.len(), 1);
    assert_eq!(reverse.q.len(), 1);
    assert!(
        forward.wsel[0] > forward.wsel_down[0],
        "forward HW should exceed TW"
    );
    assert!(
        reverse.wsel[0] > reverse.wsel_down[0],
        "reverse hydraulic HW (BD) should exceed TW (BU)"
    );
    assert!(
        (forward.head_losses[0] - reverse.head_losses[0]).abs() < 0.01,
        "symmetric channel: |Q|=20 should yield similar head loss both directions"
    );
}
#[test]
fn test_bridge_rating_curve_mixed_sign_q_values() {
    let curve = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![-15.0, 0.0, 15.0],
        bridge: BridgeSolveParams {
            low_chord: 5.0,
            high_chord: 7.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 2.5,
            tw_wsel_reverse: Some(2.6),
            low_flow_method: 3,
            channel_width: 10.0,
            manning_n: 0.03,
            ..Default::default()
        },
    });
    assert_eq!(curve.q.len(), 2, "Q=0 samples are skipped");
    assert!(curve.q.contains(&-15.0));
    assert!(curve.q.contains(&15.0));
}
#[test]
fn test_bridge_rating_curve_bidirectional_qmax_sweep() {
    let q_max = 30.0;
    let q_values = vec![-q_max, -20.0, -10.0, 10.0, 20.0, q_max];
    let curve = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values,
        bridge: symmetric_rating_bridge_params(),
    });

    assert_eq!(curve.q.len(), 6);
    assert_eq!(curve.wsel.len(), 6);
    assert_eq!(curve.wsel_down.len(), 6);
    assert_eq!(curve.flow_regimes.len(), 6);
    assert_eq!(curve.head_losses.len(), 6);

    for i in 0..curve.q.len() {
        assert!(
            curve.wsel[i] > curve.wsel_down[i],
            "hydraulic HW should exceed TW at q={}",
            curve.q[i]
        );
        assert!(
            curve.head_losses[i] > 0.0,
            "head loss should be positive at q={}",
            curve.q[i]
        );
        assert!(!curve.flow_regimes[i].is_empty());
        assert!(
            (curve.wsel[i] - curve.wsel_down[i] - curve.head_losses[i]).abs() < 1e-4,
            "head_loss = HW - TW at q={}",
            curve.q[i]
        );
    }

    let mut pos: Vec<usize> = (0..curve.q.len()).filter(|&i| curve.q[i] > 0.0).collect();
    pos.sort_by(|&a, &b| curve.q[a].partial_cmp(&curve.q[b]).unwrap());
    for w in pos.windows(2) {
        assert!(
            curve.wsel[w[1]] > curve.wsel[w[0]],
            "forward HW should increase with Q: {} -> {}",
            curve.q[w[0]],
            curve.q[w[1]]
        );
    }

    let mut neg: Vec<usize> = (0..curve.q.len()).filter(|&i| curve.q[i] < 0.0).collect();
    neg.sort_by(|&a, &b| curve.q[b].partial_cmp(&curve.q[a]).unwrap());
    for w in neg.windows(2) {
        assert!(
            curve.wsel[w[1]] > curve.wsel[w[0]],
            "reverse HW should increase with |Q|: {} -> {}",
            curve.q[w[0]],
            curve.q[w[1]]
        );
    }

    let idx_pos_20 = curve
        .q
        .iter()
        .position(|&q| (q - 20.0).abs() < 1e-6)
        .unwrap();
    let idx_neg_20 = curve
        .q
        .iter()
        .position(|&q| (q + 20.0).abs() < 1e-6)
        .unwrap();
    assert!(
        (curve.head_losses[idx_pos_20] - curve.head_losses[idx_neg_20]).abs() < 0.02,
        "symmetric |Q|=20 head loss: forward {} reverse {}",
        curve.head_losses[idx_pos_20],
        curve.head_losses[idx_neg_20]
    );
}
#[test]
fn test_bridge_rating_curve_per_side_abutments() {
    let base = BridgeSolveParams {
        low_chord: 5.0,
        high_chord: 7.0,
        z_down: 0.0,
        z_up: 0.0,
        tw_wsel: 2.5,
        low_flow_method: 4,
        channel_width: 10.0,
        manning_n: 0.03,
        ..Default::default()
    };
    let asymmetric = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0, 25.0],
        bridge: BridgeSolveParams {
            abutment_left_width: Some(1.0),
            abutment_right_width: Some(4.0),
            abutment_right_top_elevation: Some(2.5),
            ..base.clone()
        },
    });
    let legacy_symmetric = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0, 25.0],
        bridge: BridgeSolveParams {
            abutment_block_width: 5.0,
            ..base
        },
    });
    assert!(
        (asymmetric.wsel[0] - legacy_symmetric.wsel[0]).abs() > 0.01,
        "rating curve should honor per-side abutment geometry"
    );
    assert!(
        asymmetric.wsel[1] > asymmetric.wsel[0],
        "headwater should increase with discharge"
    );
}
