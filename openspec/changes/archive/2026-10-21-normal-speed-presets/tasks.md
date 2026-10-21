# Tasks

## 1. The preset actions

- [x] 1.1 Add the three preset actions to `src/keys.rs` and bind them to `[`, `\`
      and `]` in the normal-mode table, with a test that the defaults parse
- [x] 1.2 Add `speed_fine`, `speed_normal` and `speed_coarse` to `action_for` and
      `action_option` in `src/config.rs`, with a test that each option binds and
      one that `unbind` removes it
- [x] 1.3 Document the three options in `docs/configuration.md` (sample and
      options table) and the README's normal-mode row

## 2. The preset state

- [x] 2.1 Add an `Effect::Speed(f64)` variant and return it from the three
      actions in `src/mode/normal.rs`, with unit tests for each
- [x] 2.2 Hold the preset on the session, multiply it into the step, apply the
      effect in `Session::apply`, and unit-test that a move after selecting the
      fine or coarse preset uses the scaled step
- [x] 2.3 Unit-test that the preset survives a hand-over: select fine, switch to
      hint, switch back, and check the next move is still scaled

## 3. Showing it

- [x] 3.1 Carry the preset on `Frame` and append it to the status text whenever
      it is not `1x`, leaving the existing multiplier and drag text unchanged
- [x] 3.2 Extend the render test so a frame with a preset draws status text and a
      frame with the normal preset still draws none

## 4. End to end

- [x] 4.1 Add two checks to `scripts/headless-check.sh`: `]` then `l` moves four
      steps, and `[` then `l` moves a tenth of one
- [x] 4.2 Run the full gate — `cargo test`, `cargo deny check advisories bans`,
      `sh scripts/headless-check.sh sway` — and report each result
