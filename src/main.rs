use bevy::prelude::*;

mod physics;
mod simulation;

use simulation::camera::{FlyCamera, FlyCameraPlugin};
use simulation::emitter::EmitterPlugin;
use simulation::galaxy::GalaxyPlugin;
use simulation::gpu_galaxy::GpuGalaxyPlugin;
use simulation::plasma::PlasmaPlugin;
use simulation::render_settings::RenderSettingsPlugin;
// use simulation::rendering::RenderingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RenderSettingsPlugin)
        .add_plugins(FlyCameraPlugin)
        .add_plugins(GalaxyPlugin)
        .add_plugins(GpuGalaxyPlugin)
        .add_plugins(PlasmaPlugin)
        .add_plugins(EmitterPlugin)
        // .add_plugins(RenderingPlugin)
        .add_systems(Update, close_on_esc)
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    // 3D flycam
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        FlyCamera::default(),
    ));
}

fn close_on_esc(
    mut app_exit_events: EventWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        app_exit_events.send(AppExit::Success);
    }
}
