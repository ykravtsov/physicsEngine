//! GPU galaxy: the `src/galaxy.rs` force law in a compute shader, drawn as
//! soft sprites with a bloom pass. `--cpu` integrates on the CPU and only
//! uses the GPU to draw.

use crate::galaxy::{CurveCheck, GalaxyConfig, GalaxySimulation, curve_check};
use crate::hurricane_3d::Particle;
use crate::quaternion::camera::QuaternionCamera;
use crate::quaternion::math::FluxQuaternion;
use bytemuck::{Pod, Zeroable};
use std::time::Instant;
use wgpu::util::DeviceExt;
use wgpu::*;
use winit::window::Window;

pub const DEFAULT_STARS: usize = 1_048_576;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct StarGpu {
    pos: [f32; 4],
    vel: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SimUniforms {
    dt: f32,
    pattern_angle: f32,
    a0: f32,
    bhx: f32,
    bhy: f32,
    bhz: f32,
    flags: u32,
    n: u32,
    seed: u32,
    frame: u32,
    dust: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniforms {
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    viewport: [f32; 4],
    /// x star brightness, y dust-mask gain. Both fall with particle count so
    /// a million sprites stay colored instead of washing out to white.
    extra: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BloomParams {
    texel: [f32; 2],
    offset: f32,
    mode: f32,
    exposure: f32,
    bloom: f32,
    dust: f32,
    _pad: f32,
}

const FLAG_MOND: u32 = 1;
const FLAG_VORTEX: u32 = 2;
const FLAG_LEGACY: u32 = 4;

pub struct Launch {
    pub stars: usize,
    pub seed: u64,
    pub cpu: bool,
    pub dust: bool,
    pub config: GalaxyConfig,
    pub shot: Option<String>,
    pub benchmark_frames: u32,
    pub width: u32,
    pub height: u32,
}

impl Launch {
    pub fn from_args(args: &[String]) -> Self {
        let stars = arg_u64(args, "--stars").unwrap_or(DEFAULT_STARS as u64) as usize;
        Self {
            stars: stars.max(64),
            seed: arg_u64(args, "--seed").unwrap_or(1),
            cpu: args.iter().any(|a| a == "--cpu"),
            dust: !args.iter().any(|a| a == "--no-dust"),
            config: GalaxyConfig::from_args(args),
            shot: arg_str(args, "--shot"),
            benchmark_frames: arg_u64(args, "--benchmark").unwrap_or(0) as u32,
            width: arg_u64(args, "--width").unwrap_or(1280) as u32,
            height: arg_u64(args, "--height").unwrap_or(720) as u32,
        }
    }
}

fn arg_str(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn arg_u64(args: &[String], flag: &str) -> Option<u64> {
    arg_str(args, flag).and_then(|s| s.parse().ok())
}

struct Orbit {
    yaw: f32,
    tilt: f32,
    dist: f32,
    yaw_t: f32,
    tilt_t: f32,
    dist_t: f32,
    dragging: bool,
}

impl Orbit {
    fn new() -> Self {
        let tilt = 18.0_f32.to_radians();
        let dist = 820.0;
        Self {
            yaw: 0.62,
            tilt,
            dist,
            yaw_t: 0.62,
            tilt_t: tilt,
            dist_t: dist,
            dragging: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn smooth(&mut self, dt: f32) {
        let k = 1.0 - (-12.0 * dt).exp();
        self.yaw += (self.yaw_t - self.yaw) * k;
        self.tilt += (self.tilt_t - self.tilt) * k;
        self.dist += (self.dist_t - self.dist) * k;
    }

    fn eye(&self) -> (f32, f32, f32) {
        let h = self.dist * self.tilt.sin();
        let y = self.dist * self.tilt.cos();
        (h * self.yaw.sin(), y, h * self.yaw.cos())
    }
}

struct Targets {
    // Textures stay owned so the views remain valid.
    #[allow(dead_code)]
    hdr: Texture,
    hdr_view: TextureView,
    #[allow(dead_code)]
    bloom_a: Texture,
    view_a: TextureView,
    #[allow(dead_code)]
    bloom_b: Texture,
    view_b: TextureView,
    #[allow(dead_code)]
    dust: Texture,
    dust_view: TextureView,
    #[allow(dead_code)]
    dummy: Texture,
    extract_bg: BindGroup,
    blur_ab: BindGroup,
    blur_ba: BindGroup,
    blur_ab2: BindGroup,
    composite_bg: BindGroup,
}

pub struct GalaxyApp {
    pub adapter_name: String,
    /// Kept alive for the lifetime of `surface`.
    #[allow(dead_code)]
    instance: Instance,
    device: Device,
    queue: Queue,
    surface: Option<Surface<'static>>,
    surface_format: TextureFormat,
    alpha_mode: CompositeAlphaMode,
    size: (u32, u32),
    stars_buf: Buffer,
    #[allow(dead_code)]
    staging: Buffer,
    sim_buf: Buffer,
    view_stars: Buffer,
    view_dust: Buffer,
    n: u32,
    frame: u32,
    seed: u32,
    dust: bool,
    pub cpu: bool,
    sim: GalaxySimulation,
    orbit: Orbit,
    compute_pipeline: ComputePipeline,
    compute_bg: BindGroup,
    sprite_pipeline: RenderPipeline,
    sprite_bg_stars: BindGroup,
    sprite_bg_dust: BindGroup,
    bloom_pipeline: RenderPipeline,
    targets: Option<Targets>,
    bloom_layout: BindGroupLayout,
    bloom_sampler: Sampler,
    composite_pipeline: Option<RenderPipeline>,
}

pub struct Bench {
    pub adapter: String,
    pub stars: u32,
    pub compute_fps: f32,
    pub frame_fps: f32,
    pub compute_ms: f32,
    pub frame_ms: f32,
}

fn rank_device(t: DeviceType) -> u8 {
    match t {
        DeviceType::DiscreteGpu => 0,
        DeviceType::IntegratedGpu => 1,
        DeviceType::VirtualGpu => 2,
        DeviceType::Cpu => 3,
        DeviceType::Other => 4,
    }
}

fn open_adapter(instance: &Instance, surface: Option<&Surface<'_>>) -> Adapter {
    let mut adapters = instance.enumerate_adapters(Backends::all());
    if adapters.is_empty() {
        panic!("no Vulkan/GPU adapter (install mesa-vulkan-drivers for lavapipe)");
    }
    adapters.sort_by_key(|a| {
        let unsupported = match surface {
            Some(surface) => !a.is_surface_supported(surface),
            None => false,
        };
        (unsupported, rank_device(a.get_info().device_type))
    });
    adapters.swap_remove(0)
}

impl GalaxyApp {
    pub fn offscreen(launch: &Launch) -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });
        let adapter = open_adapter(&instance, None);
        Self::build(
            instance,
            adapter,
            None,
            launch,
            launch.width.max(2),
            launch.height.max(2),
        )
    }

    pub fn windowed(window: &'static Window, launch: &Launch) -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).expect("create surface");
        let adapter = open_adapter(&instance, Some(&surface));
        let size = window.inner_size();
        Self::build(
            instance,
            adapter,
            Some(surface),
            launch,
            size.width.max(2),
            size.height.max(2),
        )
    }

    fn build(
        instance: Instance,
        adapter: Adapter,
        surface: Option<Surface<'static>>,
        launch: &Launch,
        width: u32,
        height: u32,
    ) -> Self {
        let info = adapter.get_info();
        let adapter_name = format!("{} ({:?})", info.name, info.device_type);
        println!("GPU adapter: {adapter_name}");

        let alimits = adapter.limits();
        let need = (launch.stars * std::mem::size_of::<StarGpu>()) as u32;
        if alimits.max_storage_buffer_binding_size < need {
            panic!(
                "adapter storage limit {} is below {need} bytes for {} stars",
                alimits.max_storage_buffer_binding_size, launch.stars
            );
        }
        let features = if adapter.features().contains(Features::FLOAT32_FILTERABLE) {
            Features::FLOAT32_FILTERABLE
        } else {
            Features::empty()
        };
        let filter_linear = features.contains(Features::FLOAT32_FILTERABLE);
        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: Some("galaxy-gpu"),
                required_features: features,
                required_limits: alimits,
                memory_hints: MemoryHints::default(),
            },
            None,
        ))
        .expect("request device");

        let (surface_format, alpha_mode) = if let Some(surface) = &surface {
            let caps = surface.get_capabilities(&adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let alpha_mode = caps.alpha_modes[0];
            let config = SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode: PresentMode::AutoVsync,
                alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);
            (format, alpha_mode)
        } else {
            (TextureFormat::Rgba8UnormSrgb, CompositeAlphaMode::Opaque)
        };

        println!(
            "spawning {} stars ({})",
            launch.stars,
            if launch.cpu {
                "CPU physics"
            } else {
                "GPU physics"
            }
        );
        let mut sim = GalaxySimulation::with_config(launch.stars, launch.config, Some(launch.seed));
        sim.verbose = false;
        let packed = pack_stars(&sim.particles, launch.dust);
        let n = packed.len() as u32;
        if !launch.cpu {
            sim.particles.clear();
            sim.particles.shrink_to_fit();
        }

        let stars_buf = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("stars"),
            contents: bytemuck::cast_slice(&packed),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let staging = device.create_buffer(&BufferDescriptor {
            label: Some("star-readback"),
            size: stars_buf.size(),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sim_buf = device.create_buffer(&BufferDescriptor {
            label: Some("sim-uniforms"),
            size: std::mem::size_of::<SimUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_stars = device.create_buffer(&BufferDescriptor {
            label: Some("view-stars"),
            size: std::mem::size_of::<ViewUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_dust = device.create_buffer(&BufferDescriptor {
            label: Some("view-dust"),
            size: std::mem::size_of::<ViewUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("galaxy-compute"),
            source: ShaderSource::Wgsl(
                include_str!("../assets/shaders/galaxy_compute.wgsl").into(),
            ),
        });
        let sprite_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("galaxy-sprite"),
            source: ShaderSource::Wgsl(include_str!("../assets/shaders/galaxy_sprite.wgsl").into()),
        });
        let bloom_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("galaxy-bloom"),
            source: ShaderSource::Wgsl(include_str!("../assets/shaders/galaxy_bloom.wgsl").into()),
        });

        let compute_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("compute-bgl"),
            entries: &[
                storage_entry(0, true, ShaderStages::COMPUTE),
                uniform_entry(1, ShaderStages::COMPUTE),
            ],
        });
        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("galaxy-compute"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("compute-pl"),
                bind_group_layouts: &[&compute_layout],
                push_constant_ranges: &[],
            })),
            module: &compute_shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });
        let compute_bg = device.create_bind_group(&BindGroupDescriptor {
            label: Some("compute-bg"),
            layout: &compute_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: stars_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: sim_buf.as_entire_binding(),
                },
            ],
        });

        let sprite_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("sprite-bgl"),
            entries: &[
                storage_entry(0, false, ShaderStages::VERTEX | ShaderStages::FRAGMENT),
                uniform_entry(1, ShaderStages::VERTEX | ShaderStages::FRAGMENT),
            ],
        });
        let sprite_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("sprites"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("sprite-pl"),
                bind_group_layouts: &[&sprite_layout],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &sprite_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &sprite_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let sprite_bg_stars = device.create_bind_group(&BindGroupDescriptor {
            label: Some("sprite-stars"),
            layout: &sprite_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: stars_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: view_stars.as_entire_binding(),
                },
            ],
        });
        let sprite_bg_dust = device.create_bind_group(&BindGroupDescriptor {
            label: Some("sprite-dust"),
            layout: &sprite_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: stars_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: view_dust.as_entire_binding(),
                },
            ],
        });

        let bloom_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bloom-bgl"),
            entries: &[
                tex_entry(0, filter_linear),
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(if filter_linear {
                        SamplerBindingType::Filtering
                    } else {
                        SamplerBindingType::NonFiltering
                    }),
                    count: None,
                },
                tex_entry(2, filter_linear),
                tex_entry(3, filter_linear),
                uniform_entry(4, ShaderStages::FRAGMENT),
            ],
        });
        let bloom_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("bloom"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("bloom-pl"),
                bind_group_layouts: &[&bloom_layout],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &bloom_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &bloom_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let mag = if filter_linear {
            FilterMode::Linear
        } else {
            FilterMode::Nearest
        };
        let bloom_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("bloom-samp"),
            mag_filter: mag,
            min_filter: mag,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let mut app = Self {
            adapter_name,
            instance,
            device,
            queue,
            surface,
            surface_format,
            alpha_mode,
            size: (width, height),
            stars_buf,
            staging,
            sim_buf,
            view_stars,
            view_dust,
            n,
            frame: 1,
            seed: (launch.seed as u32).max(1),
            dust: launch.dust,
            cpu: launch.cpu,
            sim,
            orbit: Orbit::new(),
            compute_pipeline,
            compute_bg,
            sprite_pipeline,
            sprite_bg_stars,
            sprite_bg_dust,
            bloom_pipeline,
            targets: None,
            bloom_layout,
            bloom_sampler,
            composite_pipeline: None,
        };
        app.targets = Some(app.make_targets(width, height));
        let composite = app.make_composite_pipeline();
        app.composite_pipeline = Some(composite);
        app
    }

    fn make_targets(&self, width: u32, height: u32) -> Targets {
        let hdr = self.tex("hdr", width, height, TextureFormat::Rgba16Float);
        let bw = (width / 2).max(2);
        let bh = (height / 2).max(2);
        let bloom_a = self.tex("bloom-a", bw, bh, TextureFormat::Rgba16Float);
        let bloom_b = self.tex("bloom-b", bw, bh, TextureFormat::Rgba16Float);
        let dust = self.tex("dust", width, height, TextureFormat::Rgba16Float);
        let hdr_view = hdr.create_view(&TextureViewDescriptor::default());
        let view_a = bloom_a.create_view(&TextureViewDescriptor::default());
        let view_b = bloom_b.create_view(&TextureViewDescriptor::default());
        let dust_view = dust.create_view(&TextureViewDescriptor::default());

        let extract_buf = self.params_buf(BloomParams {
            texel: [1.0 / bw as f32, 1.0 / bh as f32],
            offset: 0.0,
            mode: 0.0,
            exposure: 0.85,
            bloom: 0.55,
            dust: if self.dust { 0.62 } else { 0.0 },
            _pad: 0.0,
        });
        let blur1 = self.params_buf(self.blur_params(bw, bh, 1.0));
        let blur2 = self.params_buf(self.blur_params(bw, bh, 2.5));
        let blur3 = self.params_buf(self.blur_params(bw, bh, 5.0));
        let composite = self.params_buf(BloomParams {
            texel: [1.0 / width as f32, 1.0 / height as f32],
            offset: 0.0,
            mode: 2.0,
            exposure: 0.85,
            bloom: 0.55,
            dust: if self.dust { 0.62 } else { 0.0 },
            _pad: 0.0,
        });

        // Unused slots must not be the pass's color target. Extract and blur
        // only sample `src`; composite is the pass that reads bloom and dust.
        let dummy = self.tex("bloom-dummy", 1, 1, TextureFormat::Rgba16Float);
        let dummy_view = dummy.create_view(&TextureViewDescriptor::default());
        let extract_bg = self.bloom_bg(&hdr_view, &dummy_view, &dummy_view, &extract_buf);
        let blur_ab = self.bloom_bg(&view_a, &dummy_view, &dummy_view, &blur1);
        let blur_ba = self.bloom_bg(&view_b, &dummy_view, &dummy_view, &blur2);
        let blur_ab2 = self.bloom_bg(&view_a, &dummy_view, &dummy_view, &blur3);
        let composite_bg = self.bloom_bg(&hdr_view, &view_b, &dust_view, &composite);

        Targets {
            hdr,
            hdr_view,
            bloom_a,
            view_a,
            bloom_b,
            view_b,
            dust,
            dust_view,
            dummy,
            extract_bg,
            blur_ab,
            blur_ba,
            blur_ab2,
            composite_bg,
        }
    }

    fn blur_params(&self, bw: u32, bh: u32, offset: f32) -> BloomParams {
        BloomParams {
            texel: [1.0 / bw as f32, 1.0 / bh as f32],
            offset,
            mode: 1.0,
            exposure: 0.85,
            bloom: 0.55,
            dust: if self.dust { 0.62 } else { 0.0 },
            _pad: 0.0,
        }
    }

    fn params_buf(&self, params: BloomParams) -> Buffer {
        self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("bloom-params"),
            contents: bytemuck::bytes_of(&params),
            usage: BufferUsages::UNIFORM,
        })
    }

    fn tex(&self, label: &str, w: u32, h: u32, format: TextureFormat) -> Texture {
        self.device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    fn bloom_bg(
        &self,
        src: &TextureView,
        bloom: &TextureView,
        dust: &TextureView,
        params: &Buffer,
    ) -> BindGroup {
        self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("bloom-bg"),
            layout: &self.bloom_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(src),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.bloom_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(bloom),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(dust),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: params.as_entire_binding(),
                },
            ],
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width < 2 || height < 2 {
            return;
        }
        self.size = (width, height);
        self.reconfigure_surface();
        self.targets = Some(self.make_targets(width, height));
    }

    fn reconfigure_surface(&self) {
        if let Some(surface) = &self.surface {
            surface.configure(
                &self.device,
                &SurfaceConfiguration {
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    format: self.surface_format,
                    width: self.size.0,
                    height: self.size.1,
                    present_mode: PresentMode::AutoVsync,
                    alpha_mode: self.alpha_mode,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                },
            );
        }
    }

    pub fn set_mond(&mut self, on: bool) {
        if self.sim.config.legacy {
            println!("MOND toggle ignored in --legacy mode");
            return;
        }
        self.sim.config.mond = on;
        println!(
            "MOND {}  a0={}",
            if on { "on" } else { "off" },
            self.sim.config.a0
        );
    }

    pub fn toggle_mond(&mut self) {
        let on = !self.sim.config.mond;
        self.set_mond(on);
    }

    pub fn orbit_drag(&mut self, dx: f32, dy: f32) {
        self.orbit.yaw_t += dx * 0.005;
        self.orbit.tilt_t = (self.orbit.tilt_t + dy * 0.004).clamp(0.08, 1.35);
    }

    pub fn orbit_zoom(&mut self, steps: f32) {
        self.orbit.dist_t = (self.orbit.dist_t * (1.0 - steps * 0.12)).clamp(240.0, 2600.0);
    }

    pub fn orbit_keys(&mut self, dt: f32, yaw: f32, tilt: f32, zoom: f32) {
        self.orbit.yaw_t += yaw * dt;
        self.orbit.tilt_t = (self.orbit.tilt_t + tilt * dt).clamp(0.08, 1.35);
        if zoom != 0.0 {
            self.orbit.dist_t = (self.orbit.dist_t * (1.0 - zoom * dt)).clamp(240.0, 2600.0);
        }
    }

    pub fn reset_orbit(&mut self) {
        self.orbit.reset();
    }

    pub fn pointer_down(&mut self, down: bool) {
        self.orbit.dragging = down;
    }

    pub fn dragging(&self) -> bool {
        self.orbit.dragging
    }

    /// One frame. Matches `GalaxySimulation::update`: dt clamped to 0.033, two substeps.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.min(0.033);
        self.orbit.smooth(dt);
        if self.cpu {
            self.sim.update(dt);
            self.upload_cpu();
        } else {
            let sub = dt * 0.5;
            self.dispatch(sub);
            self.dispatch(sub);
        }
    }

    fn upload_cpu(&self) {
        let packed = pack_stars(&self.sim.particles, self.dust);
        self.queue
            .write_buffer(&self.stars_buf, 0, bytemuck::cast_slice(&packed));
    }

    fn dispatch(&mut self, dt: f32) {
        self.sim.advance_perturber(dt);
        let bh = self.sim.black_hole_xyz();
        let flags = flags_of(&self.sim.config);
        let uni = SimUniforms {
            dt,
            pattern_angle: self.sim.pattern_angle(),
            a0: self.sim.config.a0,
            bhx: bh[0],
            bhy: bh[1],
            bhz: bh[2],
            flags,
            n: self.n,
            seed: self.seed,
            frame: self.frame,
            dust: u32::from(self.dust),
            _pad: 0,
        };
        self.frame = self.frame.wrapping_add(1);
        self.queue
            .write_buffer(&self.sim_buf, 0, bytemuck::bytes_of(&uni));
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("compute"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("stars"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.compute_pipeline);
            pass.set_bind_group(0, &self.compute_bg, &[]);
            pass.dispatch_workgroups(self.n.div_ceil(256), 1, 1);
        }
        self.queue.submit(Some(encoder.finish()));
    }

    pub fn render(&mut self, target: &TextureView, _target_format: TextureFormat) {
        self.write_views();
        let targets = self.targets.as_ref().expect("frame targets");
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("stars"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.hdr_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.sprite_pipeline);
            pass.set_bind_group(0, &self.sprite_bg_stars, &[]);
            pass.draw(0..6, 0..self.n);
        }
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("dust"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.dust_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.sprite_pipeline);
            pass.set_bind_group(0, &self.sprite_bg_dust, &[]);
            pass.draw(0..6, 0..self.n);
        }
        let bloom = &self.bloom_pipeline;
        draw_fullscreen(&mut encoder, bloom, &targets.view_a, &targets.extract_bg);
        draw_fullscreen(&mut encoder, bloom, &targets.view_b, &targets.blur_ab);
        draw_fullscreen(&mut encoder, bloom, &targets.view_a, &targets.blur_ba);
        draw_fullscreen(&mut encoder, bloom, &targets.view_b, &targets.blur_ab2);
        {
            let pipeline = self.composite_pipeline.as_ref().expect("composite");
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("composite"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &targets.composite_bg, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
    }

    fn write_views(&self) {
        let (vx, vy, vz) = self.orbit.eye();
        let mut cam = QuaternionCamera::new(vx, vy, vz);
        cam.look_at(0.0, 4.0, 0.0);
        let view = cam.view_matrix();
        let proj = FluxQuaternion::perspective_matrix(
            std::f32::consts::FRAC_PI_4,
            self.size.0 as f32 / self.size.1 as f32,
            2.0,
            8000.0,
        );
        let pattern = self.sim.pattern_angle();
        let n = self.n.max(1) as f32;
        let base = ViewUniforms {
            view,
            proj,
            cam_pos: [vx, vy, vz, 1.0],
            viewport: [self.size.0 as f32, self.size.1 as f32, pattern, 0.0],
            extra: [
                (90_000.0 / n).clamp(0.02, 1.6),
                (40_000.0 / n).clamp(0.008, 0.8),
                0.0,
                0.0,
            ],
        };
        let mut dust = base;
        dust.viewport[3] = 1.0;
        self.queue
            .write_buffer(&self.view_stars, 0, bytemuck::bytes_of(&base));
        self.queue
            .write_buffer(&self.view_dust, 0, bytemuck::bytes_of(&dust));
    }

    pub fn render_surface(&mut self) {
        let Some(surface) = &self.surface else {
            return;
        };
        let frame = match surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.reconfigure_surface();
                return;
            }
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        self.render(&view, self.surface_format);
        frame.present();
    }

    pub fn save_png(&mut self, path: &str) -> Result<(), String> {
        let (w, h) = self.size;
        let tex = self.device.create_texture(&TextureDescriptor {
            label: Some("shot"),
            size: Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.surface_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&TextureViewDescriptor::default());
        self.render(&view, self.surface_format);
        let padded = {
            let unpadded = w * 4;
            let align = COPY_BYTES_PER_ROW_ALIGNMENT;
            unpadded.div_ceil(align) * align
        };
        let read = self.device.create_buffer(&BufferDescriptor {
            label: Some("png-read"),
            size: (padded * h) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("copy-png"),
            });
        encoder.copy_texture_to_buffer(
            tex.as_image_copy(),
            ImageCopyBuffer {
                buffer: &read,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let slice = read.slice(..);
        slice.map_async(MapMode::Read, |_| {});
        let _ = self.device.poll(Maintain::Wait);
        let mapped = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for row in 0..h {
            let start = (row * padded) as usize;
            rgba.extend_from_slice(&mapped[start..start + (w * 4) as usize]);
        }
        drop(mapped);
        read.unmap();
        if let Some(parent) = std::path::Path::new(path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(file, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&rgba)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn benchmark(&mut self, frames: u32) -> Bench {
        let frames = frames.max(1);
        let t0 = Instant::now();
        for _ in 0..frames {
            self.step(1.0 / 60.0);
        }
        let _ = self.device.poll(Maintain::Wait);
        let compute_s = t0.elapsed().as_secs_f32();
        println!(
            "compute {} frames in {:.2}s ({:.1} fps)",
            frames,
            compute_s,
            frames as f32 / compute_s
        );
        let t1 = Instant::now();
        let draw_frames = frames.min(12);
        for i in 0..draw_frames {
            self.step(1.0 / 60.0);
            self.render_offscreen_once();
            let _ = self.device.poll(Maintain::Wait);
            println!(
                "frame {}/{} {:.2}s",
                i + 1,
                draw_frames,
                t1.elapsed().as_secs_f32()
            );
        }
        let frame_s = t1.elapsed().as_secs_f32();
        Bench {
            adapter: self.adapter_name.clone(),
            stars: self.n,
            compute_ms: compute_s * 1000.0 / frames as f32,
            compute_fps: frames as f32 / compute_s,
            frame_ms: frame_s * 1000.0 / draw_frames as f32,
            frame_fps: draw_frames as f32 / frame_s,
        }
    }

    fn render_offscreen_once(&mut self) {
        let (w, h) = self.size;
        let tex = self.device.create_texture(&TextureDescriptor {
            label: Some("bench-target"),
            size: Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.surface_format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&TextureViewDescriptor::default());
        self.render(&view, self.surface_format);
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn read_curve(&mut self) -> CurveCheck {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("readback"),
            });
        encoder.copy_buffer_to_buffer(&self.stars_buf, 0, &self.staging, 0, self.stars_buf.size());
        self.queue.submit(Some(encoder.finish()));
        let slice = self.staging.slice(..);
        slice.map_async(MapMode::Read, |_| {});
        let _ = self.device.poll(Maintain::Wait);
        let mapped = slice.get_mapped_range();
        let stars: &[StarGpu] = bytemuck::cast_slice(&mapped);
        let samples = stars.iter().map(|s| {
            (
                [s.pos[0], s.pos[1], s.pos[2]],
                [s.vel[0], s.vel[1], s.vel[2]],
            )
        });
        let check = curve_check(samples);
        drop(mapped);
        self.staging.unmap();
        check
    }

    fn make_composite_pipeline(&self) -> RenderPipeline {
        self.device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("composite"),
                layout: Some(
                    &self
                        .device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("composite-pl"),
                            bind_group_layouts: &[&self.bloom_layout],
                            push_constant_ranges: &[],
                        }),
                ),
                vertex: VertexState {
                    module: &self.composite_module(),
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(FragmentState {
                    module: &self.composite_module(),
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: self.surface_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            })
    }

    fn composite_module(&self) -> ShaderModule {
        self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("galaxy-bloom-composite"),
            source: ShaderSource::Wgsl(include_str!("../assets/shaders/galaxy_bloom.wgsl").into()),
        })
    }
}

fn draw_fullscreen(
    encoder: &mut CommandEncoder,
    pipeline: &RenderPipeline,
    view: &TextureView,
    bg: &BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("bloom"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::BLACK),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bg, &[]);
    pass.draw(0..3, 0..1);
}

