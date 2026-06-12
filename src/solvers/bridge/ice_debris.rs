/// Optional pier debris and ice modifiers (user units until resolved on `BridgeGeometry`).
#[derive(Debug, Clone)]
pub struct BridgeIceDebrisParams {
    /// Multiplier on net opening area / conveyance (0–1]. 1.0 = no extra blockage.
    pub opening_blockage_factor: f64,
    /// Constant ice thickness through opening (user units).
    pub ice_thickness: f64,
    /// `0` = none, `1` = constant thickness, `2` = reserved dynamic jam.
    pub ice_mode: u8,
    /// Roadway ice lowering weir crest (user units).
    pub deck_ice_thickness: f64,
    /// Total debris width per pier in opening coordinates (user units).
    pub pier_debris_widths: Vec<f64>,
    /// Debris height below WSEL per pier (user units).
    pub pier_debris_heights: Vec<f64>,
}

impl Default for BridgeIceDebrisParams {
    fn default() -> Self {
        Self {
            opening_blockage_factor: 1.0,
            ice_thickness: 0.0,
            ice_mode: 0,
            deck_ice_thickness: 0.0,
            pier_debris_widths: Vec::new(),
            pier_debris_heights: Vec::new(),
        }
    }
}

pub(crate) fn clamp_opening_blockage_factor(f: f64) -> f64 {
    if f <= 0.0 || !f.is_finite() {
        1e-6
    } else {
        f.min(1.0)
    }
}

pub(crate) fn nested_bridge_scalar(values: &Option<Vec<f64>>, b_idx: usize, default: f64) -> f64 {
    values
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(default)
}

pub(crate) fn nested_bridge_pier_row(values: &Option<Vec<Vec<f64>>>, b_idx: usize) -> Vec<f64> {
    values
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .cloned()
        .unwrap_or_default()
}

/// Resolve ice/debris inputs for bridge index `b_idx` (steady / unsteady flat arrays).
pub fn ice_debris_params_for_bridge(
    opening_blockage_factors: &Option<Vec<f64>>,
    pier_debris_widths: &Option<Vec<Vec<f64>>>,
    pier_debris_heights: &Option<Vec<Vec<f64>>>,
    ice_thicknesses: &Option<Vec<f64>>,
    ice_modes: &Option<Vec<i32>>,
    deck_ice_thicknesses: &Option<Vec<f64>>,
    b_idx: usize,
) -> BridgeIceDebrisParams {
    BridgeIceDebrisParams {
        opening_blockage_factor: clamp_opening_blockage_factor(nested_bridge_scalar(
            opening_blockage_factors,
            b_idx,
            1.0,
        )),
        ice_thickness: nested_bridge_scalar(ice_thicknesses, b_idx, 0.0),
        ice_mode: ice_modes
            .as_ref()
            .and_then(|m| m.get(b_idx))
            .copied()
            .unwrap_or(0)
            .clamp(0, 2) as u8,
        deck_ice_thickness: nested_bridge_scalar(deck_ice_thicknesses, b_idx, 0.0),
        pier_debris_widths: nested_bridge_pier_row(pier_debris_widths, b_idx),
        pier_debris_heights: nested_bridge_pier_row(pier_debris_heights, b_idx),
    }
}
