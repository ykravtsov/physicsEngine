use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderDevice;
use bevy::render::render_asset::RenderAssets;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::SpecializedMeshPipelineError;
use crate::simulation::gpu_galaxy::{GpuGalaxyResources, ParticleCount};

#[derive(Clone, ShaderType)]
pub struct GpuGalaxyMaterialUniforms {
    pub particle_count: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GpuGalaxyMaterialUniforms)]
#[bind_group_layout(layout)]
pub struct GpuGalaxyMaterial {
    #[uniform(0)]
    pub particle_count: u32,
    #[storage(1, read_only)]
    pub particle_buffer: Buffer,
}

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
    gpu_resources: Res<GpuGalaxyResources>,
    particle_count: Res<ParticleCount>,
) {
    // Create a simple quad mesh (though we won't use its vertices)
    let mesh = Mesh::from(Quad::new(Vec2::splat(1.0)));
    let material = materials.add(GpuGalaxyMaterial {
        particle_count: particle_count.count as u32,
        particle_buffer: gpu_resources.particle_buffer.clone(),
    });

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
    particle_count: Res<ParticleCount>,
    query: Query<&Handle<GpuGalaxyMaterial>, With<GpuGalaxyRenderer>>,
) {
    if let Some(resources) = gpu_resources {
        // Update the material's particle_count uniform
        if let Ok(material_handle) = query.get_single() {
            if let Some(material) = materials.get_mut(material_handle) {
                material.particle_count = particle_count.count as u32;
            }
        }
        // The particle buffer is already set in the material during setup
    }
}