//! Hint mode: labelled targets, prefix filtering, warp on unique match.

use crate::hint::labels;
use crate::keys::Action;
use crate::mode::{Effect, Mode};
use crate::wl::pointer::BTN_LEFT;

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub x: f64,
    pub y: f64,
    pub label: String,
}

#[derive(Debug, Clone, Default)]
pub struct Hint {
    targets: Vec<Target>,
    typed: Vec<char>,
}

impl Hint {
    pub fn new(points: Vec<(f64, f64)>, charset: &[char]) -> Self {
        let labels = labels::assign(points.len(), charset);
        let targets = points
            .into_iter()
            .zip(labels)
            .map(|((x, y), label)| Target { x, y, label })
            .collect();
        Self {
            targets,
            typed: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn typed(&self) -> &[char] {
        &self.typed
    }

    #[cfg(test)]
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }

    /// The targets still matching what has been typed, each with the number of
    /// leading label characters that matched, so the renderer can dim them.
    pub fn visible(&self) -> Vec<(&Target, usize)> {
        self.targets
            .iter()
            .filter(|target| starts_with(&target.label, &self.typed))
            .map(|target| (target, self.typed.len()))
            .collect()
    }

    pub fn on_char(&mut self, ch: char, click_after_warp: bool) -> Vec<Effect> {
        self.typed.push(ch);
        let matching: Vec<&Target> = self
            .targets
            .iter()
            .filter(|target| starts_with(&target.label, &self.typed))
            .collect();

        if matching.is_empty() {
            // A character that matches nothing is not an error, it is simply
            // not part of any label: typing it must leave the state alone.
            self.typed.pop();
            return vec![];
        }

        let hit = matching
            .iter()
            .find(|target| target.label.chars().count() == self.typed.len());
        let Some(target) = hit else {
            return vec![];
        };

        let mut effects = vec![Effect::Warp {
            x: target.x,
            y: target.y,
        }];
        if click_after_warp {
            effects.push(Effect::Click { button: BTN_LEFT });
        }
        effects.push(Effect::Switch(Mode::Normal));
        effects
    }

    pub fn on_action(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::HintBack => {
                // Backspace with nothing typed is the way back to normal mode.
                if self.typed.pop().is_none() {
                    return vec![Effect::Switch(Mode::Normal)];
                }
                vec![]
            }
            Action::Quit => vec![Effect::Exit],
            _ => vec![],
        }
    }
}

fn starts_with(label: &str, prefix: &[char]) -> bool {
    let mut chars = label.chars();
    prefix.iter().all(|expected| chars.next() == Some(*expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn charset() -> Vec<char> {
        "asdfghjkl".chars().collect()
    }

    fn hint(count: usize) -> Hint {
        let points = (0..count)
            .map(|i| ((i as f64 + 0.5) * 10.0, (i as f64 + 0.5) * 7.0))
            .collect();
        Hint::new(points, &charset())
    }

    #[test]
    fn targets_carry_their_point_and_label() {
        let hint = hint(4);
        assert_eq!(hint.target_count(), 4);
        assert_eq!(hint.visible().len(), 4);
        assert_eq!(hint.typed(), &[] as &[char]);
    }

    #[test]
    fn typing_filters_to_the_matching_labels() {
        let mut hint = hint(10);
        let labels: Vec<String> = hint.targets.iter().map(|t| t.label.clone()).collect();
        let first = labels[0].chars().next().unwrap();
        hint.on_char(first, false);
        let matching = labels
            .iter()
            .filter(|label| label.starts_with(first))
            .count();
        assert_eq!(hint.visible().len(), matching);
    }

    #[test]
    fn an_unmatched_character_is_ignored() {
        let mut hint = hint(10);
        hint.on_char('z', false);
        assert_eq!(hint.typed(), &[] as &[char]);
        assert_eq!(hint.visible().len(), 10);
    }

    #[test]
    fn a_unique_match_warps_and_returns_to_normal_mode() {
        let mut hint = hint(10);
        let target = hint.targets[0].clone();
        for ch in target.label.chars() {
            let effects = hint.on_char(ch, false);
            if !effects.is_empty() {
                assert_eq!(
                    effects,
                    vec![
                        Effect::Warp {
                            x: target.x,
                            y: target.y
                        },
                        Effect::Switch(Mode::Normal)
                    ]
                );
                return;
            }
        }
        panic!("typing the whole label never produced an effect");
    }

    #[test]
    fn clicking_after_warp_clicks_between_the_warp_and_the_handover() {
        let mut hint = hint(10);
        let target = hint.targets[0].clone();
        for ch in target.label.chars() {
            let effects = hint.on_char(ch, true);
            if !effects.is_empty() {
                assert_eq!(
                    effects,
                    vec![
                        Effect::Warp {
                            x: target.x,
                            y: target.y
                        },
                        Effect::Click { button: BTN_LEFT },
                        Effect::Switch(Mode::Normal)
                    ]
                );
                return;
            }
        }
        panic!("typing the whole label never produced an effect");
    }

    #[test]
    fn convergence_is_monotonic() {
        let mut hint = hint(200);
        let mut remaining = hint.visible().len();
        let label: Vec<char> = hint.targets[0].label.chars().collect();
        for ch in label {
            hint.on_char(ch, false);
            let now = hint.visible().len();
            assert!(now <= remaining, "{now} grew past {remaining}");
            remaining = now;
        }
        assert_eq!(remaining, 1);
    }

    #[test]
    fn backspace_edits_one_character_at_a_time() {
        let mut hint = hint(20);
        let first = hint.targets[0].label.chars().next().unwrap();
        hint.on_char(first, false);
        let filtered = hint.visible().len();
        assert_eq!(hint.on_action(Action::HintBack), vec![]);
        assert_eq!(hint.typed(), &[] as &[char]);
        assert_eq!(hint.visible().len(), 20);
        assert!(filtered < 20);
    }

    #[test]
    fn backspace_with_nothing_typed_returns_to_normal_mode() {
        let mut hint = hint(20);
        assert_eq!(
            hint.on_action(Action::HintBack),
            vec![Effect::Switch(Mode::Normal)]
        );
    }

    #[test]
    fn quit_is_reported() {
        let mut hint = hint(4);
        assert_eq!(hint.on_action(Action::Quit), vec![Effect::Exit]);
    }

    #[test]
    fn an_empty_source_still_produces_a_usable_mode() {
        let mut hint = Hint::new(Vec::new(), &charset());
        assert_eq!(hint.target_count(), 0);
        assert!(hint.on_char('a', false).is_empty());
        assert_eq!(hint.visible().len(), 0);
    }
}
