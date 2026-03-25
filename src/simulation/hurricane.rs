use bevy::prelude::*;
use crate::physics::math::FluxQuaternion;

// Life-like Hurricane simulation using quaternion vortex field theory.
//
// Physics model:
// - FluxQuaternion.w = atmospheric pressure (low at eye, high outside)
// - FluxQuaternion.xyz = wind velocity in 2D (x,y) + vertical (z)
// - Coriolis effect: quaternion rotation around Z axis (Earth's spin, Northern Hemisphere)
// - Pressure gradient: inward suction wave toward low-pressure eye
// - Eye-wall: peak energy density ring where quaternion norm is maximum
// - Spiral bands: constructive interference of vortex waves (like galaxy arms)

const CORIOLIS_RATE: f32 = 0.08;       // Scaled Earth rotation (makes visible spiral)
const PRESSURE_GRADIENT: f32 = 120.0;  // Low pressure center strength
const DRAG: f32 = 0.015;               // Atmospheric friction
const EYE_RADIUS: f32 = 8.0;           // Calm eye radius
const OUTER_RADIUS: f32 = 200.0;       // Hurricane outer radius
const MAX_WIND_SPEED: f32 = 60.0;      // Max wind speed cap

pub struct HurricanePlugin;

impl Plugin for HurricanePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hurricane)
            .add_systems(Update, update_hurricane_particles);
    }
}

#[derive(Component)]
pub struct HurricaneParticle {
    pub flux: FluxQuaternion, // w=pressure, x=vx, y=vy, z=vz
}

fn setup_hurricane(mut commands: Commands) {
    // Spawn particles in a disk representing the atmosphere
    for _ in 0..30000 {
        let r = rand::random::<f32>().sqrt() * OUTER_RADIUS;
        let theta = rand::random::<f32>() * std::f32::consts::TAU;
        let x = r * theta.cos();
        let y = r * theta.sin();

        // Initial tangential wind velocity (counter-clockwise = Northern Hemisphere hurricane)
        let tangent = Vec2::new(-y, x).normalize_or_zero();

        // Wind speed profile: peaks at eye-wall, calm in eye, falls off outside
        let wind_speed = if r < EYE_RADIUS {
            0.0 // Calm eye
        } else {
            // Rankine vortex profile: v = v_max * (r_eye / r) for r > r_eye
            let v_max = 40.0;
            (v_max * EYE_RADIUS / r).min(v_max)
        };

        let wind = tangent * wind_speed;

        // Pressure: low at center (eye), high outside (Bernoulli)
        let pressure = (r / OUTER_RADIUS).clamp(0.0, 1.0);

        let flux = FluxQuaternion::new(pressure, wind.x, wind.y, 0.0);

        // Color: dark blue eye, bright cyan eye-wall, white outer bands
        let color = if r < EYE_RADIUS {
            Color::srgba(0.05, 0.05, 0.2, 0.9)
        } else if r < EYE_RADIUS * 3.0 {
            // Eye-wall: bright cyan
            Color::srgba(0.0, 0.9, 1.0, 1.0)
        } else {
            // Outer bands: white to light blue
            let t = ((r - EYE_RADIUS * 3.0) / (OUTER_RADIUS - EYE_RADIUS * 3.0)).clamp(0.0, 1.0);
            Color::srgba(0.7 + t * 0.3, 0.8 + t * 0.2, 1.0, 0.8 - t * 0.5)
        };

        commands.spawn((
            HurricaneParticle { flux },
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::splat(1.2)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 0.0),
                ..default()
            },
        ));
    }
}

