use itertools::Itertools;
use rand::{seq::IteratorRandom, Rng};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

pub struct World {
    pub cells: Vec<Cell>,
    pub width: usize,
    pub height: usize,
    pub empires: Vec<Empire>,
    pub tick: usize,
    pub max_troops: u8,
}
impl World {
    /// Create a new `World` instance that can draw a moving box.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
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
            max_troops: 255,
        }
    }

    pub fn update(&mut self) {
        self.cells = self
            .cells
            .par_iter()
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
                .flatten();

                let num_of_friendlies = if cell.owner == 0 {
                    0
                } else {
                    neighbors.clone().filter(|v| v.owner == cell.owner).count()
                };

                // grow
                let should_grow = (self.tick + i) % 2 == 0;
                if should_grow {
                    cell.troops = (cell.troops as f32 * (0.1 * num_of_friendlies as f32)) as u8;
                }

                for enemy in self
                    .empires
                    .iter()
                    .map(|empire| {
                        (
                            empire.id,
                            neighbors
                                .clone()
                                .filter(|v| v.owner == empire.id)
                                .choose(&mut rand::thread_rng()),
                            neighbors.clone().filter(|v| v.owner == empire.id).count(),
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
                    if enemy.0 == cell.owner && enemy.1.troops > cell.troops {
                        cell.troops = ((enemy.1.troops as f32)
                            * rand::thread_rng().gen_range(0.98..1.))
                            as u8;
                        break;
                    }
                    if enemy.1.troops > cell.troops {
                        cell.owner = enemy.0;
                        cell.troops = ((enemy.1.troops as f32)
                            * rand::thread_rng().gen_range(0.97..1.03))
                            as u8;
                        break;
                    }
                }
                if (cell.troops < (self.max_troops as f32 * 1.0 / 16.0) as u8)
                    && rand::thread_rng().gen_bool(0.1)
                {
                    cell.owner = rand::thread_rng().gen_range(0..=self.empires.len()) as u16;
                    cell.troops = (cell.troops as f32
                        + rand::thread_rng().gen_range(1.0..self.max_troops as f32))
                        as u8;
                }
                if cell.troops == 0 {
                    cell.owner = 0;
                }
                cell.troops = cell.troops.clamp(0, self.max_troops);

                cell
            })
            .collect();
        self.tick += 1;
    }

    pub fn get(&self, x: isize, y: isize) -> Option<&Cell> {
        if x < (self.width as isize) && y < (self.height as isize) && x >= 0 && y >= 0 {
            Some(&self.cells[(y as usize) * self.width + (x as usize)])
        } else {
            None
        }
    }
    pub fn set(&mut self, x: isize, y: isize, val: Cell) {
        assert!(x >= 0 && x < (self.width as isize));
        assert!(y >= 0 && y < (self.height as isize));

        self.cells[(y as usize) * self.width + (x as usize)] = val;
    }

    /// Draw the `World` state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    pub fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = i % self.width as usize;
            let y = i / self.width as usize;

            let cell = self.get(x as isize, y as isize).unwrap();

            let rgba = if cell.owner != 0 {
                let color = self.empires[(cell.owner - 1) as usize].color;
                [
                    (color.0 as f32 * (cell.troops as f32 / self.max_troops as f32).clamp(0.1, 1.))
                        as u8,
                    (color.1 as f32 * (cell.troops as f32 / self.max_troops as f32).clamp(0.1, 1.))
                        as u8,
                    (color.2 as f32 * (cell.troops as f32 / self.max_troops as f32).clamp(0.1, 1.))
                        as u8,
                    color.3,
                ]
            } else {
                [0x00, 0x00, 0x00, 0xff]
            };

            pixel.copy_from_slice(&rgba);
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cell {
    pub owner: u16, // 0 = unclaimed
    pub troops: u8, // 0 is only valid if unclaimed
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Empire {
    pub id: u16, // from 1
    pub color: (u8, u8, u8, u8),
}
