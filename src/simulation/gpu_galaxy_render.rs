use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderDevice;
use bevy::render::render_asset::RenderAssets;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::SpecializedMeshPipelineError;
use crate::simulation::gpu_galaxy::{GpuGalaxyResources, NUM_PARTICLES};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GpuGalaxyMaterial {}

impl bevy::pbr::Material for GpuGalaxyMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/gpu_galaxy_render.wgsl".into()
    }

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/gpu_galaxy_render.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // No vertex buffers needed since we generate vertices in shader
        descriptor.vertex.buffers = vec![];
        Ok(())
    }
}

#[derive(Component)]
pub struct GpuGalaxyRenderer;

pub struct GpuGalaxyRenderPlugin;

impl Plugin for GpuGalaxyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::pbr::MaterialPlugin::<GpuGalaxyMaterial>::default())
            .add_systems(Startup, setup_gpu_galaxy_render)
            .add_systems(Update, update_gpu_galaxy_render_bind_group);
    }
}

fn setup_gpu_galaxy_render(
    mut commands: Commands,
    mut materials: ResMut<Assets<GpuGalaxyMaterial>>,
) {
    // Create a simple quad mesh (though we won't use its vertices)
    let mesh = Mesh::from(Quad::new(Vec2::splat(1.0)));
    let material = materials.add(GpuGalaxyMaterial {});

    commands.spawn((
        GpuGalaxyRenderer,
        mesh,
        material,
        Transform::default(),
    ));
}

fn update_gpu_galaxy_render_bind_group(
    mut materials: ResMut<Assets<GpuGalaxyMaterial>>,
    gpu_resources: Option<Res<GpuGalaxyResources>>,
    render_device: Res<RenderDevice>,
) {
    if let Some(resources) = gpu_resources {
        // Update the material's bind group to include the particle buffer
        // This is a simplified approach - in practice, you might need a custom material extension
        // For now, we'll assume the material can access the buffer through a global bind group
    }
}