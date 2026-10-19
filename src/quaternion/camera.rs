use crate::quaternion::math::FluxQuaternion;

#[derive(Clone, Copy)]
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

    pub fn mouse_look(&mut self, dx: f32, dy: f32) {
        let (fx, fy, fz) = self.orientation.rotate_vec3(0.0, 0.0, -1.0);
        let yaw = (-fx).atan2(-fz) - dx * 0.003;
        let limit = std::f32::consts::FRAC_PI_2 - 0.01;
        let pitch = (fy.clamp(-1.0, 1.0).asin() - dy * 0.003).clamp(-limit, limit);
        self.orientation = (FluxQuaternion::from_axis_angle(0.0, 1.0, 0.0, yaw)
            * FluxQuaternion::from_axis_angle(1.0, 0.0, 0.0, pitch))
        .normalize();
    }

    pub fn fly(&mut self, dt: f32, right: f32, up: f32, forward: f32) {
        let (rx, ry, rz) = self.orientation.rotate_vec3(1.0, 0.0, 0.0);
        let (fx, fy, fz) = self.orientation.rotate_vec3(0.0, 0.0, -1.0);
        let dx = rx * right + fx * forward;
        let dy = ry * right + fy * forward + up;
        let dz = rz * right + fz * forward;
        let distance = 180.0 * dt / (dx * dx + dy * dy + dz * dz).sqrt().max(1.0);
        self.pos_x += dx * distance;
        self.pos_y += dy * distance;
        self.pos_z += dz * distance;
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

    fn assert_position(camera: &QuaternionCamera, expected: [f32; 3]) {
        for (actual, expected) in [camera.pos_x, camera.pos_y, camera.pos_z]
            .into_iter()
            .zip(expected)
        {
            assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
        }
    }

    #[test]
    fn free_flight_can_pass_through_the_orbit_target() {
        let mut camera = QuaternionCamera::new(0.0, 0.0, 10.0);
        camera.look_at(0.0, 0.0, 0.0);
        camera.fly(1.0, 0.0, 0.0, 1.0);
        assert_position(&camera, [0.0, 0.0, -170.0]);
    }

    #[test]
    fn free_flight_uses_camera_right_and_world_up() {
        let mut camera = QuaternionCamera::new(0.0, 0.0, 0.0);
        camera.orientation =
            FluxQuaternion::from_axis_angle(0.0, 1.0, 0.0, std::f32::consts::FRAC_PI_2)
                * FluxQuaternion::from_axis_angle(1.0, 0.0, 0.0, -0.7);
        camera.fly(1.0, 1.0, 0.0, 0.0);
        assert_position(&camera, [0.0, 0.0, -180.0]);
        camera.fly(1.0, 0.0, 1.0, 0.0);
        assert_position(&camera, [0.0, 180.0, -180.0]);
    }

    #[test]
    fn diagonal_flight_is_not_faster() {
        let mut camera = QuaternionCamera::new(0.0, 0.0, 0.0);
        camera.fly(1.0, 1.0, 1.0, 1.0);
        let distance = (camera.pos_x.powi(2) + camera.pos_y.powi(2) + camera.pos_z.powi(2)).sqrt();
        assert!((distance - 180.0).abs() < 1e-3);
    }

    #[test]
    fn flight_distance_is_independent_of_frame_rate() {
        let mut initial = QuaternionCamera::new(200.0, 480.0, 300.0);
        initial.look_at(0.0, 4.0, 0.0);
        let mut expected = initial;
        expected.fly(1.0, 1.0, 0.0, 1.0);
        for fps in [1, 5, 30, 120] {
            let mut camera = initial;
            for _ in 0..fps {
                camera.fly(1.0 / fps as f32, 1.0, 0.0, 1.0);
            }
            assert_position(&camera, [expected.pos_x, expected.pos_y, expected.pos_z]);
        }
    }

    #[test]
    fn mouse_look_turns_right_and_up_without_roll() {
        let mut camera = QuaternionCamera::new(0.0, 0.0, 0.0);
        camera.mouse_look(100.0, -100.0);
        let (fx, fy, _) = camera.orientation.rotate_vec3(0.0, 0.0, -1.0);
        let (_, ry, _) = camera.orientation.rotate_vec3(1.0, 0.0, 0.0);
        assert!(fx > 0.2 && fy > 0.2);
        assert!(ry.abs() < 1e-6);
    }

    #[test]
    fn flight_view_matrix_maps_camera_axes_correctly() {
        let mut camera = QuaternionCamera::new(200.0, 480.0, 300.0);
        camera.look_at(0.0, 4.0, 0.0);
        camera.mouse_look(80.0, -40.0);
        let view = camera.view_matrix();
        for (axis, expected) in [
            ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
            ([10.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            ([0.0, 10.0, 0.0], [0.0, 10.0, 0.0]),
            ([0.0, 0.0, -10.0], [0.0, 0.0, -10.0]),
        ] {
            let (x, y, z) = camera.orientation.rotate_vec3(axis[0], axis[1], axis[2]);
            let point = [camera.pos_x + x, camera.pos_y + y, camera.pos_z + z];
            for row in 0..3 {
                let actual = view[0][row] * point[0]
                    + view[1][row] * point[1]
                    + view[2][row] * point[2]
                    + view[3][row];
                assert!((actual - expected[row]).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn mouse_look_limits_pitch_without_flipping() {
        let mut camera = QuaternionCamera::new(0.0, 0.0, 0.0);
        for dy in [-1e6, 1e6] {
            camera.mouse_look(123.0, dy);
            let (_, uy, _) = camera.orientation.rotate_vec3(0.0, 1.0, 0.0);
            let (_, ry, _) = camera.orientation.rotate_vec3(1.0, 0.0, 0.0);
            assert!(uy > 0.005);
            assert!(ry.abs() < 1e-6);
            assert!((camera.orientation.norm() - 1.0).abs() < 1e-6);
        }
    }
}
