//! Grid mode: recursively shrink the active region, then hand off to normal mode.

use crate::keys::Action;
use crate::mode::normal::Context;
use crate::mode::{Effect, Mode};
use crate::wl::pointer::BTN_LEFT;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

#[derive(Debug, Clone)]
pub struct Grid {
    region: Rect,
    history: Vec<Rect>,
    columns: u32,
    rows: u32,
    /// A region narrower than this in either direction is small enough to hand
    /// over to normal mode for fine adjustment.
    min_span: f64,
}

impl Grid {
    pub fn new(screen: (u32, u32), columns: u32, rows: u32, min_span: f64) -> Self {
        let columns = columns.max(1);
        let rows = rows.max(1);
        Self {
            region: Rect {
                x: 0.0,
                y: 0.0,
                w: screen.0 as f64,
                h: screen.1 as f64,
            },
            history: Vec::new(),
            columns,
            rows,
            min_span: min_span.max(1.0),
        }
    }

    pub fn region(&self) -> Rect {
        self.region
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// The sub-region a given cell key selects, row major. Used by the renderer
    /// to place the key hints.
    pub fn cell(&self, index: usize) -> Option<Rect> {
        let columns = self.columns as usize;
        if index >= columns * self.rows as usize {
            return None;
        }
        let (column, row) = (index % columns, index / columns);
        Some(Rect {
            x: self.region.x + self.region.w * column as f64 / self.columns as f64,
            y: self.region.y + self.region.h * row as f64 / self.rows as f64,
            w: self.region.w / self.columns as f64,
            h: self.region.h / self.rows as f64,
        })
    }

    pub fn on_cell(&mut self, index: usize, _context: &Context) -> Vec<Effect> {
        let Some(cell) = self.cell(index) else {
            return vec![];
        };
        self.history.push(self.region);
        self.region = cell;
        let minimum = self.min_span;
        let mut effects = vec![Effect::Warp {
            x: cell.center().0,
            y: cell.center().1,
        }];
        if cell.w < minimum || cell.h < minimum {
            effects.push(Effect::Switch(Mode::Normal));
        }
        effects
    }

    pub fn on_action(&mut self, action: Action, context: &Context) -> Vec<Effect> {
        match action {
            Action::GridLeft => self.pan(-self.region.w, 0.0, context),
            Action::GridRight => self.pan(self.region.w, 0.0, context),
            Action::GridUp => self.pan(0.0, -self.region.h, context),
            Action::GridDown => self.pan(0.0, self.region.h, context),
            Action::GridBack => {
                let Some(previous) = self.history.pop() else {
                    return vec![];
                };
                self.region = previous;
                let (x, y) = previous.center();
                vec![Effect::Warp { x, y }]
            }
            Action::GridWarp => {
                let (x, y) = self.region.center();
                vec![Effect::Warp { x, y }, Effect::Switch(Mode::Normal)]
            }
            Action::GridWarpClick => {
                let (x, y) = self.region.center();
                vec![
                    Effect::Warp { x, y },
                    Effect::Click { button: BTN_LEFT },
                    Effect::Switch(Mode::Normal),
                ]
            }
            Action::Quit => vec![Effect::Exit],
            _ => vec![],
        }
    }

    fn pan(&mut self, dx: f64, dy: f64, context: &Context) -> Vec<Effect> {
        let (screen_w, screen_h) = (context.screen.0 as f64, context.screen.1 as f64);
        // Clamp into the output: a region pushed past an edge stops there rather
        // than wrapping or leaving the screen.
        let max_x = (screen_w - self.region.w).max(0.0);
        let max_y = (screen_h - self.region.h).max(0.0);
        self.region.x = (self.region.x + dx).clamp(0.0, max_x);
        self.region.y = (self.region.y + dy).clamp(0.0, max_y);
        let (x, y) = self.region.center();
        vec![Effect::Warp { x, y }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> Context {
        Context {
            cursor: (0.0, 0.0),
            screen: (1280, 720),
            step: 20.0,
        }
    }

    fn grid() -> Grid {
        Grid::new((1280, 720), 2, 2, 40.0)
    }

    #[test]
    fn the_initial_region_is_the_whole_output() {
        let grid = grid();
        assert_eq!(
            grid.region(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1280.0,
                h: 720.0
            }
        );
    }

    #[test]
    fn cells_are_row_major_and_tile_the_region() {
        let grid = grid();
        assert_eq!(
            grid.cell(0),
            Some(Rect {
                x: 0.0,
                y: 0.0,
                w: 640.0,
                h: 360.0
            })
        );
        assert_eq!(
            grid.cell(1),
            Some(Rect {
                x: 640.0,
                y: 0.0,
                w: 640.0,
                h: 360.0
            })
        );
        assert_eq!(
            grid.cell(2),
            Some(Rect {
                x: 0.0,
                y: 360.0,
                w: 640.0,
                h: 360.0
            })
        );
        assert_eq!(
            grid.cell(3),
            Some(Rect {
                x: 640.0,
                y: 360.0,
                w: 640.0,
                h: 360.0
            })
        );
        assert_eq!(grid.cell(4), None);
    }

    #[test]
    fn a_cell_key_shrinks_the_region_and_centres_the_cursor() {
        let mut grid = grid();
        let effects = grid.on_cell(1, &context());
        assert_eq!(
            effects,
            vec![Effect::Warp {
                x: 960.0,
                y: 180.0
            }]
        );
        assert_eq!(
            grid.region(),
            Rect {
                x: 640.0,
                y: 0.0,
                w: 640.0,
                h: 360.0
            }
        );
    }

    #[test]
    fn repeated_shrinking_walks_into_the_target() {
        let mut grid = grid();
        let context = context();
        grid.on_cell(1, &context);
        grid.on_cell(0, &context);
        assert_eq!(
            grid.region(),
            Rect {
                x: 640.0,
                y: 0.0,
                w: 320.0,
                h: 180.0
            }
        );
    }

    #[test]
    fn a_region_below_the_minimum_hands_over_to_normal_mode() {
        let mut grid = grid();
        let context = context();
        for step in 0..8 {
            let effects = grid.on_cell(0, &context);
            let region = grid.region();
            let minimum = 40.0;
            let small = region.w < minimum || region.h < minimum;
            if effects.contains(&Effect::Switch(Mode::Normal)) {
                assert!(small, "handed over while still {region:?}");
                return;
            }
            assert!(!small, "step {step} left {region:?} below the minimum");
        }
        panic!("never handed over to normal mode");
    }

    #[test]
    fn an_unknown_cell_index_is_ignored() {
        let mut grid = grid();
        assert_eq!(grid.on_cell(9, &context()), vec![]);
        assert_eq!(grid.region().w, 1280.0);
    }

    #[test]
    fn history_walks_back_through_the_shrinks() {
        let mut grid = grid();
        let context = context();
        grid.on_cell(1, &context);
        let inner = grid.region();
        grid.on_cell(0, &context);
        assert_ne!(grid.region(), inner);
        assert_eq!(
            grid.on_action(Action::GridBack, &context),
            vec![Effect::Warp {
                x: inner.center().0,
                y: inner.center().1
            }]
        );
        assert_eq!(grid.region(), inner);
    }

    #[test]
    fn history_back_with_nothing_to_undo_changes_nothing() {
        let mut grid = grid();
        let before = grid.region();
        assert_eq!(grid.on_action(Action::GridBack, &context()), vec![]);
        assert_eq!(grid.region(), before);
    }

    #[test]
    fn panning_moves_the_region_by_its_own_size() {
        let mut grid = grid();
        let context = context();
        for _ in 0..4 {
            grid.on_cell(0, &context);
        }
        let before = grid.region();
        grid.on_action(Action::GridRight, &context);
        assert_eq!(grid.region().x, before.x + before.w);
        assert_eq!(grid.region().y, before.y);
    }

    #[test]
    fn panning_stops_at_the_output_edge() {
        let mut grid = grid();
        let context = context();
        for _ in 0..4 {
            grid.on_cell(0, &context);
        }
        for _ in 0..20 {
            grid.on_action(Action::GridLeft, &context);
        }
        assert_eq!(grid.region().x, 0.0);
        for _ in 0..40 {
            grid.on_action(Action::GridRight, &context);
        }
        assert_eq!(grid.region().x + grid.region().w, 1280.0);
    }

    #[test]
    fn warp_keeps_the_region_and_returns_to_normal_mode() {
        let mut grid = grid();
        let context = context();
        grid.on_cell(1, &context);
        let effects = grid.on_action(Action::GridWarp, &context);
        assert_eq!(
            effects,
            vec![
                Effect::Warp {
                    x: 960.0,
                    y: 180.0
                },
                Effect::Switch(Mode::Normal)
            ]
        );
    }

    #[test]
    fn warp_and_click_clicks_after_positioning() {
        let mut grid = grid();
        let context = context();
        grid.on_cell(1, &context);
        assert_eq!(
            grid.on_action(Action::GridWarpClick, &context),
            vec![
                Effect::Warp {
                    x: 960.0,
                    y: 180.0
                },
                Effect::Click { button: BTN_LEFT },
                Effect::Switch(Mode::Normal)
            ]
        );
    }

    #[test]
    fn quit_is_reported_without_touching_the_region() {
        let mut grid = grid();
        let before = grid.region();
        assert_eq!(grid.on_action(Action::Quit, &context()), vec![Effect::Exit]);
        assert_eq!(grid.region(), before);
    }

    #[test]
    fn a_three_by_three_partition_needs_nine_cells() {
        let grid = Grid::new((900, 900), 3, 3, 40.0);
        assert_eq!(
            grid.cell(4),
            Some(Rect {
                x: 300.0,
                y: 300.0,
                w: 300.0,
                h: 300.0
            })
        );
        assert_eq!(grid.cell(8).unwrap().x, 600.0);
        assert_eq!(grid.cell(9), None);
    }
}
