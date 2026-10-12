//! The hint target source: a regular lattice.

/// A point of interest, in logical coordinates relative to the overlay.
pub type Point = (f64, f64);

/// Target points on a regular lattice across the output, one at the centre of
/// each cell. `cell` is the wanted spacing in logical pixels, so the number of
/// targets follows the output size instead of being fixed.
#[derive(Debug, Clone, Copy)]
pub struct Lattice {
    pub cell: f64,
}

impl Lattice {
    fn dimensions(&self, screen: (u32, u32)) -> (usize, usize) {
        let cell = self.cell.max(1.0);
        let columns = (screen.0 as f64 / cell).floor().max(1.0) as usize;
        let rows = (screen.1 as f64 / cell).floor().max(1.0) as usize;
        (columns, rows)
    }
}

impl Lattice {
    /// Target points inside an output of `screen` logical pixels.
    pub fn targets(&self, screen: (u32, u32)) -> Vec<Point> {
        let (columns, rows) = self.dimensions(screen);
        let mut points = Vec::with_capacity(columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                points.push((
                    (column as f64 + 0.5) * screen.0 as f64 / columns as f64,
                    (row as f64 + 0.5) * screen.1 as f64 / rows as f64,
                ));
            }
        }
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_smaller_cell_produces_more_targets() {
        let coarse = Lattice { cell: 200.0 };
        let fine = Lattice { cell: 50.0 };
        assert!(fine.targets((1280, 720)).len() > coarse.targets((1280, 720)).len());
    }

    #[test]
    fn targets_sit_inside_the_output() {
        let points = Lattice { cell: 110.0 }.targets((1280, 720));
        assert!(!points.is_empty());
        for (x, y) in points {
            assert!(x > 0.0 && x < 1280.0, "x = {x}");
            assert!(y > 0.0 && y < 720.0, "y = {y}");
        }
    }

    #[test]
    fn targets_are_centred_in_their_cells() {
        // Two columns (1280 / 640) and one row (720 / 640 rounds down).
        let lattice = Lattice { cell: 640.0 };
        let points = lattice.targets((1280, 720));
        assert_eq!(points, vec![(320.0, 360.0), (960.0, 360.0)]);
    }

    #[test]
    fn a_square_output_with_a_quarter_cell_gives_a_square_lattice() {
        let lattice = Lattice { cell: 320.0 };
        let points = lattice.targets((640, 640));
        assert_eq!(points.len(), 4);
        assert_eq!(points[0], (160.0, 160.0));
        assert_eq!(points[3], (480.0, 480.0));
    }

    #[test]
    fn an_absurd_cell_still_yields_one_target() {
        let lattice = Lattice { cell: 100_000.0 };
        assert_eq!(lattice.targets((1280, 720)), vec![(640.0, 360.0)]);
    }
}