fn update_hurricane_particles(
    mut query: Query<(&mut HurricaneParticle, &mut Transform, &mut Sprite)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds().min(0.02); // cap dt for stability

    for (mut particle, mut transform, mut sprite) in query.iter_mut() {
        let pos = transform.translation;
        let r = Vec2::new(pos.x, pos.y).length();

        // 1. PRESSURE GRADIENT FORCE (inward suction toward low-pressure eye)
        let to_center = Vec2::new(-pos.x, -pos.y);
        let pressure_force = if r > EYE_RADIUS && r < OUTER_RADIUS {
            let gradient = PRESSURE_GRADIENT * (1.0 - r / OUTER_RADIUS) / r.max(1.0);
            to_center.normalize_or_zero() * gradient
        } else if r <= EYE_RADIUS {
            -to_center.normalize_or_zero() * 10.0
        } else {
            Vec2::ZERO
        };

        // 2. CORIOLIS FORCE: -2Ω × v  (velocity-dependent, NOT a fixed rotation angle)
        // Real Coriolis scales with particle speed — fast wind deflects more than slow.
        // In 2-D, rotating the velocity vector by 90° and scaling gives the perpendicular
        // deflection: F_coriolis = 2Ω * |v| * v_perp_hat
        let vel = Vec2::new(particle.flux.x, particle.flux.y);
        let vel_speed = vel.length();
        // Perpendicular direction (counter-clockwise deflection = Northern Hemisphere)
        let vel_perp = Vec2::new(-vel.y, vel.x); // 90° CCW rotation of velocity
        let coriolis_force = vel_perp * (CORIOLIS_RATE * vel_speed);

        // 3. DRAG FORCE (atmospheric friction)
        let drag_force = -vel * DRAG;

        // 4. INTEGRATE all forces directly into velocity (preserves momentum/magnitude).
        // FIX: Previously used interact() which always normalised, killing the 1/r speed
        // profile. Now we just do plain Euler integration.
        let total_force = pressure_force + coriolis_force + drag_force;
        particle.flux = particle.flux.add_force(total_force.x, total_force.y, 0.0, dt);

        // Clamp speed
        let speed = Vec2::new(particle.flux.x, particle.flux.y).length();
        if speed > MAX_WIND_SPEED {
            let scale = MAX_WIND_SPEED / speed;
            particle.flux = FluxQuaternion::new(
                particle.flux.w,
                particle.flux.x * scale,
                particle.flux.y * scale,
                0.0,
            );
        }

        // 5. UPDATE POSITION from quaternion velocity
        transform.translation.x += particle.flux.x * dt;
        transform.translation.y += particle.flux.y * dt;
        transform.translation.z = 0.0;

        // 6. BOUNDARY: wrap particles that escape
        let new_r = Vec2::new(transform.translation.x, transform.translation.y).length();
        if new_r > OUTER_RADIUS * 1.1 {
            // Respawn at random position in outer band
            let theta = rand::random::<f32>() * std::f32::consts::TAU;
            let spawn_r = OUTER_RADIUS * (0.7 + rand::random::<f32>() * 0.3);
            transform.translation.x = spawn_r * theta.cos();
            transform.translation.y = spawn_r * theta.sin();
            transform.translation.z = 0.0;
        }

        // 7. UPDATE COLOR based on speed (energy density = quaternion norm)
        let energy = speed / MAX_WIND_SPEED;
        let new_r2 = Vec2::new(transform.translation.x, transform.translation.y).length();
        if new_r2 < EYE_RADIUS {
            sprite.color = Color::srgba(0.05, 0.05, 0.2, 0.9);
        } else if new_r2 < EYE_RADIUS * 3.0 {
            // Eye-wall: bright cyan-white (highest energy)
            sprite.color = Color::srgba(energy, 0.9, 1.0, 1.0);
        } else {
            // Outer bands: blue-white gradient
            let t = ((new_r2 - EYE_RADIUS * 3.0) / (OUTER_RADIUS - EYE_RADIUS * 3.0)).clamp(0.0, 1.0);
            sprite.color = Color::srgba(
                0.3 + energy * 0.7,
                0.6 + energy * 0.4,
                1.0,
                (0.9 - t * 0.6).max(0.1),
            );
        }
    }
}
