use std::ops::{Mul, Add, Sub, Neg};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluxQuaternion {
    pub w: f32, // Scalar Pressure (Ether Density)
    pub x: f32, // Vector Flow X
    pub y: f32, // Vector Flow Y
    pub z: f32, // Vector Flow Z
}

impl FluxQuaternion {
    pub fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self { w, x, y, z }
    }

    // Custom multiplication: Hamilton product for fluid dynamics
    pub fn mul(&self, other: &FluxQuaternion) -> FluxQuaternion {
        FluxQuaternion {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }


    pub fn norm_sq(&self) -> f32 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn normalize(&self) -> Self {
        let n = self.norm_sq().sqrt();
        if n < 1e-6 {
            Self::new(1.0, 0.0, 0.0, 0.0)
        } else {
            Self { w: self.w / n, x: self.x / n, y: self.y / n, z: self.z / n }
        }
    }

    // Compute interaction between two waves (for light, gravity, wind effects)
    // NOTE: original version normalized the result, destroying magnitude (angular momentum).
    // This version preserves magnitude so conservation laws hold correctly.
    pub fn interact(&self, other: &Self) -> Self {
        // Non-linear interaction for vortex theory: product + interference
        let product = self.mul(other);
        let interference = Self::new(
            self.w * other.w, // pressure interference
            self.x * other.y - self.y * other.x, // cross terms for vortex
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
        );
        // Do NOT normalize — magnitude encodes kinetic energy / angular momentum.
        product + interference
    }

    /// Add a force vector directly to the xyz (velocity) components without
    /// touching the scalar w (pressure). Used when you want clean Euler integration
    /// rather than quaternion multiplication so momentum is conserved.
    pub fn add_force(&self, fx: f32, fy: f32, fz: f32, dt: f32) -> Self {
        Self {
            w: self.w,
            x: self.x + fx * dt,
            y: self.y + fy * dt,
            z: self.z + fz * dt,
        }
    }

    /// Scale only the velocity (xyz) components, leaving pressure (w) unchanged.
    pub fn scale_velocity(&self, s: f32) -> Self {
        Self { w: self.w, x: self.x * s, y: self.y * s, z: self.z * s }
    }

    /// Return the speed (magnitude of the xyz vector part).
    pub fn speed(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

impl Mul for FluxQuaternion {
    type Output = FluxQuaternion;

    fn mul(self, rhs: Self) -> Self::Output {
        FluxQuaternion {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }
}

impl Add for FluxQuaternion {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            w: self.w + rhs.w,
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for FluxQuaternion {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            w: self.w - rhs.w,
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Neg for FluxQuaternion {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            w: -self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}