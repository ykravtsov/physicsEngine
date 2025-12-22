use bevy::prelude::*;
use bevy::render::render_asset::Instance;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use crate::simulation::plasma::PlasmaParticle;

#[derive(Component, ShaderType, Clone)]
pub struct InstanceData {
    pub transform: Mat4,
    pub color: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct InstanceMaterial {}

impl bevy::pbr::Material for InstanceMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/instance.wgsl".into()
    }

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/instance.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            bevy::render::mesh::Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            bevy::render::mesh::Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            bevy::render::mesh::Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            Instance::<InstanceData>::ATTRIBUTE_BUFFER_ID.at_shader_location(3),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::pbr::MaterialPlugin::<InstanceMaterial>::default())
            .add_systems(Startup, setup_instance)
            .add_systems(Update, queue_instances);
    }
}

fn setup_instance(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<InstanceMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(0.1));
    let material = materials.add(InstanceMaterial {});
    commands.spawn((
        mesh,
        material,
        Instance::<InstanceData>::new(Vec::new()),
        Transform::default(),
    ));
}

fn queue_instances(
    mut query: Query<&mut Instance<InstanceData>>,
    particle_query: Query<(&PlasmaParticle, &Transform)>,
) {
    if let Ok(mut instance) = query.get_single_mut() {
        let mut data = Vec::new();
        for (particle, transform) in particle_query.iter() {
            data.push(InstanceData {
                transform: transform.compute_matrix(),
                color: particle.color.as_rgba_f32().into(),
            });
        }
        instance.set_data(data);
    }
}