use egui::{ClippedPrimitive, Context, TexturesDelta};
use egui_wgpu::renderer::{RenderPass, ScreenDescriptor};
use pixels::{wgpu, PixelsContext};
use rand::Rng;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

use crate::world::{Cell, Empire, World};

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
    pub(crate) fn prepare(&mut self, window: &Window, world: &mut World) {
        // Run the egui frame and create all paint jobs to prepare for rendering.
        let raw_input = self.egui_state.take_egui_input(window);
        let output = self.egui_ctx.run(raw_input, |egui_ctx| {
            // Draw the demo application.
            self.gui.ui(egui_ctx, world);
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
    /// Only show the egui window when true.
    window_open: bool,
    pub painting_with: u16,
    pub painting_troops: u8,
    pub playing: bool,
}
impl Gui {
    /// Create a `Gui`.
    fn new() -> Self {
        Self {
            window_open: true,
            painting_with: 0,
            painting_troops: 1,
            playing: true,
        }
    }

    /// Create the UI using egui.
    fn ui(&mut self, ctx: &Context, world: &mut World) {
        egui::TopBottomPanel::top("menubar_container").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("About...").clicked() {
                        self.window_open = true;
                        ui.close_menu();
                    }
                })
            });
        });

        egui::Window::new("window")
            .open(&mut self.window_open)
            .show(ctx, |ui| {
                if ui
                    .button("reset to 1 per empire (start with 255 troops)")
                    .clicked()
                {
                    world.cells = vec![Cell::default(); world.width * world.height];
                    for empire in world.empires.clone() {
                        for _ in 0..10 {
                            world.set(
                                rand::thread_rng().gen_range(0..world.width) as isize,
                                rand::thread_rng().gen_range(0..world.height) as isize,
                                Cell {
                                    owner: empire.id,
                                    troops: 255,
                                },
                            );
                        }
                    }
                }
                if self.playing {
                    if ui.button("pause").clicked() {
                        self.playing = false;
                    }
                } else if ui.button("play").clicked() {
                    self.playing = true;
                }
                ui.label(format!("Painting with empire {}", self.painting_with));
                egui::ComboBox::from_label("Painting with empire")
                    .selected_text(format!("{:?}", self.painting_with))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.painting_with, 0, "blank");
                        for empire in &world.empires {
                            ui.selectable_value(
                                &mut self.painting_with,
                                empire.id,
                                format!("{}", empire.id),
                            );
                        }
                    });

                if self.painting_with != 0 {
                    ui.add(
                        egui::Slider::new(&mut self.painting_troops, 1..=255)
                            .clamp_to_range(true)
                            .text("Troops to paint"),
                    );
                } else {
                    self.painting_troops = 0;
                }

                ui.heading("World Settings");
                ui.add(
                    egui::Slider::new(&mut world.max_troops, 1..=255)
                        .clamp_to_range(true)
                        .text("Max troops/cell"),
                );
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for empire in &mut world.empires {
                            ui.heading(format!("Empire {}", empire.id));
                            let mut color = [
                                empire.color.0 as f32 / 255.,
                                empire.color.1 as f32 / 255.,
                                empire.color.2 as f32 / 255.,
                                empire.color.3 as f32 / 255.,
                            ];
                            ui.color_edit_button_rgba_premultiplied(&mut color);
                            empire.color = (
                                (color[0] * 255.) as u8,
                                (color[1] * 255.) as u8,
                                (color[2] * 255.) as u8,
                                (color[3] * 255.) as u8,
                            );
                            ui.label(format!(
                                "{} cells",
                                world.cells.iter().filter(|v| v.owner == empire.id).count()
                            ));
                            ui.label(format!(
                                "{} troops",
                                world
                                    .cells
                                    .iter()
                                    .filter_map(|v| if v.owner == empire.id {
                                        Some(v.troops as usize)
                                    } else {
                                        None
                                    })
                                    .sum::<usize>()
                            ));
                        }
                    });

                if ui.button("create new empire").clicked() {
                    world.empires.push(Empire {
                        id: (world.empires.len() + 1) as u16,
                        color: (rand::random(), rand::random(), rand::random(), 255),
                    });
                }
            });
    }
}
