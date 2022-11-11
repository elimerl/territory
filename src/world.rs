use itertools::Itertools;
use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng,
};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

pub struct World {
    pub cells: Vec<Cell>,
    pub width: usize,
    pub height: usize,
    pub empires: Vec<Empire>,
    pub tick: usize,
}
impl World {
    /// Create a new `World` instance that can draw a moving box.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![Cell::default(); width * height],
            empires: vec![],
            width,
            height,
            tick: 0,
        }
    }

    pub fn update(&mut self) {
        let mut buf = self.cells.clone();

        for x in 0isize..self.width as isize {
            for y in 0isize..self.height as isize {
                let mut cell = *self.get(x, y).unwrap();

                let mut neighbors = [
                    self.get(x - 1, y),
                    self.get(x + 1, y),
                    self.get(x, y - 1),
                    self.get(x, y + 1),
                    self.get(x - 1, y - 1),
                    self.get(x + 1, y - 1),
                    self.get(x - 1, y + 1),
                    self.get(x + 1, y + 1),
                ];
                neighbors.shuffle(&mut rand::thread_rng());

                cell.troops = (cell.troops as f32 * 0.95) as u16;

                for neighbor in neighbors.iter().flatten() {
                    if neighbor.owner == cell.owner && neighbor.troops > cell.troops {
                        cell.owner = neighbor.owner;
                        cell.troops = (neighbor.troops as f32
                            * rand::thread_rng().gen_range(0.98..1.01))
                            as u16;
                        break;
                    }
                    if neighbor.troops > cell.troops {
                        cell.owner = neighbor.owner;
                        cell.troops = (neighbor.troops as f32
                            * rand::thread_rng().gen_range(0.98..1.01))
                            as u16;
                        break;
                    }
                }

                if cell.owner == 0 {
                    cell.troops = 0;
                }

                buf[(y as usize) * self.width + (x as usize)] = cell;
            }
        }

        self.cells = buf;

        // self.cells = self
        //     .cells
        //     .par_iter()
        //     .copied()
        //     .enumerate()
        //     .map(|(i, cell)| {
        //         let mut cell = cell;

        //         let x = (i % self.width) as isize;
        //         let y = (i / self.width) as isize;

        //         let mut neighbors = [
        //             self.get(x - 1, y),
        //             self.get(x + 1, y),
        //             self.get(x, y - 1),
        //             self.get(x, y + 1),
        //             self.get(x - 1, y - 1),
        //             self.get(x + 1, y - 1),
        //             self.get(x - 1, y + 1),
        //             self.get(x + 1, y + 1),
        //         ]
        //         .into_iter()
        //         .flatten()
        //         .collect::<Vec<_>>();
        //         neighbors.shuffle(&mut rand::thread_rng());

        //         let num_of_friendlies = if cell.owner == 0 {
        //             0
        //         } else {
        //             neighbors.iter().filter(|v| v.owner == cell.owner).count()
        //         };

        //         // cell.troops = (cell.troops as f32 * rand::thread_rng().gen_range(0.99..1.02)) as u8;

        //         // Decay
        //         if (self.tick + i) % rand::thread_rng().gen_range(3..5) == 0 {
        //             cell.troops = (cell.troops as f32
        //                 * (rand::thread_rng().gen_range(0.05..0.13) * num_of_friendlies as f32))
        //                 as u8;
        //         }

        //         // Takeover
        //         for neighbor in neighbors {
        //             if neighbor.owner == 0 {
        //                 continue;
        //             }
        //             if num_of_friendlies < 2
        //                 || neighbor.troops > cell.troops && (rand::random::<u8>() < neighbor.troops)
        //             {
        //                 cell.owner = neighbor.owner;
        //                 cell.troops = (neighbor.troops as f32
        //                     * rand::thread_rng().gen_range(0.9..1.01))
        //                     as u8;
        //                 break;
        //             }
        //             // if neighbor.troops.abs_diff(cell.troops) < 32 {
        //             //     cell.troops =
        //             //         (cell.troops as f32 * rand::thread_rng().gen_range(0.9..1.1f32)) as u8;
        //             // }
        //             // if neighbor.owner != cell.owner && neighbor.troops == cell.troops {
        //             //     cell.troops =
        //             //         (cell.troops as f32 * rand::thread_rng().gen_range(0.1..1.1f32)) as u8;
        //             // }
        //         }

        //         if cell.troops == 0 {
        //             cell.owner = 0;
        //         }
        //         cell.troops = cell.troops.clamp(0, self.max_troops);

        //         cell
        //     })
        //     .collect();
        self.tick += 1;
    }

    pub fn get(&self, x: isize, y: isize) -> Option<&Cell> {
        // if x < 0 || x >= self.width as isize || y < 0 || y >= self.height as isize {
        //     None
        // } else {
        Some(
            &self.cells[(y.rem_euclid(self.height as isize) as usize) * self.width
                + (x.rem_euclid(self.width as isize) as usize)],
        )
        // }
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
                    (color.0 as f32 * (cell.troops as f32 / 65355.0)) as u8,
                    (color.1 as f32 * (cell.troops as f32 / 65355.0)) as u8,
                    (color.2 as f32 * (cell.troops as f32 / 65355.0)) as u8,
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
    pub troops: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Empire {
    pub id: u16, // from 1
    pub color: (u8, u8, u8, u8),
}
