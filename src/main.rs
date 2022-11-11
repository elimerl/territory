use std::time::Instant;

use itertools::Itertools;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use rand::seq::IteratorRandom;
use rand::Rng;
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new((WIDTH * 2) as f64, (HEIGHT * 2) as f64);
        WindowBuilder::new()
            .with_title("territory")
            .with_inner_size(size)
            .with_resizable(false)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
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
            Event::RedrawRequested(_) => {
                world.draw(pixels.get_frame_mut());
                if pixels
                    .render()
                    .map_err(|e| error!("pixels.render() failed: {}", e))
                    .is_err()
                {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            _ => (),
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
                window.request_redraw();
            }
            // Update internal state and request a redraw
            if last_tick.elapsed().as_millis() >= 25 {
                last_tick = Instant::now();
                world.update();
                window.request_redraw();
            }
        }
    });
}

struct World {
    pub cells: Vec<Cell>,
    pub width: usize,
    pub height: usize,
    pub empires: Vec<Empire>,
    pub tick: usize,
}

impl World {
    /// Create a new `World` instance that can draw a moving box.
    fn new(width: usize, height: usize) -> Self {
        let mut this = Self {
            cells: vec![Cell::default(); width * height],
            empires: vec![
                Empire {
                    id: 1,
                    color: (255, 0, 0, 255),
                },
                Empire {
                    id: 2,
                    color: (0, 255, 0, 255),
                },
                Empire {
                    id: 3,
                    color: (0, 0, 255, 255),
                },
            ],
            width,
            height,
            tick: 0,
        };
        this.cells[0] = Cell {
            owner: 1,
            troops: 255,
        };
        // this.cells[(this.width * this.height) - 1] = Cell {
        //     owner: 2,
        //     troops: 20,
        // };
        this.cells[(this.width * this.height) - 1] = Cell {
            owner: 3,
            troops: 150,
        };
        this
    }

    fn update(&mut self) {
        self.cells = self
            .cells
            .iter()
            .copied()
            .enumerate()
            .map(|(i, cell)| {
                let mut cell = cell;

                let x = (i % self.width) as isize;
                let y = (i / self.width) as isize;

                let neighbors = [
                    self.get(x - 1, y),
                    self.get(x + 1, y),
                    self.get(x, y - 1),
                    self.get(x, y + 1),
                    self.get(x - 1, y - 1),
                    self.get(x + 1, y - 1),
                    self.get(x - 1, y + 1),
                    self.get(x + 1, y + 1),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<&Cell>>();

                let num_of_friendlies = if cell.owner == 0 {
                    0
                } else {
                    neighbors.iter().filter(|v| v.owner == cell.owner).count()
                };

                let decay = (self.tick + i) % 5 == 0;
                if decay {
                    cell.troops = ((cell.troops as f32) * ((num_of_friendlies as f32) * 0.1)) as u8;
                }

                // let enemies = {
                //     let cell = cell.clone();
                // };

                let c = cell;

                for enemy in self
                    .empires
                    .iter()
                    .map(|empire| {
                        (
                            empire.id,
                            neighbors
                                .iter()
                                .filter(|v| v.owner == empire.id)
                                .choose(&mut rand::thread_rng()),
                            neighbors.iter().filter(|v| v.owner == empire.id).count(),
                        )
                    })
                    .filter_map(|v| {
                        if v.1.is_some() {
                            Some((v.0, v.1.unwrap(), v.2))
                        } else {
                            None
                        }
                    })
                    .sorted_by_key(|v| v.2)
                    .rev()
                {
                    if (num_of_friendlies < 2 || enemy.1.troops > (cell.troops)) && enemy.2 >= 1 {
                        cell.owner = enemy.0;
                        cell.troops = ((enemy.1.troops as f32)
                            * rand::thread_rng().gen_range(0.97..1.02))
                            as u8;
                        break;
                    }
                    // if cell.owner == 0 {
                    //     cell.owner = enemy.0;
                    //     cell.troops = 1;
                    //     break;
                    // }
                    // if enemy.1 == cell.troops as u16 {
                    //     cell.troops =
                    //         (cell.troops as i32 + rand::thread_rng().gen_range(-10i32..10)) as u8;
                    //     break;
                    // }
                }
                if cell.troops == 0 {
                    cell.owner = 0;
                }

                // for enemy in neighbors
                //     .iter()
                //     .filter(|v| v.owner != c.owner && v.owner != 0)
                // {
                //     if enemy.troops > cell.troops {
                //         cell.owner = enemy.owner;
                //         cell.troops = enemy.troops;
                //         // cell.troops = ((((enemy.troops as f32) * 0.75).ceil()
                //         //     + rand::thread_rng().gen_range(-10f32..10.))
                //         //     as u16)
                //         //     .clamp(1, u16::MAX);
                //         break;
                //     }
                //     if enemy.troops == cell.troops {
                //         cell.troops =
                //             (cell.troops as i32 + rand::thread_rng().gen_range(-10i32..10)) as u16;
                //         break;
                //     }
                // }

                cell
            })
            .collect();
        self.tick += 1;
    }

    fn get(&self, x: isize, y: isize) -> Option<&Cell> {
        if x < (self.width as isize) && y < (self.height as isize) && x >= 0 && y >= 0 {
            Some(&self.cells[(y as usize) * self.width + (x as usize)])
        } else {
            None
        }
    }

    /// Draw the `World` state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = i % self.width as usize;
            let y = i / self.width as usize;

            let cell = self.get(x as isize, y as isize).unwrap();

            let rgba = if cell.owner != 0 {
                let color = self.empires[(cell.owner - 1) as usize].color;
                [
                    (color.0 as f32 * (cell.troops as f32 / 128.0)) as u8,
                    (color.1 as f32 * (cell.troops as f32 / 128.0)) as u8,
                    (color.2 as f32 * (cell.troops as f32 / 128.0)) as u8,
                    color.3,
                ]
            } else {
                [0xf2, 0xd5, 0x6b, 0xff]
            };

            pixel.copy_from_slice(&rgba);
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Cell {
    pub owner: u16, // 0 = unclaimed
    pub troops: u8, // 0 is only valid if unclaimed
}

struct Empire {
    pub id: u16, // from 1
    pub color: (u8, u8, u8, u8),
}
