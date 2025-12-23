use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};

const NUM_PARTICLES: usize = 1_000_000;
const WORKGROUP_SIZE: u32 = 64;
const PHI: f32 = 1.6180339887498948482;

#[derive(ShaderType, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Particle {
    pub pos: Vec4,
    pub vel: Vec4,
    pub color: Vec4,
}

#[derive(ShaderType, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct GalaxyUniforms {
    pub time: f32,
    pub dt: f32,
    pub pinch_strength: f32,
}

#[derive(Resource)]
pub struct GpuGalaxyResources {
    pub particle_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub compute_pipeline: ComputePipeline,
    pub bind_group: BindGroup,
}

#[derive(Component)]
pub struct GpuParticle {
    pub entity_index: usize,
}

pub struct GpuGalaxyPlugin;

impl Plugin for GpuGalaxyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_gpu_galaxy, spawn_gpu_particles))
            .add_systems(Update, (update_gpu_galaxy, update_particle_transforms));
    }
}

fn setup_gpu_galaxy(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
) {
    // Create particle buffer with initial spiral data
    let mut particles = Vec::with_capacity(NUM_PARTICLES);

    // Genesis Alignment: Initialize particles in stable spiral formation
    for i in 0..NUM_PARTICLES {
        let t = i as f32 / NUM_PARTICLES as f32;
        let r = 1.0 + t * 50.0; // Radius from 1 to 51
        let angle = t * 20.0 * std::f32::consts::PI + (i % 2) as f32 * std::f32::consts::PI; // Two arms

        let x = r * angle.cos();
        let z = r * angle.sin();
        let y = (rand::random::<f32>() - 0.5) * 2.0; // Small Y variation

        // Initial velocity tangent to the spiral
        let speed = 10.0 + rand::random::<f32>() * 5.0;
        let tangent_x = -z / r * speed;
        let tangent_z = x / r * speed;

        particles.push(Particle {
            pos: Vec4::new(x, y, z, 1.0),
            vel: Vec4::new(tangent_x, 0.0, tangent_z, 0.0),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0), // Initial white color
        });
    }

    let particle_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Particle Buffer"),
        contents: bytemuck::cast_slice(&particles),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    // Create uniform buffer
    let uniforms = GalaxyUniforms {
        time: 0.0,
        dt: 0.016, // ~60 FPS
        pinch_strength: 5.0,
    };

    let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Galaxy Uniforms"),
        contents: bytemuck::bytes_of(&uniforms),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    // Simplified setup - in a real implementation, this would need proper pipeline management
    // For now, we'll store the buffers and create pipelines in the update system
    let bind_group_layout = render_device.create_bind_group_layout(
        "Galaxy Bind Group Layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    );

    let bind_group = render_device.create_bind_group(
        "Galaxy Bind Group",
        &bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    );

    // Create compute pipeline
    let shader = render_device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Galaxy Compute Shader"),
        source: ShaderSource::Wgsl(include_str!("../../assets/shaders/galaxy_sim.wgsl").into()),
    });

    let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Galaxy Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = render_device.create_compute_pipeline(&RawComputePipelineDescriptor {
        label: Some("Galaxy Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
        compilation_options: PipelineCompilationOptions::default(),
    });

    commands.insert_resource(GpuGalaxyResources {
        particle_buffer,
        uniform_buffer,
        compute_pipeline,
        bind_group,
    });
}

fn update_gpu_galaxy(
    time: Res<Time>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut resources: ResMut<GpuGalaxyResources>,
) {
    // Update uniforms
    let uniforms = GalaxyUniforms {
        time: time.elapsed_seconds(),
        dt: time.delta_seconds(),
        pinch_strength: 5.0,
    };

    render_queue.write_buffer(&resources.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

    // Dispatch compute shader
    let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Galaxy Compute Encoder"),
    });

    {
        let mut compute_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Galaxy Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&resources.compute_pipeline);
        compute_pass.set_bind_group(0, &resources.bind_group, &[]);
        compute_pass.dispatch_workgroups((NUM_PARTICLES as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE, 1, 1);
    }

    render_queue.submit([command_encoder.finish()]);
}

fn spawn_gpu_particles(mut commands: Commands) {
    // Spawn a subset of particles for rendering (10,000 out of 1,000,000)
    const VISIBLE_PARTICLES: usize = 10_000;
    let step = NUM_PARTICLES / VISIBLE_PARTICLES;

    for i in 0..VISIBLE_PARTICLES {
        let particle_index = i * step;
        commands.spawn((
            GpuParticle { entity_index: particle_index },
            Transform::default(),
        ));
    }
}

fn update_particle_transforms(
    mut query: Query<(&GpuParticle, &mut Transform)>,
    resources: Res<GpuGalaxyResources>,
    render_device: Res<RenderDevice>,
) {
    // This is a simplified approach - in practice, you'd want to read back
    // the buffer data and update transforms
    // For now, we'll just demonstrate the concept

    // Note: Reading back 1M particles every frame would be very slow
    // A proper implementation would use GPU-only rendering
}