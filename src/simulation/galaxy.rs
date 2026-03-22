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
    // The Drag Constant (Phi^-4)
    const PHI_INV_4: f32 = 0.1464466094067262;
    let cos_mu = PHI_INV_4.cos();
    let sin_mu = PHI_INV_4.sin();

    // The "Black Hole" Suction Strength (Gravity)
    let suction_strength = 5.0;

    for (mut star, mut transform) in param_set.p0().iter_mut() {
        let dt = time.delta_seconds();
        let pos = transform.translation;

        // 1. CALCULATE SUCTION (Gravity) as quaternion wave
        let to_center = black_hole_pos - pos;
        let dist_sq = to_center.length_squared();

        // In QQM, suction is high near the drain - represent as quaternion wave
        let suction_wave = if dist_sq > 0.1 {
            let suction_vector = to_center.normalize() * (suction_strength / pos.length().max(1.0));
            FluxQuaternion::new(0.0, suction_vector.x, suction_vector.y, suction_vector.z)
        } else {
            FluxQuaternion::new(0.0, 0.0, 0.0, 0.0)
        };

        // 2. APPLY GOLDEN DRAG (The Spin) as quaternion rotation
        let drag_rotation = FluxQuaternion::new(cos_mu, 0.0, 0.0, sin_mu);

        // Update quaternion state with wave interactions
        star.quaternion_state = star.quaternion_state.interact(&suction_wave);
        star.quaternion_state = drag_rotation.mul(&star.quaternion_state);

        // 3. UPDATE POSITION using quaternion velocity vector
        let velocity_vec = Vec3::new(star.quaternion_state.x, star.quaternion_state.y, star.quaternion_state.z);
        transform.translation += velocity_vec * dt;
    }
}
