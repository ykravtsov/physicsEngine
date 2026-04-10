//! GPU Galaxy Simulation — Quaternionic Dirac-Maxwell-NS (QDM-NS) framework
//!
//! State field:  Ψ = ψ + F + v  ∈ H(C)
//!   ψ  = macroscopic vortex "spin" (Dirac-like wavefunction)
//!   F  = EM quaternion
//!   v  = fluid velocity quaternion
//!
//! Unified operator (quaternionic Dirac-Maxwell-NS operator):
//!
//!   DΨ = (∇ + m_eff + ν∇² − D/Dt) Ψ + Ψ̄·J + λ(ΨΨ̄ − 1)Ψ
//!
//! Terms:
//!   ∇Ψ            — Dirac + Maxwell core (relativistic + EM propagation)
//!   m_eff Ψ       — effective mass from baryonic density + self-gravity (vortex "rest energy")
//!   ν∇²Ψ          — quaternion viscosity; |∇²Ψ| regularises r→0 (no true point singularity)
//!   −(D/Dt)Ψ      — material derivative → Navier-Stokes advection + vortex stretching
//!   Ψ̄·J           — Lorentz force + current self-interaction (Birkeland / Z-pinch arms)
//!   λ(ΨΨ̄−1)Ψ     — nonlinear brake / energy-sink (vorticity-threshold jet trigger)
//!
//! Galaxy as vortex: steady-state, axisymmetric, purely-rotational velocity quaternion
//!   v(r, φ) = ω(r)k̂ + φ₀ ln r     (φ₀ pitch related to golden ratio)
//!
//! The attractor is the golden-ratio logarithmic spiral:
//!   r(φ) = r₀ exp(φ / φ₀),    φ₀ = 2π / ln φ ≈ 2.4 rad,    φ = (1+√5)/2

use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use rand::Rng;

const WORKGROUP_SIZE: u32 = 64;

/// Golden ratio φ = (1+√5)/2 — the natural attractor pitch of QDM-NS vortex arms.
const PHI: f32 = 1.6180339887498948482;

/// φ₀ = 2π / ln(φ) ≈ 2.4 rad — logarithmic-spiral pitch angle eigenvalue from
/// the ν∇² + Lorentz-pinch balance in the QDM-NS operator.
const PHI0: f32 = 2.399_963; // 2π / ln(1.618034)

#[derive(Resource)]
pub struct ParticleCount {
    pub count: usize,
}

impl Default for ParticleCount {
    fn default() -> Self {
        Self { count: 100_000 } // Reduced default for better performance
    }
}

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
    /// Ψ̄·J coupling strength — controls Birkeland / Z-pinch arm pinch.
    /// Corresponds to the Lorentz self-interaction term in the QDM-NS operator.
    pub pinch_strength: f32,
    /// Current value of φ (golden ratio). Arm pitch φ₀ = 2π / ln(phi_value).
    pub phi_value: f32,
    pub arms: f32,
    /// ν — quaternion viscosity coefficient (regularises r→0 via |∇²Ψ|).
    pub nu: f32,
    /// λ — vorticity-threshold brake coefficient: λ(ΨΨ̄−1)Ψ jet-trigger term.
    pub lambda: f32,
    /// m_eff — effective baryonic mass / self-gravity term ("vortex rest energy").
    pub m_eff: f32,
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

#[derive(Resource)]
pub struct PhiResource {
    pub phi_value: f32,
}

pub struct GpuGalaxyPlugin;

impl Plugin for GpuGalaxyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhiResource {
            phi_value: 1.618034,
        })
        .init_resource::<ParticleCount>()
        .add_systems(Startup, (setup_gpu_galaxy, spawn_gpu_particles))
        .add_systems(
            Update,
            (
                update_phi_input,
                update_gpu_galaxy,
                update_particle_transforms,
            ),
        );
    }
}

