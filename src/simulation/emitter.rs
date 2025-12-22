use bevy::prelude::*;
use crate::simulation::plasma::PlasmaParticle;
use crate::simulation::galaxy::BlackHole;

pub struct EmitterPlugin;

impl Plugin for EmitterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, big_bang_burst);
    }
}

fn big_bang_burst(
    mut commands: Commands,
    black_hole_query: Query<&Transform, With<BlackHole>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    let black_hole_pos = black_hole_query.single().translation;

    for _ in 0..50_000 {
        // Choose arm: 0 or 1
        let arm = rand::random::<u32>() % 2;
        let arm_offset = if arm == 0 { 0.0 } else { std::f32::consts::PI };

        // Radius from 1.0 to 100.0
        let radius = 1.0 + rand::random::<f32>() * 99.0;

        // Angle = (ln(Radius) / 0.3) + ArmOffset
        let angle = (radius.ln() / 0.3) + arm_offset;

        // Jitter: add slight random noise
        let jitter = (rand::random::<f32>() - 0.5) * 0.1; // small noise
        let angle_jittered = angle + jitter;

        let position = black_hole_pos + Vec3::new(radius * angle_jittered.cos(), 0.0, radius * angle_jittered.sin());

        // Initial tangential velocity matching galaxy rotation
        let omega = (1.618 - 1.0) * 0.2;
        let tangent = Vec3::new(-angle_jittered.sin(), 0.0, angle_jittered.cos());
        let velocity = tangent * radius * omega;

        // Color based on radius
        let color = if radius < 10.0 {
            Color::srgb(1.0, 0.84, 0.0) // Gold
        } else {
            Color::srgb(0.0, 1.0, 1.0) // Cyan
        };

        commands.spawn((
            PlasmaParticle {
                velocity,
                history: std::collections::VecDeque::new(),
                color,
                original_radius: radius,
                arm,
            },
            Transform::from_translation(position),
        ));
    }
}