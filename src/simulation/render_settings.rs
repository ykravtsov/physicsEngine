use bevy::prelude::*;
use bevy::core_pipeline::bloom::BloomSettings;

pub struct RenderSettingsPlugin;

impl Plugin for RenderSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_render_settings);
    }
}

fn setup_render_settings(
    mut commands: Commands,
    mut cameras: Query<Entity, With<Camera>>,
) {
    for entity in cameras.iter_mut() {
        commands.entity(entity).insert(BloomSettings::default());
        // Note: HDR is already enabled by default on Camera3d, but to be explicit:
        // If needed, we can modify the camera component, but for now, assume it's set.
    }
}