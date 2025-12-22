use std::ops::Mul;

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

    // Apply Golden Drag: rotate the flow vector by μ ≈ φ^{-4} around z-axis
    pub fn apply_golden_drag(&self) -> FluxQuaternion {
        const PHI_INV_4: f32 = 0.146446609406726237799577818947237528; // φ^{-4}
        let cos_mu = PHI_INV_4.cos();
        let sin_mu = PHI_INV_4.sin();

        // Rotate (x, y) around z by μ
        let new_x = self.x * cos_mu - self.y * sin_mu;
        let new_y = self.x * sin_mu + self.y * cos_mu;

        FluxQuaternion {
            w: self.w,
            x: new_x,
            y: new_y,
            z: self.z,
        }
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