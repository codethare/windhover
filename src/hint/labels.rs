//! Prefix-free label assignment over the configured character set.

/// Labels for `count` targets, in the order the targets are given.
///
/// Every label is the same length by construction, so no label is a prefix of
/// another and a unique match is a complete label. The length is the shortest
/// the alphabet can name `count` targets in, and whatever room is left over
/// becomes one-character labels for the first targets: giving up one short label
/// frees `charset.len() - 1` longer ones, so the number of short labels is the
/// spare room divided by that.
///
/// The short labels go to the targets the caller lists first, which is how a
/// target that matters more costs fewer keystrokes.
pub fn assign(count: usize, charset: &[char]) -> Vec<String> {
    if count == 0 || charset.is_empty() {
        return Vec::new();
    }
    let base = charset.len();
    // One character can only ever name one target, so an alphabet of one has
    // nothing to spread over the rest.
    if base == 1 {
        return vec![charset[0].to_string()];
    }
    let mut length = 1usize;
    while base.saturating_pow(length as u32) < count {
        length += 1;
    }
    if length == 1 {
        return charset[..count.min(base)]
            .iter()
            .map(|ch| ch.to_string())
            .collect();
    }
    // Giving up one short label costs `base^(length-1) - 1` s-char labels: the
    // shorter ones are as long as `base^(length-1)` apart. At length two that is
    // the `base - 1` of the rule this follows.
    let room = base.saturating_pow(length as u32) - count;
    let denial = base.saturating_pow(length as u32 - 1) - 1;
    let short = (room / denial).min(base);
    let mut labels: Vec<String> = charset[..short]
        .iter()
        .map(|ch| ch.to_string())
        .collect();
    // The longer labels may not start with a character a short label took, or
    // that short label would be a prefix of them.
    let tail = base.saturating_pow(length as u32 - 1);
    for index in 0..((base - short) * tail) {
        if labels.len() == count {
            break;
        }
        let first = short + index / tail;
        let rest = index % tail;
        labels.push(format!("{}{}", charset[first], label(rest, length - 1, charset)));
    }
    labels
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

    fn twenty() -> Vec<char> {
        "asdfjkl;ghqwertyuiop".chars().collect()
    }

    fn lengths(labels: &[String]) -> Vec<usize> {
        labels.iter().map(|l| l.chars().count()).collect()
    }

    #[test]
    fn a_full_lattice_is_two_characters_and_nothing_shorter() {
        // 20 x 20 = 400 leaves no room for a one-character label.
        let labels = assign(400, &twenty());
        assert_eq!(labels.len(), 400);
        assert!(lengths(&labels).iter().all(|len| *len == 2));
        assert_eq!(labels[0], "aa");
    }

    #[test]
    fn spare_room_becomes_one_character_labels_for_the_first_targets() {
        // 19 spare in the two-character budget, one short label per 19 given up:
        // 300 targets is 5 short labels and 295 of two.
        let labels = assign(300, &twenty());
        let ones = lengths(&labels).iter().filter(|len| **len == 1).count();
        assert_eq!(ones, 5);
        assert_eq!(&labels[..5], ["a", "s", "d", "f", "j"]);
        assert_eq!(labels[5], "ka", "the longer labels skipped the taken ones");
        // A count that divides exactly leaves none: 400 - 400 is no room.
        assert!(lengths(&assign(400, &twenty())).iter().all(|len| *len == 2));
        // 25 targets over a nine-character alphabet: (81-25)/8 = 7 short.
        let labels = assign(25, &charset());
        assert_eq!(lengths(&labels).iter().filter(|len| **len == 1).count(), 7);
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
        // Nine characters: up to nine targets fit in one, up to 81 in two.
        assert!(lengths(&assign(4, &charset())).iter().all(|len| *len == 1));
        assert!(lengths(&assign(9, &charset())).iter().all(|len| *len == 1));
        // Ten targets: eight of them are one character, the last two need two.
        let labels = assign(10, &charset());
        assert_eq!(labels.len(), 10);
        assert!(lengths(&labels).iter().all(|len| *len <= 2));
        assert_eq!(lengths(&labels).iter().filter(|len| **len == 1).count(), 8);
        assert!(lengths(&assign(81, &charset())).iter().all(|len| *len == 2));
        assert!(lengths(&assign(82, &charset())).iter().all(|len| *len <= 3));
        assert!(lengths(&assign(82, &charset())).iter().any(|len| *len == 3));
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
