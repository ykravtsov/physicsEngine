mod engine;
mod galaxy;
mod headless;
mod hurricane_3d;
mod quaternion;

use engine::gpu::GpuState;
use engine::input::InputState;
use galaxy::{GalaxyConfig, GalaxySimulation};
use quaternion::camera::QuaternionCamera;

use winit::{
    event::{DeviceEvent, ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
};

fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--headless") {
        headless::run(&args);
        return;
    }
    pollster::block_on(run(args));
}

async fn run(args: Vec<String>) {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = {
        use winit::window::Window;
        let attrs = Window::default_attributes()
            .with_title("QDM-NS Galaxy — Black Hole Through Space Fluid")
            .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32));
        #[allow(deprecated)]
        event_loop
            .create_window(attrs)
            .expect("Failed to create window")
    };

    // SAFETY: window lives for the duration of the program
    let window: &'static winit::window::Window = Box::leak(Box::new(window));

    let mut gpu = GpuState::new(window).await;
    let mut input = InputState::new();

    // Oblique view: far enough to see the arms, high enough to see the eyewall.
    // look_at aims the quaternion at the disk. The old rotate(0, 18) was a
    // mouse-delta (×0.002 rad), so it pitched about 2°, not 18°.
    let mut camera = QuaternionCamera::new(200.0, 480.0, 300.0);
    camera.move_speed = 120.0;
    camera.look_at(0.0, 8.0, 0.0);

    let config = GalaxyConfig::from_args(&args);
    println!(
        "QDM-NS galaxy  {}  (M toggles MOND, Esc quits)",
        config.label()
    );
    let mut galaxy = GalaxySimulation::with_config(60_000, config, None);
    let renderer = hurricane_3d::renderer::ParticleRenderer::new(&gpu.device, &gpu.config);

    let mut last_time = std::time::Instant::now();

    // Capture mouse for fly camera
    window.set_cursor_visible(false);
    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);

    #[allow(deprecated)]
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let PhysicalKey::Code(key) = event.physical_key {
                            match event.state {
                                ElementState::Pressed => {
                                    input.keys_held.insert(key);
                                    if key == KeyCode::Escape {
                                        elwt.exit();
                                    }
                                    if key == KeyCode::KeyM {
                                        if galaxy.config.legacy {
                                            println!("MOND toggle ignored in --legacy mode");
                                        } else {
                                            galaxy.config.mond = !galaxy.config.mond;
                                            println!(
                                                "MOND {}  a0={}",
                                                if galaxy.config.mond { "on" } else { "off" },
                                                galaxy.config.a0
                                            );
                                        }
                                    }
                                }
                                ElementState::Released => {
                                    input.keys_held.remove(&key);
                                }
                            }
                        }
                    }
                    WindowEvent::Resized(size) => {
                        gpu.resize(size.width, size.height);
                    }
                    WindowEvent::RedrawRequested => {
                        let now = std::time::Instant::now();
                        let dt = now.duration_since(last_time).as_secs_f32();
                        last_time = now;

                        // --- INPUT ---
                        let speed = camera.move_speed * dt;
                        if input.is_held(KeyCode::KeyW) {
                            camera.move_forward(speed);
                        }
                        if input.is_held(KeyCode::KeyS) {
                            camera.move_forward(-speed);
                        }
                        if input.is_held(KeyCode::KeyA) {
                            camera.move_right(-speed);
                        }
                        if input.is_held(KeyCode::KeyD) {
                            camera.move_right(speed);
                        }
                        if input.is_held(KeyCode::Space) {
                            camera.move_up(speed);
                        }
                        if input.is_held(KeyCode::ShiftLeft) {
                            camera.move_up(-speed);
                        }

                        // --- PHYSICS ---
                        galaxy.update(dt);

                        // --- RENDER ---
                        let view = camera.view_matrix();
                        let (w, h) = gpu.size;
                        let proj = quaternion::math::FluxQuaternion::perspective_matrix(
                            std::f32::consts::FRAC_PI_4,
                            w as f32 / h as f32,
                            0.1,
                            5000.0,
                        );

                        renderer.render(&gpu, &galaxy.particles, view, proj);

                        input.reset_frame();
                        window.request_redraw();
                    }
                    _ => {}
                },
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } => {
                    camera.rotate(delta.0 as f32, delta.1 as f32);
                    input.mouse_delta = (delta.0 as f32, delta.1 as f32);
                }
                _ => {}
            }
        })
        .expect("Event loop error");
}
