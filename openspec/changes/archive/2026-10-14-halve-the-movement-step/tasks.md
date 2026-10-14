# Tasks

## 1. The step

- [x] 1.1 Derive the step from `output_height / 72`, and halve the floor with it from 8 to 4 so a small output keeps moving by the same fraction of itself. Verify: `cargo test`.
- [x] 1.2 Update the test expectations that read a position after a press, from the distances the tests' own arithmetic implies. Verify: `cargo test`, which lists every one of them when the derivation changes.

## 2. Records

- [x] 2.1 The `step` row in `docs/configuration.md` and the value written out in its sample configuration. The sample is parsed by a test, so it has to stay valid. Verify: `cargo test`.
- [x] 2.2 The end-to-end expectation helper computes `s` from the output geometry, so it follows in one line. Verify: `scripts/headless-check.sh` on sway, river and niri, whose positional checks are all expressed in terms of `s`.

## 3. Not changed, deliberately

- [x] 3.1 The gain of a hold: the step still grows to four times over the intervals it stays down, so a hold covers half the ground per repeat and the same ground per press-and-hold shape. Verify: the growth tests pass unchanged in form.
- [x] 3.2 An explicit `step:` in a configuration still wins, which is the way back to the old distance. Verify: the configuration checks set their own step and are unaffected.
