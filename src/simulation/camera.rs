use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, PrimaryWindow};

#[derive(Component)]
pub struct FlyCamera {
    pub speed: f32,
    pub sensitivity: f32,
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            speed: 10.0,
            sensitivity: 0.002,
        }
    }
}

pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (camera_movement, camera_look, cursor_grab));
    }
}

fn camera_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&FlyCamera, &mut Transform)>,
) {
    for (fly_camera, mut transform) in query.iter_mut() {
        let mut velocity = Vec3::ZERO;

        if keys.pressed(KeyCode::KeyW) {
            velocity += *transform.forward();
        }
        if keys.pressed(KeyCode::KeyS) {
            velocity -= *transform.forward();
        }
        if keys.pressed(KeyCode::KeyA) {
            velocity -= *transform.right();
        }
        if keys.pressed(KeyCode::KeyD) {
            velocity += *transform.right();
        }
        if keys.pressed(KeyCode::Space) {
            velocity += Vec3::Y;
        }
        if keys.pressed(KeyCode::ShiftLeft) {
            velocity -= Vec3::Y;
        }

        transform.translation += velocity * fly_camera.speed * time.delta_seconds();
    }
}

fn camera_look(
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<(&FlyCamera, &mut Transform)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }

    for (fly_camera, mut transform) in query.iter_mut() {
        let (mut yaw, mut pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);

        yaw -= delta.x * fly_camera.sensitivity;
        pitch -= delta.y * fly_camera.sensitivity;

        pitch = pitch.clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
}

fn cursor_grab(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = window_query.single_mut();
    window.cursor.grab_mode = CursorGrabMode::Locked;
    window.cursor.visible = false;
}