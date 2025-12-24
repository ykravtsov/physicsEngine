use bevy::prelude::*;
use std::collections::VecDeque;
use crate::simulation::galaxy::BlackHole;
use crate::simulation::gpu_galaxy::PhiResource;

const RESONANCE_SENSITIVITY: f32 = 1000.0;
const GOLDEN_RATIO: f32 = 1.61803398875;

pub struct PlasmaPlugin;

impl Plugin for PlasmaPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlasmaConfig::default())
            .insert_resource(CloudCentroid::default())
            .add_systems(Update, update_galaxy_physics)
            .add_systems(Update, calculate_centroid)
            .add_systems(Update, draw_trails);
    }
}

#[derive(Component)]
pub struct PlasmaParticle {
    pub velocity: Vec3,
    pub history: VecDeque<Vec3>,
    pub color: Color,
    pub original_radius: f32,
    pub arm: u32,
}

#[derive(Resource)]
pub struct PlasmaConfig;

impl Default for PlasmaConfig {
    fn default() -> Self {
        Self
    }
}

#[derive(Resource)]
pub struct CloudCentroid {
    pub position: Vec3,
}

impl Default for CloudCentroid {
    fn default() -> Self {
        Self { position: Vec3::ZERO }
    }
}


fn ideal_spiral_pos(r: f32, b: f32) -> Vec3 {
    let theta = b * r.ln();
    Vec3::new(r * theta.cos(), 0.0, r * theta.sin())
}

fn arm_tangent(r: f32, b: f32) -> Vec3 {
    let theta = b * r.ln();
    let dr_dtheta = r / b;
    let tangent_x = dr_dtheta * theta.cos() - r * theta.sin();
    let tangent_z = dr_dtheta * theta.sin() + r * theta.cos();
    Vec3::new(tangent_x, 0.0, tangent_z).normalize()
}

fn calculate_centroid(
    query: Query<&Transform, With<PlasmaParticle>>,
    mut centroid: ResMut<CloudCentroid>,
) {
    let mut sum = Vec3::ZERO;
    let mut count = 0;
    for transform in query.iter() {
        sum += transform.translation;
        count += 1;
    }
    if count > 0 {
        centroid.position = sum / count as f32;
    }
}

pub fn update_galaxy_physics(
    mut param_set: ParamSet<(
        Query<(&mut PlasmaParticle, &mut Transform)>,
        Query<&Transform, With<BlackHole>>,
    )>,
    time: Res<Time>,
    phi_res: Res<PhiResource>, // <--- INJECT THE RESOURCE
) {
    let black_hole_pos = param_set.p1().single().translation;
    let dt = time.delta_seconds();
    let galaxy_angle = time.elapsed_seconds() * 0.1;

    // READ THE DYNAMIC PHI FROM KEYBOARD INPUT
    let current_phi = phi_res.phi_value;

    for (mut particle, mut transform) in param_set.p0().iter_mut() {
        let pos = transform.translation;
        let delta_pos = pos - black_hole_pos;
        let r = delta_pos.length();

        // Jet Physics: If in jet (y > 10.0), disable spiral physics
        if pos.y.abs() > 10.0 {
            // Just update position, no forces
        } else {
            // Quasar Logic: If r < 3.0, eject vertically
            if r < 3.0 {
                transform.translation.x = black_hole_pos.x;
                transform.translation.z = black_hole_pos.z;
                particle.velocity = if rand::random::<bool>() {
                    Vec3::Y * 80.0
                } else {
                    Vec3::NEG_Y * 80.0
                };
                particle.color = Color::srgb(0.0, 1.0, 1.0); // Cyan
            } else {
                // --- RESONANCE CHECK (The Fix) ---
                // The Ether only vibrates effectively at the Golden Ratio.
                // If the system is detuned, the "Pinch" loses coherence.
                let golden_ratio = GOLDEN_RATIO;
                let deviation = (current_phi - golden_ratio).abs();

                // Gaussian Falloff: High sensitivity (RESONANCE_SENSITIVITY).
                // Small deviations causes massive loss of force.
                let resonance = (-deviation * deviation * RESONANCE_SENSITIVITY).exp();

                // --- Z-PINCH LOGIC ---
                let arm_offset = if particle.arm == 0 { 0.0 } else { std::f32::consts::PI };

                // We still calculate target based on current input to visualize the "attempt",
                // but the STRENGTH of the result depends on Resonance.
                let angle = (particle.original_radius.ln() * current_phi) + arm_offset + galaxy_angle;

                let ideal_pos = black_hole_pos + Vec3::new(
                    particle.original_radius * angle.cos(),
                    0.0,
                    particle.original_radius * angle.sin()
                );

                // Apply Forces
                let pinch_vector = ideal_pos - pos;

                // APPLY RESONANCE TO THE FORCE
                // If resonance is 1.0 (Tuned), force is 10.0.
                // If resonance is 0.0 (Detuned), force is 0.0 -> Galaxy flies apart.
                let pinch_force = pinch_vector * 10.0 * resonance;

                let drag_force = -particle.velocity * 0.5;

                particle.velocity += (pinch_force + drag_force) * dt;
            }
        }

        // Update position
        transform.translation += particle.velocity * dt;

        // Update history
        particle.history.push_back(transform.translation);
        if particle.history.len() > 20 {
            particle.history.pop_front();
        }
    }
}

fn draw_trails(mut gizmos: Gizmos, query: Query<&PlasmaParticle>) {
    for particle in query.iter() {
        if particle.history.len() < 2 {
            continue;
        }
        let positions: Vec<Vec3> = particle.history.iter().cloned().collect();
        let num_points = positions.len();
        for i in 0..num_points - 1 {
            let start = positions[i];
            let end = positions[i + 1];
            let t = i as f32 / (num_points - 1) as f32;
            // Fade from White (head, t=0) to Blue (tail, t=1)
            let color = Color::srgb(1.0 - t * 0.5, 1.0 - t * 0.5, 1.0);
            gizmos.line(start, end, color);
        }
    }
}