fn storage_entry(binding: u32, writable: bool, visibility: ShaderStages) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage {
                read_only: !writable,
            },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32, visibility: ShaderStages) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn tex_entry(binding: u32, filterable: bool) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::FRAGMENT,
        ty: BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn flags_of(config: &GalaxyConfig) -> u32 {
    let mut f = 0;
    if config.mond {
        f |= FLAG_MOND;
    }
    if config.vortex {
        f |= FLAG_VORTEX;
    }
    if config.legacy {
        f |= FLAG_LEGACY;
    }
    f
}

fn pack_stars(particles: &[Particle], dust: bool) -> Vec<StarGpu> {
    particles
        .iter()
        .enumerate()
        .map(|(i, p)| StarGpu {
            pos: [p.x, p.y, p.z, if dust && i % 8 == 0 { 1.0 } else { 0.0 }],
            vel: [p.flux.x, p.flux.y, p.flux.z, p.flux.w],
        })
        .collect()
}

pub fn run_offline(launch: &Launch) {
    let mut app = GalaxyApp::offscreen(launch);
    if launch.benchmark_frames > 0 {
        let bench = app.benchmark(launch.benchmark_frames);
        println!(
            "benchmark adapter={} stars={} compute {:.2} fps ({:.2} ms)  full frame {:.2} fps ({:.2} ms)",
            bench.adapter,
            bench.stars,
            bench.compute_fps,
            bench.compute_ms,
            bench.frame_fps,
            bench.frame_ms
        );
    }
    if let Some(path) = &launch.shot {
        app.save_png(path).expect("write screenshot");
        println!("wrote {path}");
    }
}

