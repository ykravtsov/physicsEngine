pub mod renderer;

use crate::quaternion::math::FluxQuaternion;

const CORIOLIS_RATE: f32 = 0.08;
const PRESSURE_GRADIENT: f32 = 120.0;
const DRAG: f32 = 0.015;
const EYE_RADIUS: f32 = 15.0;
const OUTER_RADIUS: f32 = 250.0;
const MAX_WIND_SPEED: f32 = 80.0;
const HURRICANE_HEIGHT: f32 = 120.0;

// Air viscosity: couples neighboring particles' velocities (diffusion)
const VISCOSITY: f32 = 0.08;

// Harmonic series: multiple spiral arms at different frequencies
// Models the mixing of warm/cold air layers
const HARMONIC_ARMS: &[(f32, f32)] = &[
    (1.0, 1.0),   // fundamental: main spiral
    (2.0, 0.4),   // 2nd harmonic: inner spiral bands
    (3.0, 0.2),   // 3rd harmonic: fine structure
    (0.5, 0.3),   // sub-harmonic: large outer band
];

/// A single hurricane particle with quaternion state
#[derive(Clone, Copy)]
pub struct Particle {
    pub x: f32, pub y: f32, pub z: f32,   // position
    pub flux: FluxQuaternion,              // w=pressure, xyz=velocity
    pub r: f32, pub g: f32, pub b: f32, pub a: f32, // color
    pub size: f32,
}

pub struct HurricaneSimulation {
    pub particles: Vec<Particle>,
}

