use bevy::prelude::*;
use std::collections::VecDeque;
use crate::simulation::galaxy::BlackHole;

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

const PHI: f32 = 1.618033988749;
const PINCH_STRENGTH: f32 = 10.0;
const DAMPING: f32 = 5.0;

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
) {
    let black_hole_pos = param_set.p1().single().translation;
    let dt = time.delta_seconds();
    let galaxy_angle = time.elapsed_seconds() * 0.1;

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
                // Z-Pinch Logic
                let arm_offset = if particle.arm == 0 { 0.0 } else { std::f32::consts::PI };
                let angle = (particle.original_radius.ln() / 0.3) + arm_offset + galaxy_angle;
                let ideal_pos = black_hole_pos + Vec3::new(particle.original_radius * angle.cos(), 0.0, particle.original_radius * angle.sin());

                // Apply force towards ideal position with damping
                let pinch_vector = ideal_pos - pos;
                let pinch_force = pinch_vector * PINCH_STRENGTH;
                let drag_force = -particle.velocity * 0.1;
                let force = pinch_force + drag_force;
                particle.velocity += force * dt;
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