/// Step the GPU kernel and the CPU integrator from the same initial disk.
#[cfg_attr(not(test), allow(dead_code))]
pub fn parity_against_cpu(
    n: usize,
    seconds: f32,
    config: GalaxyConfig,
    seed: u64,
) -> (CurveCheck, CurveCheck) {
    let launch = Launch {
        stars: n,
        seed,
        cpu: false,
        dust: false,
        config,
        shot: None,
        benchmark_frames: 0,
        width: 64,
        height: 64,
    };
    let mut gpu = GalaxyApp::offscreen(&launch);
    let mut cpu = GalaxySimulation::with_config(n, config, Some(seed));
    cpu.verbose = false;
    let steps = (seconds * 60.0).round() as u32;
    let dt = 1.0 / 60.0;
    for _ in 0..steps {
        cpu.update(dt);
        gpu.step(dt);
    }
    let cpu_check = curve_check(
        cpu.particles
            .iter()
            .map(|p| ([p.x, p.y, p.z], [p.flux.x, p.flux.y, p.flux.z])),
    );
    let gpu_check = gpu.read_curve();
    (cpu_check, gpu_check)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(name: &str, gpu: f32, cpu: f32, rel: f32) {
        let denom = cpu.abs().max(1e-3);
        let d = (gpu - cpu).abs() / denom;
        assert!(
            d < rel,
            "{name}: gpu {gpu:.4} cpu {cpu:.4} relative {d:.3} (limit {rel})"
        );
    }

    #[test]
    fn gpu_kernel_matches_cpu_curve_and_thickness() {
        for (label, config) in [
            ("mond", GalaxyConfig::default()),
            (
                "newton",
                GalaxyConfig {
                    mond: false,
                    ..GalaxyConfig::default()
                },
            ),
        ] {
            let (cpu, gpu) = parity_against_cpu(4096, 3.0, config, 11);
            eprintln!(
                "{label}: cpu ratio {:.4} v52 {:.4} v195 {:.4} rms {:.4} n {}/{} | gpu ratio {:.4} v52 {:.4} v195 {:.4} rms {:.4} n {}/{}",
                cpu.ratio,
                cpu.v_52,
                cpu.v_195,
                cpu.rms_y,
                cpu.n_52,
                cpu.n_195,
                gpu.ratio,
                gpu.v_52,
                gpu.v_195,
                gpu.rms_y,
                gpu.n_52,
                gpu.n_195
            );
            assert!(
                cpu.v_52.is_finite() && cpu.v_52 > 1.0,
                "{label} cpu v52 {}",
                cpu.v_52
            );
            assert!(cpu.n_52 > 20, "{label} cpu inner bin {}", cpu.n_52);
            assert!(cpu.n_195 > 40, "{label} cpu outer bin {}", cpu.n_195);
            assert!(gpu.n_52 > 20 && gpu.n_195 > 40, "{label} gpu bins empty");
            assert_close(&format!("{label} ratio"), gpu.ratio, cpu.ratio, 0.06);
            assert_close(&format!("{label} v195"), gpu.v_195, cpu.v_195, 0.06);
            assert_close(&format!("{label} rms y"), gpu.rms_y, cpu.rms_y, 0.08);
        }
    }
}
