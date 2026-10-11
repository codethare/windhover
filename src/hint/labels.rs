//! Prefix-free label assignment over the configured character set.

/// Fixed-length labels over `charset`.
///
/// Because every label is the same length, no label can be a prefix of another,
/// which is what lets hint mode act the moment a typed prefix matches exactly
/// one target: a unique match *is* a complete label.
pub fn assign(count: usize, charset: &[char]) -> Vec<String> {
    if count == 0 || charset.is_empty() {
        return Vec::new();
    }
    let base = charset.len();
    // Cap the length so a nonsense configuration cannot ask for a label longer
    // than the screen could ever need.
    let mut length = 1usize;
    while length < 8 && base.saturating_pow(length as u32) < count {
        length += 1;
    }
    (0..count).map(|i| label(i, length, charset)).collect()
}

fn label(mut index: usize, length: usize, charset: &[char]) -> String {
    let mut digits = vec![charset[0]; length];
    for slot in (0..length).rev() {
        digits[slot] = charset[index % charset.len()];
        index /= charset.len();
    }
    digits.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn charset() -> Vec<char> {
        "asdfghjkl".chars().collect()
    }

    #[test]
    fn labels_are_unique() {
        let labels = assign(80, &charset());
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), 80);
    }

    #[test]
    fn no_label_is_a_prefix_of_another() {
        let labels = assign(700, &charset());
        for outer in &labels {
            for inner in &labels {
                if outer != inner {
                    assert!(
                        !inner.starts_with(outer.as_str()),
                        "`{inner}` starts with `{outer}`"
                    );
                }
            }
        }
    }

    #[test]
    fn the_length_grows_with_the_target_count() {
        assert!(assign(4, &charset()).iter().all(|l| l.chars().count() == 1));
        assert!(assign(9, &charset()).iter().all(|l| l.chars().count() == 1));
        assert!(assign(10, &charset()).iter().all(|l| l.chars().count() == 2));
        assert!(assign(81, &charset()).iter().all(|l| l.chars().count() == 2));
        assert!(assign(82, &charset()).iter().all(|l| l.chars().count() == 3));
    }

    #[test]
    fn every_label_uses_only_the_configured_characters() {
        for label in assign(40, &charset()) {
            assert!(label.chars().all(|ch| charset().contains(&ch)), "{label}");
        }
    }

    #[test]
    fn an_empty_request_produces_nothing() {
        assert!(assign(0, &charset()).is_empty());
    }

    #[test]
    fn an_empty_charset_produces_nothing_rather_than_looping() {
        assert!(assign(10, &[]).is_empty());
    }
}
