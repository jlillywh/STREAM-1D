//! Reach friction-slope averaging for Preissmann momentum (HEC-RAS plan methods 1–4).

/// HEC-RAS `Unsteady Friction Slope Method` (plan file numbering).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsteadyFrictionSlopeMethod {
    /// $(Q_1+Q_2)^2 / (K_1+K_2)^2$ — STREAM-1D historical default.
    AverageConveyance = 1,
    /// $(S_{f1}+S_{f2})/2$ with $S_{fi}=(Q/K_i)^2$ — HEC-RAS unsteady default.
    AverageFrictionSlope = 2,
    /// $\sqrt{S_{f1} S_{f2}}$
    GeometricMean = 3,
    /// $2 S_{f1} S_{f2}/(S_{f1}+S_{f2})$
    HarmonicMean = 4,
}

impl UnsteadyFrictionSlopeMethod {
    pub fn from_i32(val: i32) -> Self {
        match val {
            2 => Self::AverageFrictionSlope,
            3 => Self::GeometricMean,
            4 => Self::HarmonicMean,
            // Method 5 (HEC-6 adaptive) not implemented — use arithmetic mean.
            5 => Self::AverageFrictionSlope,
            _ => Self::AverageConveyance,
        }
    }
}

/// Friction slope and partial derivatives for one reach interval.
#[derive(Debug, Clone, Copy)]
pub struct ReachFrictionSlopeDerivs {
    pub sf: f64,
    pub d_sf_dy_i: f64,
    pub d_sf_dy_ip: f64,
    pub d_sf_dq_i: f64,
    pub d_sf_dq_ip: f64,
}

fn local_sf(q: f64, k: f64) -> f64 {
    let k_clamp = k.max(0.01);
    (q * q.abs()) / (k_clamp * k_clamp)
}

fn d_local_sf_dq(q: f64, k: f64) -> f64 {
    let k_clamp = k.max(0.01);
    2.0 * q.abs() / (k_clamp * k_clamp)
}

fn d_local_sf_dy(q: f64, k: f64, dk_dy: f64, depth: f64) -> f64 {
    if depth < 0.1 {
        return 0.0;
    }
    let k_clamp = k.max(0.01);
    -q * q.abs() / (k_clamp * k_clamp * k_clamp) * dk_dy
}