impl HurricaneSimulation {
    pub fn new(count: usize) -> Self {
        let mut particles = Vec::with_capacity(count);

        for _ in 0..count {
            let r = rand::random::<f32>().sqrt() * OUTER_RADIUS;
            let theta = rand::random::<f32>() * std::f32::consts::TAU;
            let x = r * theta.cos();
            let z = r * theta.sin();

            // Vertical distribution: taller near eye-wall, thin at edges
            let height_scale = if r < EYE_RADIUS {
                0.1 // calm eye, low particles
            } else if r < EYE_RADIUS * 3.0 {
                1.0 // eye-wall: full height
            } else {
                1.0 - (r - EYE_RADIUS * 3.0) / (OUTER_RADIUS - EYE_RADIUS * 3.0)
            };
            let y = (rand::random::<f32>() - 0.3) * HURRICANE_HEIGHT * height_scale;

            // Rankine vortex wind profile
            let wind_speed = if r < EYE_RADIUS {
                0.0
            } else {
                let v_max = 50.0;
                (v_max * EYE_RADIUS / r).min(v_max)
            };

            // Tangential wind (counter-clockwise in XZ plane)
            let tx = -z / r.max(0.01);
            let tz = x / r.max(0.01);

            // Updraft in eye-wall, downdraft in eye
            let vy = if r < EYE_RADIUS {
                -5.0 // downdraft in eye
            } else if r < EYE_RADIUS * 2.5 {
                15.0 * (1.0 - (r - EYE_RADIUS) / (EYE_RADIUS * 1.5)) // updraft in eye-wall
            } else {
                -2.0 // gentle downdraft in outer bands
            };

            let pressure = (r / OUTER_RADIUS).clamp(0.0, 1.0);
            let flux = FluxQuaternion::new(pressure, tx * wind_speed, vy, tz * wind_speed);

            // Color: dark blue eye, bright cyan eye-wall, white outer bands
            let (cr, cg, cb, ca) = if r < EYE_RADIUS {
                (0.05, 0.05, 0.3, 0.6)
            } else if r < EYE_RADIUS * 2.5 {
                let t = (r - EYE_RADIUS) / (EYE_RADIUS * 1.5);
                (t * 0.3, 0.7 + t * 0.3, 1.0, 1.0)
            } else {
                let t = ((r - EYE_RADIUS * 2.5) / (OUTER_RADIUS - EYE_RADIUS * 2.5)).clamp(0.0, 1.0);
                (0.5 + t * 0.5, 0.7 + t * 0.3, 1.0, 0.8 - t * 0.6)
            };

            particles.push(Particle {
                x, y, z,
                flux,
                r: cr, g: cg, b: cb, a: ca,
                size: if r < EYE_RADIUS * 2.5 { 1.5 } else { 1.0 },
            });
        }

        Self { particles }
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.min(0.02);

        for p in &mut self.particles {
            let r = (p.x * p.x + p.z * p.z).sqrt();

            // 1. PRESSURE GRADIENT (inward suction in XZ plane)
            let to_center_x = -p.x;
            let to_center_z = -p.z;
            let (pfx, pfz) = if r > EYE_RADIUS && r < OUTER_RADIUS {
                let gradient = PRESSURE_GRADIENT * (1.0 - r / OUTER_RADIUS) / r.max(1.0);
                let len = (to_center_x * to_center_x + to_center_z * to_center_z).sqrt().max(1e-6);
                (to_center_x / len * gradient, to_center_z / len * gradient)
            } else if r <= EYE_RADIUS {
                let len = (to_center_x * to_center_x + to_center_z * to_center_z).sqrt().max(1e-6);
                (-to_center_x / len * 10.0, -to_center_z / len * 10.0)
            } else {
                (0.0, 0.0)
            };

            // 2. TANGENTIAL FORCE: the key to vortex rotation
            let tangent_x = -p.z / r.max(0.01);
            let tangent_z = p.x / r.max(0.01);
            let tangential_strength = if r > EYE_RADIUS && r < OUTER_RADIUS {
                let v_max = 60.0;
                v_max * EYE_RADIUS / r.max(EYE_RADIUS)
            } else {
                0.0
            };
            let tfx = tangent_x * tangential_strength;
            let tfz = tangent_z * tangential_strength;

            // 3. HARMONIC SERIES: multiple spiral frequencies mix warm/cold air
            // Each harmonic adds a tangential perturbation at a different frequency
            // This creates the banded structure of real hurricanes
            let theta = p.z.atan2(p.x); // angular position
            let mut harmonic_x = 0.0f32;
            let mut harmonic_z = 0.0f32;
            for &(freq, amp) in HARMONIC_ARMS {
                let phase = theta * freq;
                // Tangential perturbation at this harmonic frequency
                harmonic_x += -p.z / r.max(0.01) * amp * 8.0 * phase.sin();
                harmonic_z += p.x / r.max(0.01) * amp * 8.0 * phase.sin();
            }

            // 4. AIR VISCOSITY: couples velocity to local mean field
            // Approximated as: viscosity * (target_tangential_velocity - current_velocity)
            // This keeps the vortex coherent and prevents particles from flying apart
            let target_vx = tangent_x * tangential_strength;
            let target_vz = tangent_z * tangential_strength;
            let viscosity_x = (target_vx - p.flux.x) * VISCOSITY;
            let viscosity_z = (target_vz - p.flux.z) * VISCOSITY;

            // 5. DRAG
            let drag_x = -p.flux.x * DRAG;
            let drag_y = -p.flux.y * DRAG;
            let drag_z = -p.flux.z * DRAG;

            // 6. GRAVITY
            let gravity_y = -p.y * 0.05;

            // 7. INTEGRATE ALL FORCES
            p.flux.x += (pfx + tfx + harmonic_x + viscosity_x + drag_x) * dt;
            p.flux.y += (drag_y + gravity_y) * dt;
            p.flux.z += (pfz + tfz + harmonic_z + viscosity_z + drag_z) * dt;

            // 8. CORIOLIS: rotate velocity in XZ plane each frame
            let coriolis_angle = CORIOLIS_RATE * dt;
            let cos_c = coriolis_angle.cos();
            let sin_c = coriolis_angle.sin();
            let new_vx = p.flux.x * cos_c - p.flux.z * sin_c;
            let new_vz = p.flux.x * sin_c + p.flux.z * cos_c;
            p.flux.x = new_vx;
            p.flux.z = new_vz;

            // Cap speed
            let speed = (p.flux.x * p.flux.x + p.flux.y * p.flux.y + p.flux.z * p.flux.z).sqrt();
            if speed > MAX_WIND_SPEED {
                let scale = MAX_WIND_SPEED / speed;
                p.flux.x *= scale;
                p.flux.y *= scale;
                p.flux.z *= scale;
            }

            // 7. UPDATE POSITION
            p.x += p.flux.x * dt;
            p.y += p.flux.y * dt;
            p.z += p.flux.z * dt;

            // 7. BOUNDARY: respawn escaped particles
            let new_r = (p.x * p.x + p.z * p.z).sqrt();
            if new_r > OUTER_RADIUS * 1.1 || p.y.abs() > HURRICANE_HEIGHT * 1.5 {
                let theta = rand::random::<f32>() * std::f32::consts::TAU;
                let spawn_r = OUTER_RADIUS * (0.6 + rand::random::<f32>() * 0.4);
                p.x = spawn_r * theta.cos();
                p.z = spawn_r * theta.sin();
                p.y = (rand::random::<f32>() - 0.5) * HURRICANE_HEIGHT * 0.5;
            }

            // 8. UPDATE COLOR based on energy
            let energy = speed / MAX_WIND_SPEED;
            let new_r2 = (p.x * p.x + p.z * p.z).sqrt();
            if new_r2 < EYE_RADIUS {
                p.r = 0.05; p.g = 0.05; p.b = 0.3; p.a = 0.6;
            } else if new_r2 < EYE_RADIUS * 2.5 {
                p.r = energy * 0.5; p.g = 0.7 + energy * 0.3; p.b = 1.0; p.a = 1.0;
            } else {
                let t = ((new_r2 - EYE_RADIUS * 2.5) / (OUTER_RADIUS - EYE_RADIUS * 2.5)).clamp(0.0, 1.0);
                p.r = 0.4 + energy * 0.6; p.g = 0.6 + energy * 0.4; p.b = 1.0;
                p.a = (0.8 - t * 0.6).max(0.05);
            }
        }
    }
}
