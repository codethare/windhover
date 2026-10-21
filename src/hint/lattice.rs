//! The hint target source: a regular lattice.

/// A point of interest, in logical coordinates relative to the overlay.
pub type Point = (f64, f64);

/// One cell of the lattice, in logical coordinates relative to the overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Cell {
    pub fn centre(&self) -> Point {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// Target points on a regular lattice across the output, one at the centre of
/// each cell. `cell` is the wanted spacing in logical pixels, so the number of
/// targets follows the output size instead of being fixed.
#[derive(Debug, Clone, Copy)]
pub struct Lattice {
    pub cell: f64,
    /// The most cells either axis may have. One alphabet character names each
    /// axis, so the lattice can never be wider or taller than the alphabet —
    /// which is what keeps every label at two characters.
    pub max_axis: usize,
}

impl Lattice {
    #[cfg(test)]
    pub fn new(cell: f64) -> Self {
        Self {
            cell,
            max_axis: usize::MAX,
        }
    }

    pub fn capped(cell: f64, max_axis: usize) -> Self {
        Self {
            cell,
            max_axis: max_axis.max(1),
        }
    }

    fn dimensions(&self, screen: (u32, u32)) -> (usize, usize) {
        let cell = self.cell.max(1.0);
        let columns = ((screen.0 as f64 / cell).floor().max(1.0) as usize).min(self.max_axis);
        let rows = ((screen.1 as f64 / cell).floor().max(1.0) as usize).min(self.max_axis);
        (columns, rows)
    }
}

impl Lattice {
    /// The cells that tile the output, row major. The cells are distributed
    /// evenly, so they cover the output exactly at whatever spacing the wanted
    /// cell size rounds to.
    pub fn cells(&self, screen: (u32, u32)) -> Vec<Cell> {
        let (columns, rows) = self.dimensions(screen);
        let (w, h) = (
            screen.0 as f64 / columns as f64,
            screen.1 as f64 / rows as f64,
        );
        let mut cells = Vec::with_capacity(columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                cells.push(Cell {
                    x: column as f64 * w,
                    y: row as f64 * h,
                    w,
                    h,
                });
            }
        }
        cells
    }

    /// The cell boundaries, as x and y positions, outer edges included, so a
    /// grid drawn from these closes.
    pub fn lines(&self, screen: (u32, u32)) -> (Vec<f64>, Vec<f64>) {
        let (columns, rows) = self.dimensions(screen);
        let xs = (0..=columns)
            .map(|i| i as f64 * screen.0 as f64 / columns as f64)
            .collect();
        let ys = (0..=rows)
            .map(|i| i as f64 * screen.1 as f64 / rows as f64)
            .collect();
        (xs, ys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_smaller_cell_produces_more_targets() {
        let coarse = Lattice::new(200.0);
        let fine = Lattice::new(50.0);
        assert!(fine.cells((1280, 720)).len() > coarse.cells((1280, 720)).len());
    }

    #[test]
    fn a_target_sits_inside_the_output() {
        let cells = Lattice::new(110.0).cells((1280, 720));
        assert!(!cells.is_empty());
        for cell in cells {
            let (x, y) = cell.centre();
            assert!(x > 0.0 && x < 1280.0, "x = {x}");
            assert!(y > 0.0 && y < 720.0, "y = {y}");
        }
    }

    #[test]
    fn targets_are_centred_in_their_cells() {
        // Two columns (1280 / 640) and one row (720 / 640 rounds down).
        let cells = Lattice::new(640.0).cells((1280, 720));
        let centres: Vec<Point> = cells.iter().map(Cell::centre).collect();
        assert_eq!(centres, vec![(320.0, 360.0), (960.0, 360.0)]);
    }

    #[test]
    fn a_square_output_with_a_quarter_cell_gives_a_square_lattice() {
        let cells = Lattice::new(320.0).cells((640, 640));
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0].centre(), (160.0, 160.0));
        assert_eq!(cells[3].centre(), (480.0, 480.0));
    }

    #[test]
    fn an_absurd_cell_still_yields_one_target() {
        let cells = Lattice::new(100_000.0).cells((1280, 720));
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].centre(), (640.0, 360.0));
    }

    #[test]
    fn the_cap_bounds_each_axis_to_the_alphabet() {
        // A wide screen wants more columns than the alphabet can name, so the
        // cells get wider rather than the labels longer.
        let wanted = Lattice::new(60.0).cells((1280, 720));
        assert_eq!(wanted.len(), 21 * 12);
        let capped = Lattice::capped(60.0, 20).cells((1280, 720));
        assert_eq!(capped.len(), 20 * 12);
        assert_eq!(capped[0].w, 64.0);
    }

    #[test]
    fn the_lines_are_the_cell_boundaries() {
        let lines = Lattice::new(110.0).lines((1280, 720));
        // 11 columns and 6 rows, so one more boundary than cells on each axis.
        assert_eq!(lines.0.len(), 12);
        assert_eq!(lines.1.len(), 7);
        assert_eq!(lines.0[0], 0.0);
        assert_eq!(*lines.0.last().unwrap(), 1280.0);
        assert_eq!(lines.1[0], 0.0);
        assert_eq!(*lines.1.last().unwrap(), 720.0);
        // Every boundary is a cell's near edge, in order.
        let cells = Lattice::new(110.0).cells((1280, 720));
        assert_eq!(lines.0[3], cells[3].x);
        assert_eq!(lines.1[2], cells[22].y);
    }

    #[test]
    fn the_cells_tile_the_output() {
        let cells = Lattice::new(110.0).cells((1280, 720));
        let (columns, rows) = (11, 6);
        assert_eq!(cells.len(), columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                let cell = cells[row * columns + column];
                assert_eq!(cell.x, column as f64 * cell.w);
                assert_eq!(cell.y, row as f64 * cell.h);
            }
        }
        // No gaps and no overlap: the last cell's far edge is the output's.
        let last = cells[cells.len() - 1];
        assert!((cells[columns - 1].x + cells[columns - 1].w - 1280.0).abs() < 1e-9);
        assert!((last.y + last.h - 720.0).abs() < 1e-9);
    }
}