/// Compute representative reach friction slope and Jacobian terms.
pub fn reach_friction_slope(
    method: UnsteadyFrictionSlopeMethod,
    q_i: f64,
    q_ip: f64,
    k_i: f64,
    k_ip: f64,
    dk_dy_i: f64,
    dk_dy_ip: f64,
    depth_i: f64,
    depth_ip: f64,
) -> ReachFrictionSlopeDerivs {
    match method {
        UnsteadyFrictionSlopeMethod::AverageConveyance => {
            let q_avg = 0.5 * (q_i + q_ip);
            let k_avg = 0.5 * (k_i + k_ip).max(0.01);
            let d_sf_dq = d_local_sf_dq(q_avg, k_avg);
            ReachFrictionSlopeDerivs {
                sf: local_sf(q_avg, k_avg),
                d_sf_dy_i: d_local_sf_dy(q_avg, k_avg, 0.5 * dk_dy_i, depth_i),
                d_sf_dy_ip: d_local_sf_dy(q_avg, k_avg, 0.5 * dk_dy_ip, depth_ip),
                d_sf_dq_i: 0.5 * d_sf_dq,
                d_sf_dq_ip: 0.5 * d_sf_dq,
            }
        }
        UnsteadyFrictionSlopeMethod::AverageFrictionSlope => {
            let sf_i = local_sf(q_i, k_i);
            let sf_ip = local_sf(q_ip, k_ip);
            ReachFrictionSlopeDerivs {
                sf: 0.5 * (sf_i + sf_ip),
                d_sf_dy_i: 0.5 * d_local_sf_dy(q_i, k_i, dk_dy_i, depth_i),
                d_sf_dy_ip: 0.5 * d_local_sf_dy(q_ip, k_ip, dk_dy_ip, depth_ip),
                d_sf_dq_i: 0.5 * d_local_sf_dq(q_i, k_i),
                d_sf_dq_ip: 0.5 * d_local_sf_dq(q_ip, k_ip),
            }
        }
        UnsteadyFrictionSlopeMethod::GeometricMean => {
            let sf_i = local_sf(q_i, k_i);
            let sf_ip = local_sf(q_ip, k_ip);
            let sf = (sf_i * sf_ip).sqrt();
            if sf <= 1e-12 {
                return ReachFrictionSlopeDerivs {
                    sf: 0.0,
                    d_sf_dy_i: 0.0,
                    d_sf_dy_ip: 0.0,
                    d_sf_dq_i: 0.0,
                    d_sf_dq_ip: 0.0,
                };
            }
            let half_ratio_i = 0.5 * sf / sf_i.max(1e-12);
            let half_ratio_ip = 0.5 * sf / sf_ip.max(1e-12);
            ReachFrictionSlopeDerivs {
                sf,
                d_sf_dy_i: half_ratio_i * d_local_sf_dy(q_i, k_i, dk_dy_i, depth_i),
                d_sf_dy_ip: half_ratio_ip * d_local_sf_dy(q_ip, k_ip, dk_dy_ip, depth_ip),
                d_sf_dq_i: half_ratio_i * d_local_sf_dq(q_i, k_i),
                d_sf_dq_ip: half_ratio_ip * d_local_sf_dq(q_ip, k_ip),
            }
        }
        UnsteadyFrictionSlopeMethod::HarmonicMean => {
            let sf_i = local_sf(q_i, k_i);
            let sf_ip = local_sf(q_ip, k_ip);
            let denom = sf_i + sf_ip;
            if denom <= 1e-12 {
                return ReachFrictionSlopeDerivs {
                    sf: 0.0,
                    d_sf_dy_i: 0.0,
                    d_sf_dy_ip: 0.0,
                    d_sf_dq_i: 0.0,
                    d_sf_dq_ip: 0.0,
                };
            }
            let sf = 2.0 * sf_i * sf_ip / denom;
            let d_sf_d_sf_i = 2.0 * sf_ip * sf_ip / (denom * denom);
            let d_sf_d_sf_ip = 2.0 * sf_i * sf_i / (denom * denom);
            ReachFrictionSlopeDerivs {
                sf,
                d_sf_dy_i: d_sf_d_sf_i * d_local_sf_dy(q_i, k_i, dk_dy_i, depth_i),
                d_sf_dy_ip: d_sf_d_sf_ip * d_local_sf_dy(q_ip, k_ip, dk_dy_ip, depth_ip),
                d_sf_dq_i: d_sf_d_sf_i * d_local_sf_dq(q_i, k_i),
                d_sf_dq_ip: d_sf_d_sf_ip * d_local_sf_dq(q_ip, k_ip),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_conveyance_matches_legacy_formula() {
        let d = reach_friction_slope(
            UnsteadyFrictionSlopeMethod::AverageConveyance,
            10.0,
            12.0,
            50.0,
            60.0,
            0.0,
            0.0,
            1.0,
            1.0,
        );
        let q_avg = 11.0;
        let k_avg = 55.0;
        let expected = (q_avg * q_avg) / (k_avg * k_avg);
        assert!((d.sf - expected).abs() < 1e-12);
    }

    #[test]
    fn average_friction_slope_is_arithmetic_mean_of_local() {
        let d = reach_friction_slope(
            UnsteadyFrictionSlopeMethod::AverageFrictionSlope,
            10.0,
            12.0,
            50.0,
            60.0,
            0.0,
            0.0,
            1.0,
            1.0,
        );
        let sf_i = (10.0 * 10.0) / (50.0 * 50.0);
        let sf_ip = (12.0 * 12.0) / (60.0 * 60.0);
        assert!((d.sf - 0.5 * (sf_i + sf_ip)).abs() < 1e-12);
    }
}
