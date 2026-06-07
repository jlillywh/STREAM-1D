//! Shared mathematical utilities and unit conversions for STREAMS-1D.

/// Acceleration due to gravity (g) in m/s^2.
pub const G_METRIC: f64 = 9.80665;

/// Conversion factor: feet to meters.
pub const FT_TO_M: f64 = 0.3048;

/// Conversion factor: cubic feet per second (cfs) to cubic meters per second (cms).
pub const CFS_TO_CMS: f64 = 0.028316846592;

/// Conversion factor: feet squared to meters squared.
pub const FT2_TO_M2: f64 = FT_TO_M * FT_TO_M;

/// Supported unit systems for inputs and outputs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UnitSystem {
    USCustomary,
    Metric,
}

/// Helper structure for 2x2 matrices to support the block Thomas algorithm.
#[derive(Debug, Copy, Clone)]
pub struct Mat2 {
    pub m11: f64, pub m12: f64,
    pub m21: f64, pub m22: f64,
}

impl Mat2 {
    pub fn zero() -> Self {
        Self { m11: 0.0, m12: 0.0, m21: 0.0, m22: 0.0 }
    }

    pub fn identity() -> Self {
        Self { m11: 1.0, m12: 0.0, m21: 0.0, m22: 1.0 }
    }

    pub fn inv(&self) -> Option<Self> {
        let det = self.m11 * self.m22 - self.m12 * self.m21;
        if det.abs() < 1e-12 {
            return None;
        }
        let inv_det = 1.0 / det;
        Some(Self {
            m11: self.m22 * inv_det,
            m12: -self.m12 * inv_det,
            m21: -self.m21 * inv_det,
            m22: self.m11 * inv_det,
        })
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self {
            m11: self.m11 * other.m11 + self.m12 * other.m21,
            m12: self.m11 * other.m12 + self.m12 * other.m22,
            m21: self.m21 * other.m11 + self.m22 * other.m21,
            m22: self.m21 * other.m12 + self.m22 * other.m22,
        }
    }

    pub fn mul_vec(&self, v: &Vec2) -> Vec2 {
        Vec2 {
            v1: self.m11 * v.v1 + self.m12 * v.v2,
            v2: self.m21 * v.v1 + self.m22 * v.v2,
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self {
            m11: self.m11 - other.m11,
            m12: self.m12 - other.m12,
            m21: self.m21 - other.m21,
            m22: self.m22 - other.m22,
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            m11: self.m11 + other.m11,
            m12: self.m12 + other.m12,
            m21: self.m21 + other.m21,
            m22: self.m22 + other.m22,
        }
    }
}

/// Helper structure for 2D vectors.
#[derive(Debug, Copy, Clone)]
pub struct Vec2 {
    pub v1: f64,
    pub v2: f64,
}

impl Vec2 {
    pub fn zero() -> Self {
        Self { v1: 0.0, v2: 0.0 }
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            v1: self.v1 + other.v1,
            v2: self.v2 + other.v2,
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self {
            v1: self.v1 - other.v1,
            v2: self.v2 - other.v2,
        }
    }
}

/// Solves a 2x2 block tridiagonal system of linear equations of the form:
/// A_i * X_{i-1} + B_i * X_i + C_i * X_{i+1} = D_i  for i = 0..N-1
///
/// Under boundaries:
/// A_0 = 0 (no X_{-1})
/// C_{N-1} = 0 (no X_N)
///
/// Returns the list of solved 2D vectors X_i, or None if inversion fails.
pub fn solve_block_tridiagonal(
    a: &[Mat2],
    b: &[Mat2],
    c: &[Mat2],
    d: &[Vec2],
) -> Option<Vec<Vec2>> {
    let n = b.len();
    if n == 0 {
        return Some(vec![]);
    }
    if a.len() != n || c.len() != n || d.len() != n {
        return None;
    }

    let mut e = vec![Mat2::zero(); n];
    let mut f = vec![Vec2::zero(); n];

    // Node 0
    let b0_inv = b[0].inv()?;
    e[0] = b0_inv.mul(&c[0]);
    f[0] = b0_inv.mul_vec(&d[0]);

    // Forward sweep
    for i in 1..n {
        let denom = b[i].sub(&a[i].mul(&e[i - 1]));
        let denom_inv = denom.inv()?;
        if i < n - 1 {
            e[i] = denom_inv.mul(&c[i]);
        }
        let diff_d = d[i].sub(&a[i].mul_vec(&f[i - 1]));
        f[i] = denom_inv.mul_vec(&diff_d);
    }

    // Backward substitution
    let mut x = vec![Vec2::zero(); n];
    x[n - 1] = f[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = f[i].sub(&e[i].mul_vec(&x[i + 1]));
    }

    Some(x)
}

/// Solves a standard scalar tridiagonal matrix system of the form:
/// a_i * x_{i-1} + b_i * x_i + c_i * x_{i+1} = d_i
pub fn solve_scalar_tridiagonal(
    a: &[f64],
    b: &[f64],
    c: &[f64],
    d: &[f64],
) -> Option<Vec<f64>> {
    let n = b.len();
    if n == 0 {
        return Some(vec![]);
    }
    if a.len() != n || c.len() != n || d.len() != n {
        return None;
    }

    let mut c_prime = vec![0.0; n];
    let mut d_prime = vec![0.0; n];

    if b[0].abs() < 1e-12 {
        return None;
    }
    c_prime[0] = c[0] / b[0];
    d_prime[0] = d[0] / b[0];

    for i in 1..n {
        let denom = b[i] - a[i] * c_prime[i - 1];
        if denom.abs() < 1e-12 {
            return None;
        }
        if i < n - 1 {
            c_prime[i] = c[i] / denom;
        }
        d_prime[i] = (d[i] - a[i] * d_prime[i - 1]) / denom;
    }

    let mut x = vec![0.0; n];
    x[n - 1] = d_prime[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = d_prime[i] - c_prime[i] * x[i + 1];
    }

    Some(x)
}
