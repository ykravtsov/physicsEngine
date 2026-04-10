use std::ops::{Add, Mul, Neg, Sub};

/// FluxQuaternion: the core mathematical object of the vortex theory.
/// - w = scalar pressure (ether density / energy)
/// - x, y, z = vector flow (velocity / force direction)
///
/// All physics, camera, and rendering operations use this type.
/// No matrices anywhere except the final GPU projection.
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

    pub fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 0.0)
    }

    /// Pure quaternion from a 3D vector (w=0)
    pub fn from_vec3(x: f32, y: f32, z: f32) -> Self {
        Self::new(0.0, x, y, z)
    }

    /// Unit rotation quaternion: rotate by `angle` radians around axis (ax, ay, az)
    pub fn from_axis_angle(ax: f32, ay: f32, az: f32, angle: f32) -> Self {
        let half = angle * 0.5;
        let s = half.sin();
        let len = (ax * ax + ay * ay + az * az).sqrt().max(1e-8);
        Self::new(half.cos(), ax / len * s, ay / len * s, az / len * s)
    }

    /// Hamilton product: the fundamental quaternion multiplication
    pub fn mul(&self, other: &FluxQuaternion) -> FluxQuaternion {
        FluxQuaternion {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    pub fn conjugate(&self) -> Self {
        Self::new(self.w, -self.x, -self.y, -self.z)
    }

    pub fn norm_sq(&self) -> f32 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn norm(&self) -> f32 {
        self.norm_sq().sqrt()
    }

    pub fn normalize(&self) -> Self {
        let n = self.norm();
        if n < 1e-8 {
            Self::identity()
        } else {
            Self {
                w: self.w / n,
                x: self.x / n,
                y: self.y / n,
                z: self.z / n,
            }
        }
    }

    /// Rotate a 3D vector using quaternion sandwich product: q * v * q^-1
    pub fn rotate_vec3(&self, vx: f32, vy: f32, vz: f32) -> (f32, f32, f32) {
        let qv = Self::from_vec3(vx, vy, vz);
        let conj = self.conjugate();
        let temp = self.mul(&qv);
        let result = temp.mul(conj);
        (result.x, result.y, result.z)
    }

    /// Build a 4x4 view matrix from a camera quaternion orientation and position.
    /// This is the ONLY place in the engine where we produce a matrix.
    /// The matrix is only needed for the GPU vertex shader clip-space transform.
    pub fn to_view_matrix(
        orientation: &FluxQuaternion,
        px: f32,
        py: f32,
        pz: f32,
    ) -> [[f32; 4]; 4] {
        // Extract basis vectors by rotating world axes with the camera quaternion
        let (rx, ry, rz) = orientation.rotate_vec3(1.0, 0.0, 0.0); // right
        let (ux, uy, uz) = orientation.rotate_vec3(0.0, 1.0, 0.0); // up
        let (fx, fy, fz) = orientation.rotate_vec3(0.0, 0.0, -1.0); // forward (into screen)

        // View matrix = rotation^T * translation
        [
            [rx, ux, -fx, 0.0],
            [ry, uy, -fy, 0.0],
            [rz, uz, -fz, 0.0],
            [
                -(rx * px + ry * py + rz * pz),
                -(ux * px + uy * py + uz * pz),
                (fx * px + fy * py + fz * pz),
                1.0,
            ],
        ]
    }

    /// Build a perspective projection matrix from FOV, aspect, near, far.
    pub fn perspective_matrix(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
        let f = 1.0 / (fov_y_rad / 2.0).tan();
        let range_inv = 1.0 / (near - far);
        [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (near + far) * range_inv, -1.0],
            [0.0, 0.0, near * far * range_inv * 2.0, 0.0],
        ]
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
