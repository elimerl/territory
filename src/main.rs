mod gui;

use std::rc::Rc;
use std::time::Instant;

use gui::Framework;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};

use libterritory::world::{Cell, World};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new((WIDTH * 2) as f64, (HEIGHT * 2) as f64);
        WindowBuilder::new()
            .with_title("territory")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let (mut pixels, mut framework) = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).expect("Pixels error");

        let framework = Framework::new(
            &event_loop,
            window_size.width,
            window_size.height,
            window.scale_factor() as f32,
            &pixels,
        );

        (pixels, framework)
    };

    let mut world = World::new(WIDTH as usize, HEIGHT as usize);
    window.focus_window();
    let mut last_tick = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { ref event, .. } => {
                // Update egui inputs
                framework.handle_event(event);
            }

            Event::RedrawRequested(_) => {
                world.draw(pixels.get_frame_mut());
                // Prepare egui
                framework.prepare(&window, &mut world, &mut pixels);

                // Render everything together
                let render_result = pixels.render_with(|encoder, render_target, context| {
                    // Render the world texture
                    context.scaling_renderer.render(encoder, render_target);

                    // Render egui
                    framework.render(encoder, render_target, context);

                    Ok(())
                });

                // Basic error handling
                if render_result
                    .map_err(|e| error!("pixels.render() failed: {}", e))
                    .is_err()
                {
                    *control_flow = ControlFlow::Exit;
                }
            }

            _ => (),
        }

        // Handle input events
        if input.update(&event) {
            // Update the scale factor
            if let Some(scale_factor) = input.scale_factor() {
                framework.scale_factor(scale_factor);
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
                framework.resize(size.width, size.height);
            }

            // Update internal state and request a redraw
            if last_tick.elapsed().as_millis() >= 10 && framework.gui.playing {
                last_tick = Instant::now();

                world.update();
            }

            window.request_redraw();
        }
    });
}
