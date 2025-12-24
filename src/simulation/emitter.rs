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
    if *done { return; }
    *done = true;
    let black_hole_pos = black_hole_query.single().translation;

    // GENESIS PARAMETERS
    let phi = 1.618034;

    for i in 0..50_000 {
        // 1. RADIUS (Logarithmic-like distribution looks best, but linear is fine)
        // Spread them from 2.0 to 100.0
        let radius = 2.0 + rand::random::<f32>() * 98.0;

        // 2. SPIRAL MATH (Genesis Mode)
        // Theta = ln(r) * Phi
        let base_theta = radius.ln() * phi;

        // 3. SYMMETRY (2 Arms)
        // Even = Arm A (0), Odd = Arm B (PI)
        let arm_id = if i % 2 == 0 { 0 } else { 1 };
        let arm_offset = if arm_id == 0 { 0.0 } else { std::f32::consts::PI };

        // 4. FUZZ (Thickness)
        let fuzz = (rand::random::<f32>() - 0.5) * 0.5;
        let theta = base_theta + arm_offset + fuzz;

        // 5. POSITION
        let position = black_hole_pos + Vec3::new(
            radius * theta.cos(),
            (rand::random::<f32>() - 0.5) * 4.0, // Vertical spread
            radius * theta.sin()
        );

        // 6. VELOCITY (Tangent stability)
        let tangent = Vec3::new(-position.z, 0.0, position.x).normalize();
        let velocity = tangent * 15.0;

        // 7. COLOR
        let color = if radius < 15.0 {
            Color::srgb(1.0, 0.84, 0.0) // Gold Core
        } else {
            Color::srgb(0.0, 1.0, 1.0) // Cyan Arms
        };

        commands.spawn((
            PlasmaParticle {
                velocity,
                history: std::collections::VecDeque::new(),
                color,
                original_radius: radius,
                arm: arm_id, // Save the ID so Physics knows where to pull!
            },
            Transform::from_translation(position),
        ));
    }
}