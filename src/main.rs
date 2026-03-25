mod engine;
mod quaternion;
mod hurricane_3d;

use engine::gpu::GpuState;
use engine::input::InputState;
use quaternion::camera::QuaternionCamera;
use hurricane_3d::HurricaneSimulation;

use winit::{
    event::{Event, WindowEvent, DeviceEvent, ElementState},
    event_loop::{EventLoop, ControlFlow},
    keyboard::{KeyCode, PhysicalKey},
};

fn main() {
    env_logger::init();
    pollster::block_on(run());
}

async fn run() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = {
        use winit::window::Window;
        let attrs = Window::default_attributes()
            .with_title("Quaternion Vortex Engine — Hurricane")
            .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32));
        #[allow(deprecated)]
        event_loop.create_window(attrs).expect("Failed to create window")
    };

    // SAFETY: window lives for the duration of the program
    let window: &'static winit::window::Window = Box::leak(Box::new(window));

    let mut gpu = GpuState::new(window).await;
    let mut input = InputState::new();
    let mut camera = QuaternionCamera::new(0.0, 50.0, 300.0);
    camera.move_speed = 50.0;

    let mut hurricane = HurricaneSimulation::new(50000);
    let renderer = hurricane_3d::renderer::ParticleRenderer::new(&gpu.device, &gpu.config);

    let mut last_time = std::time::Instant::now();

    // Capture mouse for fly camera
    window.set_cursor_visible(false);
    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);

    #[allow(deprecated)]
    event_loop.run(move |event, elwt| {
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
                    if input.is_held(KeyCode::KeyW) { camera.move_forward(speed); }
                    if input.is_held(KeyCode::KeyS) { camera.move_forward(-speed); }
                    if input.is_held(KeyCode::KeyA) { camera.move_right(-speed); }
                    if input.is_held(KeyCode::KeyD) { camera.move_right(speed); }
                    if input.is_held(KeyCode::Space) { camera.move_up(speed); }
                    if input.is_held(KeyCode::ShiftLeft) { camera.move_up(-speed); }

                    // --- PHYSICS ---
                    hurricane.update(dt);

                    // --- RENDER ---
                    let view = camera.view_matrix();
                    let (w, h) = gpu.size;
                    let proj = quaternion::math::FluxQuaternion::perspective_matrix(
                        std::f32::consts::FRAC_PI_4,
                        w as f32 / h as f32,
                        0.1,
                        5000.0,
                    );

                    renderer.render(&gpu, &hurricane.particles, view, proj);

                    input.reset_frame();
                    window.request_redraw();
                }
                _ => {}
            },
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                camera.rotate(delta.0 as f32, delta.1 as f32);
                input.mouse_delta = (delta.0 as f32, delta.1 as f32);
            }
            _ => {}
        }
    }).expect("Event loop error");
}
