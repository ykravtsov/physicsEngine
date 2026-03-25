use bevy::prelude::*;
use bevy::time::{Timer, TimerMode};
use crate::simulation::plasma::update_galaxy_physics;
use crate::physics::math::FluxQuaternion;

pub struct GalaxyPlugin;

impl Plugin for GalaxyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_galaxy, setup_black_hole))
            .add_systems(Update, (
                rotate_black_hole,
                spawn_trail,
                update_stars,
                update_trail,
                update_galaxy_physics,
            ));
    }
}

#[derive(Component)]
pub struct Star {
    pub quaternion_state: FluxQuaternion,
}

#[derive(Component)]
pub struct BlackHole {
    #[allow(dead_code)]
    pub stored_mass: usize,
    #[allow(dead_code)]
    pub velocity: Vec3,
}


#[derive(Component)]
pub struct TrailPoint {
    pub timer: Timer,
}

fn setup_galaxy(mut commands: Commands) {
    // Spawn stars in random disk for emergent spiral behavior
    for _ in 0..10000 {
        let theta = rand::random::<f32>() * std::f32::consts::TAU;
        let r = rand::random::<f32>().sqrt() * 50.0;
        let y = (rand::random::<f32>() - 0.5) * 4.0;
        let pos = Vec3::new(
            r * theta.cos(),
            y,
            r * theta.sin(),
        );
        let tangent = Vec3::new(-pos.z, 0.0, pos.x).normalize();
        let speed = 15.0;
        let velocity = tangent * speed + Vec3::new(0.0, (rand::random::<f32>() - 0.5) * 0.5, 0.0);
        // Initialize quaternion state with velocity as vector part
        let quaternion_state = FluxQuaternion::new(1.0, velocity.x, velocity.y, velocity.z);
        commands.spawn((
            Star { quaternion_state },
            Transform::from_translation(pos),
        ));
    }
}

fn setup_black_hole(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere_mesh = meshes.add(Sphere::new(1.0));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.0, 0.0), // Black
        emissive: Color::srgb(1.0, 1.0, 1.0).into(), // White rim glow
        ..default()
    });

    commands.spawn((
        BlackHole {
            stored_mass: 0,
            velocity: Vec3::ZERO,
        },
        PbrBundle {
            mesh: sphere_mesh,
            material,
            transform: Transform::from_translation(Vec3::ZERO),
            ..default()
        },
    ));
}

const PHI: f32 = 1.618033988749;

pub fn rotate_black_hole(
    mut query: Query<&mut Transform, With<BlackHole>>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    for mut transform in query.iter_mut() {
        transform.rotate_y(PHI * dt * 0.1);
    }
}

fn spawn_trail(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    black_hole_query: Query<&Transform, With<BlackHole>>,
    time: Res<Time>,
    mut last_spawn: Local<f32>,
) {
    let sphere_mesh = meshes.add(Sphere::new(0.5));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, 0.3), // Faint black
        emissive: Color::srgba(1.0, 1.0, 1.0, 0.1).into(),
        ..default()
    });

    let black_hole_pos = black_hole_query.single().translation;

    // Spawn trail point every 0.1 seconds
    if time.elapsed_seconds() - *last_spawn > 0.1 {
        *last_spawn = time.elapsed_seconds();
        commands.spawn((
            TrailPoint {
                timer: Timer::from_seconds(5.0, TimerMode::Once),
            },
            PbrBundle {
                mesh: sphere_mesh,
                material,
                transform: Transform::from_translation(black_hole_pos),
                ..default()
            },
        ));
    }
}

fn update_trail(
    mut commands: Commands,
    mut query: Query<(Entity, &mut TrailPoint)>,
    time: Res<Time>,
) {
    for (entity, mut trail) in query.iter_mut() {
        trail.timer.tick(time.delta());
        if trail.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn update_stars(
    mut param_set: ParamSet<(
        Query<(&mut Star, &mut Transform)>,
        Query<&Transform, With<BlackHole>>,
    )>,
    time: Res<Time>,
) {
    let black_hole_pos = param_set.p1().single().translation;

    // Gravitational constant (tune to taste)
    let gm: f32 = 300.0;
    // Small inward "drag" fraction per second that keeps stars on spiraling paths
    // rather than pure circular orbits.  Value ~0.003 gives a gentle inward drift.
    let spiral_inward: f32 = 0.003;
    let max_speed: f32 = 40.0;

    for (mut star, mut transform) in param_set.p0().iter_mut() {
        let dt = time.delta_seconds();
        let pos = transform.translation;

        let to_center = black_hole_pos - pos;
        let dist = to_center.length().max(0.5);

        // FIX 1: Gravity = GM/r² (inverse-square), NOT 1/r.
        let grav_accel = to_center.normalize() * (gm / (dist * dist));

        // FIX 2: Differential rotation — Keplerian angular velocity ω = sqrt(GM/r³).
        // Each star gets its own rotation angle this frame so inner stars rotate faster.
        let omega = (gm / (dist * dist * dist)).sqrt(); // rad/s, decreases with r
        let angle = omega * dt;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        // Rotate around the galaxy's spin axis (Y-axis in this 3-D layout).
        let rot_y = FluxQuaternion::new(cos_a, 0.0, sin_a, 0.0);

        // FIX 3: Apply gravity as a direct velocity integration (preserves momentum).
        star.quaternion_state = star.quaternion_state.add_force(
            grav_accel.x, grav_accel.y, grav_accel.z, dt,
        );

        // Apply differential rotation to the velocity vector.
        star.quaternion_state = rot_y.mul(&star.quaternion_state);

        // FIX 4: Small inward radial component creates the logarithmic spiral.
        // Without this particles just orbit; with it they slowly wind inward like
        // a galaxy arm or hurricane band.
        let inward = to_center.normalize() * spiral_inward;
        star.quaternion_state = star.quaternion_state.add_force(
            inward.x, inward.y, inward.z, 1.0, // already scaled
        );

        // Clamp to max speed so centre doesn't blow up.
        let spd = star.quaternion_state.speed();
        if spd > max_speed {
            star.quaternion_state = star.quaternion_state.scale_velocity(max_speed / spd);
        }

        // UPDATE POSITION from velocity.
        let velocity_vec = Vec3::new(
            star.quaternion_state.x,
            star.quaternion_state.y,
            star.quaternion_state.z,
        );
        transform.translation += velocity_vec * dt;

        // Respawn stars that fall into the black hole centre.
        if dist < 1.2 {
            let theta = rand::random::<f32>() * std::f32::consts::TAU;
            let spawn_r = 20.0 + rand::random::<f32>() * 30.0;
            transform.translation = Vec3::new(
                spawn_r * theta.cos(),
                (rand::random::<f32>() - 0.5) * 4.0,
                spawn_r * theta.sin(),
            );
            let tangent = Vec3::new(-transform.translation.z, 0.0, transform.translation.x).normalize();
            let v = tangent * (gm / spawn_r).sqrt();
            star.quaternion_state = FluxQuaternion::new(1.0, v.x, v.y, v.z);
        }
    }
}
