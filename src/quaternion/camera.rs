use crate::quaternion::math::FluxQuaternion;

/// Quaternion-based fly camera.
/// - Position stored as (x, y, z) floats
/// - Orientation stored as a unit FluxQuaternion (no Euler angles, no gimbal lock)
/// - All movement and rotation done via quaternion operations
pub struct QuaternionCamera {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub orientation: FluxQuaternion, // unit quaternion
    pub move_speed: f32,
    pub look_sensitivity: f32,
}

impl QuaternionCamera {
    pub fn new(px: f32, py: f32, pz: f32) -> Self {
        Self {
            pos_x: px,
            pos_y: py,
            pos_z: pz,
            orientation: FluxQuaternion::identity(),
            move_speed: 30.0,
            look_sensitivity: 0.002,
        }
    }

    /// Move forward/back along camera's local Z axis
    pub fn move_forward(&mut self, amount: f32) {
        let (fx, fy, fz) = self.orientation.rotate_vec3(0.0, 0.0, -1.0);
        self.pos_x += fx * amount;
        self.pos_y += fy * amount;
        self.pos_z += fz * amount;
    }

    /// Move right/left along camera's local X axis
    pub fn move_right(&mut self, amount: f32) {
        let (rx, ry, rz) = self.orientation.rotate_vec3(1.0, 0.0, 0.0);
        self.pos_x += rx * amount;
        self.pos_y += ry * amount;
        self.pos_z += rz * amount;
    }

    /// Move up/down along camera's local Y axis
    pub fn move_up(&mut self, amount: f32) {
        let (ux, uy, uz) = self.orientation.rotate_vec3(0.0, 1.0, 0.0);
        self.pos_x += ux * amount;
        self.pos_y += uy * amount;
        self.pos_z += uz * amount;
    }

    /// Rotate camera by mouse delta (dx = yaw, dy = pitch)
    /// Uses quaternion multiplication — no gimbal lock
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        // Yaw: rotate around world Y axis
        let yaw = FluxQuaternion::from_axis_angle(0.0, 1.0, 0.0, -dx * self.look_sensitivity);
        // Pitch: rotate around camera's local X axis
        let pitch = FluxQuaternion::from_axis_angle(1.0, 0.0, 0.0, -dy * self.look_sensitivity);

        // Apply yaw in world space, pitch in local space
        self.orientation = yaw.mul(&self.orientation).mul(&pitch).normalize();
    }

    /// Build the view matrix for the GPU (the ONLY matrix in the engine)
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        FluxQuaternion::to_view_matrix(&self.orientation, self.pos_x, self.pos_y, self.pos_z)
    }
}
