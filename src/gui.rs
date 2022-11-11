use egui::{ClippedPrimitive, Context, TexturesDelta};
use egui_wgpu::renderer::{RenderPass, ScreenDescriptor};
use itertools::Itertools;
use pixels::{wgpu, Pixels, PixelsContext};
use rand::Rng;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

use libterritory::world::{Cell, Empire, World};

/// Manages all state required for rendering egui over `Pixels`.
pub(crate) struct Framework {
    // State for egui.
    egui_ctx: Context,
    egui_state: egui_winit::State,
    screen_descriptor: ScreenDescriptor,
    rpass: RenderPass,
    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,

    // State for the GUI
    pub gui: Gui,
}

impl Framework {
    /// Create egui.
    pub(crate) fn new<T>(
        event_loop: &EventLoopWindowTarget<T>,
        width: u32,
        height: u32,
        scale_factor: f32,
        pixels: &pixels::Pixels,
    ) -> Self {
        let max_texture_size = pixels.device().limits().max_texture_dimension_2d as usize;

        let egui_ctx = Context::default();
        let mut egui_state = egui_winit::State::new(event_loop);
        egui_state.set_max_texture_side(max_texture_size);
        egui_state.set_pixels_per_point(scale_factor);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: scale_factor,
        };
        let rpass = RenderPass::new(pixels.device(), pixels.render_texture_format(), 1);
        let textures = TexturesDelta::default();
        let gui = Gui::new();

        Self {
            egui_ctx,
            egui_state,
            screen_descriptor,
            rpass,
            paint_jobs: Vec::new(),
            textures,
            gui,
        }
    }

    /// Handle input events from the window manager.
    pub(crate) fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        self.egui_state.on_event(&self.egui_ctx, event);
    }

    /// Resize egui.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.screen_descriptor.size_in_pixels = [width, height];
        }
    }

    /// Update scaling factor.
    pub(crate) fn scale_factor(&mut self, scale_factor: f64) {
        self.screen_descriptor.pixels_per_point = scale_factor as f32;
    }

    /// Prepare egui.
    pub(crate) fn prepare(&mut self, window: &Window, world: &mut World, pixels: &mut Pixels) {
        // Run the egui frame and create all paint jobs to prepare for rendering.
        let raw_input = self.egui_state.take_egui_input(window);
        let output = self.egui_ctx.run(raw_input, |egui_ctx| {
            // Draw the demo application.
            self.gui.ui(egui_ctx, world, pixels);
        });

        self.textures.append(output.textures_delta);
        self.egui_state
            .handle_platform_output(window, &self.egui_ctx, output.platform_output);
        self.paint_jobs = self.egui_ctx.tessellate(output.shapes);
    }

    /// Render egui.
    pub(crate) fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &PixelsContext,
    ) {
        // Upload all resources to the GPU.
        for (id, image_delta) in &self.textures.set {
            self.rpass
                .update_texture(&context.device, &context.queue, *id, image_delta);
        }
        self.rpass.update_buffers(
            &context.device,
            &context.queue,
            &self.paint_jobs,
            &self.screen_descriptor,
        );

        // Record all render passes.
        self.rpass.execute(
            encoder,
            render_target,
            &self.paint_jobs,
            &self.screen_descriptor,
            None,
        );

        // Cleanup
        let textures = std::mem::take(&mut self.textures);
        for id in &textures.free {
            self.rpass.free_texture(id);
        }
    }
}

/// Example application state. A real application will need a lot more state than this.
pub struct Gui {
    pub playing: bool,
    new_width: u32,
    new_height: u32,
}
impl Gui {
    /// Create a `Gui`.
    fn new() -> Self {
        Self {
            playing: true,
            new_width: 256,
            new_height: 256,
        }
    }

