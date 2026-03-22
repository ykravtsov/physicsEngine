use wgpu::*;
use bytemuck::{Pod, Zeroable};
use crate::engine::gpu::GpuState;
use super::Particle;

/// GPU vertex for a single particle
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ParticleVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub size: f32,
}

/// Camera uniform buffer layout
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniforms {
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
}

pub struct ParticleRenderer {
    pub render_pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub max_particles: usize,
}

impl ParticleRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let max_particles = 100_000;

        // Camera uniform buffer
        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniforms"),
            size: std::mem::size_of::<CameraUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Camera BGL"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera BG"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Vertex buffer for particles
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Particle Vertex Buffer"),
            size: (max_particles * std::mem::size_of::<ParticleVertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Particle Shader"),
            source: ShaderSource::Wgsl(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/shaders/particle3d.wgsl")).into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Particle Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Particle Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<ParticleVertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute { offset: 0, shader_location: 0, format: VertexFormat::Float32x3 },
                        VertexAttribute { offset: 12, shader_location: 1, format: VertexFormat::Float32x4 },
                        VertexAttribute { offset: 28, shader_location: 2, format: VertexFormat::Float32 },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::One, // additive blending for glow
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent::OVER,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::PointList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            vertex_buffer,
            camera_buffer,
            camera_bind_group,
            max_particles,
        }
    }

    pub fn render(
        &self,
        gpu: &GpuState,
        particles: &[Particle],
        view: [[f32; 4]; 4],
        proj: [[f32; 4]; 4],
    ) {
        // Upload camera uniforms
        let camera_uniforms = CameraUniforms {
            view,
            proj,
            camera_pos: [0.0; 4],
        };
        gpu.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniforms));

        // Upload particle vertices
        let count = particles.len().min(self.max_particles);
        let vertices: Vec<ParticleVertex> = particles[..count].iter().map(|p| ParticleVertex {
            position: [p.x, p.y, p.z],
            color: [p.r, p.g, p.b, p.a],
            size: p.size,
        }).collect();
        gpu.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        // Render
        let output = match gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view_tex = output.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = gpu.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Particle Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view_tex,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color { r: 0.0, g: 0.0, b: 0.02, a: 1.0 }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..count as u32, 0..1);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
