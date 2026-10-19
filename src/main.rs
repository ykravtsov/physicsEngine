mod engine;
mod galaxy;
mod gpu_galaxy;
mod headless;
mod quaternion;

use engine::input::InputState;
use gpu_galaxy::{GalaxyApp, Launch};

use winit::{
    event::{DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
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

fn capture_mouse(window: &Window, capture: bool) -> bool {
    if !capture {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
        return false;
    }
    match window
        .set_cursor_grab(CursorGrabMode::Locked)
        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
    {
        Ok(()) => {
            window.set_cursor_visible(false);
            true
        }
        Err(error) => {
            window.set_cursor_visible(true);
            eprintln!("Mouse capture unavailable ({error}); hold left mouse to look around.");
            false
        }
    }
}

async fn run(launch: Launch) {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = {
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
        "QDM-NS galaxy  {}  {} stars  {}  (F toggles flight, orbit drag, scroll zoom, M toggles MOND, Esc quits)",
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
    let mut focused = window.has_focus();
    let mut mouse_captured = false;
    let mut last_time = std::time::Instant::now();

    #[allow(deprecated)]
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Focused(focus) => {
                        focused = focus;
                        input.clear();
                        app.pointer_down(false);
                        mouse_captured = capture_mouse(window, focused && app.flying());
                        last_time = std::time::Instant::now();
                    }
                    WindowEvent::KeyboardInput {
                        event,
                        is_synthetic,
                        ..
                    } if focused => {
                        if let PhysicalKey::Code(key) = event.physical_key {
                            match event.state {
                                ElementState::Pressed => {
                                    let first_press = input.press(key);
                                    if first_press && !event.repeat && !is_synthetic {
                                        match key {
                                            KeyCode::Escape => elwt.exit(),
                                            KeyCode::KeyM => app.toggle_mond(),
                                            KeyCode::KeyR => app.reset_camera(),
                                            KeyCode::KeyF => {
                                                let flying = app.toggle_flight();
                                                mouse_captured = capture_mouse(window, flying);
                                                println!(
                                                    "Camera: {}",
                                                    if flying {
                                                        "flight — WASD move, Space up, Shift down, mouse look, F returns to orbit"
                                                    } else {
                                                        "orbit — drag/AD yaw, QE tilt, scroll/WS zoom, F enables flight"
                                                    }
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                ElementState::Released => input.release(key),
                            }
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } if focused => {
                        if button == MouseButton::Left {
                            let down = state == ElementState::Pressed;
                            app.pointer_down(down);
                            if down && app.flying() && !mouse_captured {
                                mouse_captured = capture_mouse(window, true);
                            }
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } if focused => {
                        let steps = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(p) => (p.y / 80.0) as f32,
                        };
                        app.camera_zoom(steps);
                    }
                    WindowEvent::Resized(size) => {
                        app.resize(size.width, size.height);
                    }
                    WindowEvent::RedrawRequested => {
                        let now = std::time::Instant::now();
                        let dt = now.duration_since(last_time).as_secs_f32();
                        last_time = now;

                        let (right, up, forward) = input.movement(app.flying());
                        app.navigate(dt, right, up, forward);
                        app.step(dt);
                        app.render_surface();
                        window.request_redraw();
                    }
                    _ => {}
                },
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } if focused && (mouse_captured || app.dragging()) => {
                    app.camera_drag(delta.0 as f32, delta.1 as f32);
                }
                _ => {}
            }
        })
        .expect("Event loop error");
}