    /// Create the UI using egui.
    fn ui(&mut self, ctx: &Context, world: &mut World, pixels: &mut Pixels) {
        egui::Window::new("About").show(ctx, |ui| {
            ui.heading("Usage");
			ui.label("To get started, press 'Add empire' in the world settings window a few times, then hit 'Randomize' and watch!");
        });

        egui::Window::new("World Settings").show(ctx, |ui| {
            ui.label("Settings to play with about the simulation.");

            ui.add(egui::Slider::new(&mut self.new_width, 0..=1024).text("width"));
            ui.add(egui::Slider::new(&mut self.new_height, 0..=1024).text("height"));

            if ui.button("Resize").clicked() {
                world.resize(self.new_width as usize, self.new_height as usize);
                pixels.resize_buffer(self.new_width, self.new_height);
            }

            if ui.button("Add empire").clicked() {
                world.empires.push(Empire {
                    id: (world.empires.len() + 1) as u16,
                    color: (rand::random(), rand::random(), rand::random(), 255),
                });
            }
        });

        egui::Window::new("World Info").show(ctx, |ui| {
            if ui.button("Randomize").clicked() {
                world.cells = vec![Cell::default(); world.width * world.height];
                for empire in world.empires.clone() {
                    world.set(
                        rand::thread_rng().gen_range(0..world.width) as isize,
                        rand::thread_rng().gen_range(0..world.height) as isize,
                        Cell {
                            owner: empire.id,
                            troops: rand::random(),
                        },
                    );
                }
            }
            if self.playing {
                if ui.button("Pause").clicked() {
                    self.playing = false;
                }
            } else if ui.button("Play").clicked() {
                self.playing = true;
            }

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let mut colors = vec![(0, 0, 0, 0); world.empires.len()];
                    let empires_sorted = world
                        .empires
                        .iter()
                        .map(|empire| {
                            let e = &empire;
                            (
                                empire,
                                world.cells.iter().filter(|v| v.owner == e.id).count(),
                                world
                                    .cells
                                    .iter()
                                    .filter_map(|v| {
                                        if v.owner == e.id {
                                            Some(v.troops as usize)
                                        } else {
                                            None
                                        }
                                    })
                                    .sum::<usize>(),
                            )
                        })
                        .sorted_by_key(|v| v.2)
                        .rev()
                        .collect::<Vec<_>>();
                    for (i, (empire, cells, troops)) in empires_sorted.iter().enumerate() {
                        ui.heading(format!("Empire {}", empire.id));

                        ui.label(format!(
                            "{}",
                            world
                                .empires
                                .iter()
                                .filter(|v| {
                                    v.id != empire.id && empires_sorted[v.id as usize - 1].1 == 0
                                })
                                .count()
                        ));
                        if world
                            .empires
                            .iter()
                            .filter(|v| {
                                v.id != empire.id && empires_sorted[v.id as usize - 1].1 == 0
                            })
                            .count()
                            == world.empires.len() - 1
                        {
                            ui.label("Winner winner chicken dinner");
                        }
                        ui.label(format!("This empire is #{} in troops", i + 1));
                        let mut color = [
                            empire.color.0 as f32 / 255.,
                            empire.color.1 as f32 / 255.,
                            empire.color.2 as f32 / 255.,
                            empire.color.3 as f32 / 255.,
                        ];
                        ui.color_edit_button_rgba_premultiplied(&mut color);

                        colors[(empire.id - 1) as usize] = (
                            (color[0] * 255.) as u8,
                            (color[1] * 255.) as u8,
                            (color[2] * 255.) as u8,
                            (color[3] * 255.) as u8,
                        );
                        ui.label(format!("{} cells", cells));
                        ui.label(if *troops > 1_000_000_000 {
                            format!("{} billion troops", troops / 1_000_000_000)
                        } else if *troops > 1_000_000 {
                            format!("{} million troops", troops / 1_000_000)
                        } else {
                            format!("{} troops", troops)
                        });
                    }
                    for (i, color) in colors.iter().enumerate() {
                        world.empires[i].color = *color;
                    }
                });
        });
    }
}
