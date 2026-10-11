//! The `HintSource` interface shared by the lattice and element sources.

/// A point of interest, in logical coordinates relative to the overlay.
pub type Point = (f64, f64);

pub trait HintSource {
    /// Target points inside the output the overlay covers.
    ///
    /// `origin` is the logical position of that output within the compositor's
    /// global space, which is what the accessibility tree reports extents in;
    /// `screen` is the overlay's own size. A source that has nothing to offer
    /// returns an empty list rather than failing.
    fn targets(&mut self, origin: (i32, i32), screen: (u32, u32)) -> Vec<Point>;
}

pub use super::lattice::Lattice;

/// Try `primary` first and fall back to `fallback` when it has nothing.
///
/// The element source is allowed to come back empty — no accessibility bus, no
/// candidates, or too slow — and hint mode has to keep working in all three
/// cases. Sharing the fallback means the label assignment and filtering are the
/// same either way.
#[cfg(feature = "atspi")]
pub struct Cascade {
    pub primary: Box<dyn HintSource>,
    pub fallback: Lattice,
}

#[cfg(feature = "atspi")]
impl HintSource for Cascade {
    fn targets(&mut self, origin: (i32, i32), screen: (u32, u32)) -> Vec<Point> {
        let points = self.primary.targets(origin, screen);
        if points.is_empty() {
            return self.fallback.targets(origin, screen);
        }
        points
    }
}

/// The source hint mode uses in this build.
#[cfg(feature = "atspi")]
pub fn select(cell: f64, budget: std::time::Duration) -> Box<dyn HintSource> {
    Box::new(Cascade {
        primary: Box::new(super::atspi::Elements { budget }),
        fallback: Lattice { cell },
    })
}

#[cfg(not(feature = "atspi"))]
pub fn select(cell: f64, budget: std::time::Duration) -> Box<dyn HintSource> {
    // The lattice is the only source in this build, so the budget is unused.
    let _ = budget;
    Box::new(Lattice { cell })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "atspi")]
    #[cfg(feature = "atspi")]
    struct Empty;
    #[cfg(feature = "atspi")]
    impl HintSource for Empty {
        fn targets(&mut self, _: (i32, i32), _: (u32, u32)) -> Vec<Point> {
            Vec::new()
        }
    }

    #[cfg(feature = "atspi")]
    #[cfg(feature = "atspi")]
    struct Fixed(Vec<Point>);
    #[cfg(feature = "atspi")]
    impl HintSource for Fixed {
        fn targets(&mut self, _: (i32, i32), _: (u32, u32)) -> Vec<Point> {
            self.0.clone()
        }
    }

    #[cfg(feature = "atspi")]
    fn cascade(primary: Box<dyn HintSource>) -> Cascade {
        Cascade {
            primary,
            fallback: Lattice { cell: 640.0 },
        }
    }

    #[cfg(feature = "atspi")]
    #[test]
    fn an_empty_primary_falls_back_to_the_lattice() {
        let mut source = cascade(Box::new(Empty));
        let points = source.targets((0, 0), (1280, 720));
        assert_eq!(points, vec![(320.0, 360.0), (960.0, 360.0)]);
    }

    #[cfg(feature = "atspi")]
    #[test]
    fn a_non_empty_primary_is_used_as_is() {
        let mut source = cascade(Box::new(Fixed(vec![(10.0, 20.0)])));
        assert_eq!(source.targets((0, 0), (1280, 720)), vec![(10.0, 20.0)]);
    }

    #[cfg(not(feature = "atspi"))]
    #[test]
    fn the_default_build_has_no_element_source() {
        let mut source = select(640.0, std::time::Duration::from_millis(1));
        assert_eq!(source.targets((0, 0), (1280, 720)).len(), 2);
    }
}
