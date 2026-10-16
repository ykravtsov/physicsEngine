mod engine;
mod galaxy;
mod gpu_galaxy;
mod headless;
mod hurricane_3d;
mod quaternion;

use engine::input::InputState;
use gpu_galaxy::{GalaxyApp, Launch};

use winit::{
    event::{DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
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
    let launch = Launch::from_args(&args);
    if launch.shot.is_some() || launch.benchmark_frames > 0 {
        gpu_galaxy::run_offline(&launch);
        return;
    }
    pollster::block_on(run(launch));
}

async fn run(launch: Launch) {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = {
        use winit::window::Window;
        let attrs = Window::default_attributes()
            .with_title("QDM-NS Galaxy — GPU")
            .with_inner_size(winit::dpi::LogicalSize::new(launch.width, launch.height));
        #[allow(deprecated)]
        event_loop
            .create_window(attrs)
            .expect("Failed to create window")
    };

    // SAFETY: window lives for the duration of the program
    let window: &'static winit::window::Window = Box::leak(Box::new(window));

    println!(
        "QDM-NS galaxy  {}  {} stars  {}  (orbit drag, scroll zoom, M toggles MOND, Esc quits)",
        launch.config.label(),
        launch.stars,
        if launch.cpu {
            "CPU physics"
        } else {
            "GPU physics"
        }
    );
    let mut app = GalaxyApp::windowed(window, &launch);
    let mut input = InputState::new();

    let mut last_time = std::time::Instant::now();

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
                                        app.toggle_mond();
                                    }
                                    if key == KeyCode::KeyR {
                                        app.reset_orbit();
                                    }
                                }
                                ElementState::Released => {
                                    input.keys_held.remove(&key);
                                }
                            }
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if button == MouseButton::Left {
                            app.pointer_down(state == ElementState::Pressed);
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let steps = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(p) => (p.y / 80.0) as f32,
                        };
                        app.orbit_zoom(steps);
                    }
                    WindowEvent::Resized(size) => {
                        app.resize(size.width, size.height);
                    }
                    WindowEvent::RedrawRequested => {
                        let now = std::time::Instant::now();
                        let dt = now.duration_since(last_time).as_secs_f32().min(0.05);
                        last_time = now;

                        let mut yaw = 0.0;
                        let mut tilt = 0.0;
                        let mut zoom = 0.0;
                        if input.is_held(KeyCode::KeyA) {
                            yaw -= 0.7;
                        }
                        if input.is_held(KeyCode::KeyD) {
                            yaw += 0.7;
                        }
                        if input.is_held(KeyCode::KeyQ) {
                            tilt -= 0.45;
                        }
                        if input.is_held(KeyCode::KeyE) {
                            tilt += 0.45;
                        }
                        if input.is_held(KeyCode::KeyW) {
                            zoom += 0.65;
                        }
                        if input.is_held(KeyCode::KeyS) {
                            zoom -= 0.65;
                        }
                        app.orbit_keys(dt, yaw, tilt, zoom);
                        app.step(dt);
                        app.render_surface();
                        input.reset_frame();
                        window.request_redraw();
                    }
                    _ => {}
                },
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } if app.dragging() => {
                    app.orbit_drag(delta.0 as f32, delta.1 as f32);
                }
                _ => {}
            }
        })
        .expect("Event loop error");
}
