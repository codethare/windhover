//! Element-level target source backed by the accessibility tree (feature `atspi`).
//!
//! The walk is asynchronous because the AT-SPI bindings are, and it runs on a
//! thread with a deadline: a hung D-Bus call must cost the session a bounded
//! wait, not a freeze. Anything that goes wrong — no accessibility bus, an
//! error mid-walk, or simply taking too long — comes back as "no candidates",
//! which is what makes the caller fall back to the lattice.

use std::sync::mpsc;
use std::time::Duration;

use atspi::connection::AccessibilityConnection;
use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::proxy_ext::ProxyExt;
use atspi::zbus::proxy::CacheProperties;
use atspi::{CoordType, Role};
use futures_executor::block_on;

use super::source::{HintSource, Point};

/// How many elements to visit before giving up on the tree. Accessibility trees
/// include every label, list item and table cell; the budget is what keeps the
/// walk proportional to the screen rather than to the application.
const MAX_NODES: u32 = 1500;
const MAX_DEPTH: usize = 24;

pub struct Elements {
    pub budget: Duration,
}

impl HintSource for Elements {
    fn targets(&mut self, origin: (i32, i32), screen: (u32, u32)) -> Vec<Point> {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            // The receiver may already have given up; that is the timeout case.
            let _ = sender.send(block_on(walk_tree(origin, screen)));
        });
        match receiver.recv_timeout(self.budget) {
            Ok(points) => points,
            Err(_) => {
                eprintln!(
                    "windhover: accessibility tree did not answer within {:?}; using the lattice",
                    self.budget
                );
                Vec::new()
            }
        }
    }
}

async fn walk_tree(origin: (i32, i32), screen: (u32, u32)) -> Vec<Point> {
    let Ok(connection) = AccessibilityConnection::new().await else {
        // No accessibility bus is the normal case on a desktop that has not
        // been asked for one, and it is not worth a warning on every invocation.
        return Vec::new();
    };
    let Ok(root) = connection.root_accessible_on_registry().await else {
        return Vec::new();
    };

    // An explicit stack rather than recursion: the bound is a node count, and
    // the futures this would otherwise produce would have to be boxed.
    let mut points = Vec::new();
    let mut stack = vec![(root, 0usize)];
    let mut visited = 0u32;

    while let Some((proxy, depth)) = stack.pop() {
        if visited >= MAX_NODES {
            break;
        }
        visited += 1;

        if depth <= MAX_DEPTH {
            if let Ok(role) = proxy.get_role().await {
                if is_interactive(role) {
                    if let Some(point) = centre_of(&proxy, origin).await {
                        if inside(point, screen) {
                            points.push(point);
                        }
                        // An interactive element's children are its own parts,
                        // not separate targets.
                        continue;
                    }
                }
            }
        }

        if depth >= MAX_DEPTH {
            continue;
        }
        let Ok(children) = proxy.get_children().await else {
            continue;
        };
        for child in children {
            // Owned copies: `child` is dropped at the end of the loop body while
            // the builder still needs its name and path.
            let Some(name) = child.name().map(|name| name.to_string()) else {
                continue;
            };
            let path = child.path().to_string();
            // The owned name and path are moved in: a borrowed one would have
            // to live as long as the connection.
            let Ok(builder) = AccessibleProxy::builder(connection.connection())
                .destination(name)
                .and_then(|builder| builder.path(path))
            else {
                continue;
            };
            // Caching is off because a walk wants the tree as it is now, not as
            // the cache last saw it.
            let Ok(built) = builder.cache_properties(CacheProperties::No).build().await else {
                continue;
            };
            stack.push((built, depth + 1));
        }
    }

    points
}

async fn centre_of(proxy: &AccessibleProxy<'_>, origin: (i32, i32)) -> Option<Point> {
    let component = proxy.proxies().await.ok()?.component().await.ok()?;
    let (x, y, width, height) = component.get_extents(CoordType::Screen).await.ok()?;
    if width <= 0 || height <= 0 {
        return None;
    }
    // Extents are in compositor-global coordinates; the overlay works in its own.
    Some((
        (x + width / 2 - origin.0) as f64,
        (y + height / 2 - origin.1) as f64,
    ))
}

fn inside(point: Point, screen: (u32, u32)) -> bool {
    point.0 >= 0.0 && point.1 >= 0.0 && point.0 <= screen.0 as f64 && point.1 <= screen.1 as f64
}

/// The roles worth putting a label on: things a pointer is expected to press,
/// type into or drag. Deliberately narrow — labelling every label and list item
/// would bury the useful targets.
fn is_interactive(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::ToggleButton
            | Role::CheckBox
            | Role::RadioButton
            | Role::CheckMenuItem
            | Role::RadioMenuItem
            | Role::MenuItem
            | Role::Link
            | Role::Entry
            | Role::ComboBox
            | Role::ListItem
            | Role::PageTab
            | Role::Slider
            | Role::SpinButton
            | Role::Icon
            | Role::TableCell
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn a_budget_of_zero_gives_up_immediately() {
        let mut elements = Elements {
            budget: Duration::ZERO,
        };
        assert!(elements.targets((0, 0), (1280, 720)).is_empty());
    }

    #[test]
    fn the_budget_bounds_the_wait() {
        // Whether or not an accessibility bus is running, the call has to come
        // back inside its budget: a hang here would freeze the session.
        let budget = Duration::from_millis(50);
        let mut elements = Elements { budget };
        let started = Instant::now();
        let _ = elements.targets((0, 0), (1280, 720));
        assert!(
            started.elapsed() < budget * 6,
            "the accessibility walk overran its budget: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn an_empty_result_falls_back_to_the_lattice() {
        let mut source = crate::hint::source::Cascade {
            primary: Box::new(Elements {
                budget: Duration::ZERO,
            }),
            fallback: crate::hint::source::Lattice { cell: 640.0 },
        };
        assert_eq!(
            source.targets((0, 0), (1280, 720)),
            vec![(320.0, 360.0), (960.0, 360.0)]
        );
    }

    #[test]
    fn extents_are_translated_to_overlay_coordinates() {
        // The walker subtracts the output origin; a second output's elements
        // must not land inside the first output's overlay.
        assert!(inside((10.0, 10.0), (1280, 720)));
        assert!(!inside((-10.0, 10.0), (1280, 720)));
        assert!(!inside((10.0, 10.0), (0, 0)));
    }

    #[test]
    fn the_interactive_roles_do_not_include_plain_labels() {
        assert!(is_interactive(Role::Button));
        assert!(is_interactive(Role::Entry));
        assert!(is_interactive(Role::ListItem));
        assert!(!is_interactive(Role::Label));
        assert!(!is_interactive(Role::Paragraph));
        assert!(!is_interactive(Role::Frame));
    }
}