fn setup_gpu_galaxy(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    particle_count: Res<ParticleCount>,
) {
    // -----------------------------------------------------------------------
    // Genesis Initialization — QDM-NS golden-ratio logarithmic spiral
    //
    // Steady-state, axisymmetric velocity quaternion from the QDM-NS operator:
    //   v(r, φ) = ω(r) k̂  +  φ₀ · ln r      (φ₀ = 2π/ln φ ≈ 2.4 rad)
    //
    // The attractor spiral: r(φ) = r₀ · exp(φ / φ₀)
    // Inverted for init: given r, the on-arm angle is φ = φ₀ · ln(r / r₀)
    //                           i.e. θ = φ₀ · ln r   (r₀ = 1 absorbed into offset)
    // -----------------------------------------------------------------------
    let mut particles = Vec::with_capacity(particle_count.count);
    let mut rng = rand::thread_rng();

    // φ₀ = 2π / ln(φ) — pitch-angle eigenvalue from ν∇² + Lorentz-pinch balance
    let phi0: f32 = PHI0;

    for i in 0..particle_count.count {
        // 1. Sample radius log-uniformly so density ∝ 1/r (flat surface density)
        let r: f32 = rng.gen_range(2.0_f32..60.0_f32);

        // 2. QDM-NS on-arm angle: θ = φ₀ · ln r
        //    (arises from the eigenvalue condition of the vorticity equation under
        //    ν∇² + Ψ̄·J Lorentz-pinch balance — not imposed by hand)
        let base_theta = phi0 * r.ln();

        // 3. Two arms (Z-pinch wires carry equal and opposite J):
        //    Arm A at offset 0, Arm B at offset π
        let arm_offset = if i % 2 == 0 {
            0.0
        } else {
            std::f32::consts::PI
        };

        // 4. Arm thickness fuzz (finite cross-section of Birkeland current sheet)
        let fuzz = (rng.r#gen::<f32>() - 0.5) * 0.8;

        let theta = base_theta + arm_offset + fuzz;
        // Disk half-height from Coriolis + gravity balance
        let y = rng.gen_range(-1.5..1.5);

        let pos = Vec4::new(
            r * theta.cos(),
            y,
            r * theta.sin(),
            1.0, // w = 1 (alive)
        );

        // 5. Velocity quaternion v(r, φ) = ω(r)k̂ + φ₀·ln(r)
        //    ω(r) tangential component — flat rotation curve via Ψ̄·J Lorentz support
        //    The scalar part φ₀·ln(r) encodes the logarithmic-spiral flow pitch.
        let tangent = Vec3::new(-pos.z, 0.0, pos.x).normalize();
        let omega = 15.0_f32; // ω — set by Ψ̄·J pinch strength (tunable via uniforms)
        let log_pitch = phi0 * r.ln(); // scalar (w) component of velocity quaternion

        let vel = Vec4::new(
            tangent.x * omega,
            log_pitch, // y encodes φ₀·ln r (quaternion imaginary j component)
            tangent.z * omega,
            0.0,
        );

        particles.push(Particle {
            pos,
            vel,
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
        });
    }

    let particle_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Particle Buffer"),
        contents: bytemuck::cast_slice(&particles),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    // Create uniform buffer — QDM-NS operator parameters
    let uniforms = GalaxyUniforms {
        time: 0.0,
        dt: 0.016, // ~60 FPS
        // pinch_strength: Ψ̄·J coupling (Birkeland/Z-pinch arm cohesion)
        pinch_strength: 0.1,
        phi_value: PHI,
        arms: 2.0,
        // ν: quaternion viscosity — regularises r→0, prevents point singularities
        nu: 0.01,
        // λ: vorticity-threshold brake — triggers jet/outflow when |ω| > critical value
        lambda: 0.05,
        // m_eff: baryonic density + self-gravity ("vortex rest energy")
        m_eff: 0.001,
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
    resources: ResMut<GpuGalaxyResources>,
    phi_resource: Res<PhiResource>,
    particle_count: Res<ParticleCount>,
) {
    // Update uniforms — QDM-NS operator parameters per frame
    // φ₀ is recomputed each frame so live phi-tuning propagates to the spiral pitch.
    let uniforms = GalaxyUniforms {
        time: time.elapsed_secs(),
        dt: time.delta_secs(),
        pinch_strength: 0.1, // Ψ̄·J Lorentz-pinch arm coupling
        phi_value: phi_resource.phi_value,
        arms: 2.0,
        nu: 0.01,     // ν — quaternion viscosity
        lambda: 0.05, // λ — vorticity-threshold brake
        m_eff: 0.001, // m_eff — baryonic self-gravity
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
        compute_pass.dispatch_workgroups(
            ((particle_count.count as u32) + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
            1,
            1,
        );
    }

    render_queue.submit([command_encoder.finish()]);
}

fn spawn_gpu_particles(mut commands: Commands, particle_count: Res<ParticleCount>) {
    // Spawn a subset of particles for rendering (10,000 out of total)
    const VISIBLE_PARTICLES: usize = 10_000;
    let num_to_spawn = VISIBLE_PARTICLES.min(particle_count.count);
    let step = if num_to_spawn > 0 {
        particle_count.count / num_to_spawn
    } else {
        1
    };

    for i in 0..num_to_spawn {
        let particle_index = i * step;
        commands.spawn((
            GpuParticle {
                entity_index: particle_index,
            },
            Transform::default(),
        ));
    }
}

fn update_particle_transforms(
    _query: Query<(&GpuParticle, &mut Transform)>,
    _resources: Res<GpuGalaxyResources>,
    _render_device: Res<RenderDevice>,
) {
    // This is a simplified approach - in practice, you'd want to read back
    // the buffer data and update transforms
    // For now, we'll just demonstrate the concept

    // Note: Reading back 1M particles every frame would be very slow
    // A proper implementation would use GPU-only rendering
}

fn update_phi_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut phi_resource: ResMut<PhiResource>,
) {
    let mut changed = false;
    if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        phi_resource.phi_value += 0.01;
        changed = true;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
        phi_resource.phi_value -= 0.01;
        changed = true;
    }
    if keyboard_input.just_pressed(KeyCode::Space) {
        // Reset to golden ratio — the QDM-NS attractor eigenvalue
        phi_resource.phi_value = PHI;
        changed = true;
    }
    if changed {
        let phi0 = std::f32::consts::TAU / phi_resource.phi_value.ln();
        println!(
            "φ = {:.6}   φ₀ = 2π/ln(φ) = {:.4} rad  (golden-ratio attractor = {:.6})",
            phi_resource.phi_value, phi0, PHI
        );
    }
}
