use crate::quaternion::math::FluxQuaternion;

/// Orbit camera orientation. Position is stored as floats. The view direction
/// is a unit `FluxQuaternion` aimed with `look_at` (no Euler pitch stored here).
pub struct QuaternionCamera {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub orientation: FluxQuaternion,
}

impl QuaternionCamera {
    pub fn new(px: f32, py: f32, pz: f32) -> Self {
        Self {
            pos_x: px,
            pos_y: py,
            pos_z: pz,
            orientation: FluxQuaternion::identity(),
        }
    }

    /// Point the camera's forward axis (−Z) at a world target, keeping world-up.
    pub fn look_at(&mut self, tx: f32, ty: f32, tz: f32) {
        let mut fx = tx - self.pos_x;
        let mut fy = ty - self.pos_y;
        let mut fz = tz - self.pos_z;
        let fl = (fx * fx + fy * fy + fz * fz).sqrt().max(1e-8);
        fx /= fl;
        fy /= fl;
        fz /= fl;

        // right = normalize(forward × world_up). world_up × forward points left.
        let mut rx = -fz;
        let mut ry = 0.0;
        let mut rz = fx;
        let rl = (rx * rx + ry * ry + rz * rz).sqrt();
        if rl < 1e-4 {
            rx = 1.0;
            ry = 0.0;
            rz = 0.0;
        } else {
            rx /= rl;
            ry /= rl;
            rz /= rl;
        }

        // camera up so that cross(right, up) = back, back = −forward
        let backx = -fx;
        let backy = -fy;
        let backz = -fz;
        let ux = backy * rz - backz * ry;
        let uy = backz * rx - backx * rz;
        let uz = backx * ry - backy * rx;

        self.orientation =
            quat_from_basis([rx, ry, rz], [ux, uy, uz], [backx, backy, backz]).normalize();
    }

    /// Build the view matrix for the GPU (the ONLY matrix in the engine)
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        FluxQuaternion::to_view_matrix(&self.orientation, self.pos_x, self.pos_y, self.pos_z)
    }
}

/// Quaternion that maps the local axes onto the three world vectors (columns).
fn quat_from_basis(right: [f32; 3], up: [f32; 3], back: [f32; 3]) -> FluxQuaternion {
    let (m00, m10, m20) = (right[0], right[1], right[2]);
    let (m01, m11, m21) = (up[0], up[1], up[2]);
    let (m02, m12, m22) = (back[0], back[1], back[2]);
    let trace = m00 + m11 + m22;
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        FluxQuaternion::new(0.25 * s, (m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s)
    } else if m00 > m11 && m00 > m22 {
        let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
        FluxQuaternion::new((m21 - m12) / s, 0.25 * s, (m01 + m10) / s, (m02 + m20) / s)
    } else if m11 > m22 {
        let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
        FluxQuaternion::new((m02 - m20) / s, (m01 + m10) / s, 0.25 * s, (m12 + m21) / s)
    } else {
        let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
        FluxQuaternion::new((m10 - m01) / s, (m02 + m20) / s, (m12 + m21) / s, 0.25 * s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_at_aims_forward_and_stays_upright() {
        let mut cam = QuaternionCamera::new(200.0, 480.0, 300.0);
        cam.look_at(0.0, 8.0, 0.0);
        let (fx, fy, fz) = cam.orientation.rotate_vec3(0.0, 0.0, -1.0);
        let (dx, dy, dz) = (-200.0_f32, 8.0 - 480.0, -300.0);
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        let dot = fx * dx / len + fy * dy / len + fz * dz / len;
        assert!(dot > 0.999, "forward dot {dot}, f=({fx},{fy},{fz})");
        let (_ux, uy, _uz) = cam.orientation.rotate_vec3(0.0, 1.0, 0.0);
        assert!(uy > 0.3, "camera up world-y {uy}");
    }
}
