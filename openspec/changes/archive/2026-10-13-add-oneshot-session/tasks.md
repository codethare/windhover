# Tasks

## 1. The flag

- [x] 1.1 Parse `-o` / `--oneshot` in `src/main.rs`, refuse it with normal mode, and say so in `--help`. Verify: the argument tests cover reading it, keeping it with the mode, and the refusal.
- [x] 1.2 End the session at the hand-over in `src/session.rs`, in the one place every effect passes through. Grid and hint both hand over through `Effect::Switch(Mode::Normal)`, and normal mode never emits it, so no mode knows about the flag. Verify: `cargo test`.
- [x] 1.3 Keep the hand-over's own work: the warp and the click still happen before the exit. Verify: the unit test asserts the effect order, and the click path is already covered end to end.

## 2. Cover it

- [x] 2.1 Unit tests: a one-shot hint selection ends the session rather than switching, it still warps first, a one-shot grid warp ends it, and a session without the flag still returns to normal mode. Verify: `cargo test`.
- [x] 2.2 End-to-end: a one-shot hint session exits 0 on its own, twice in a row, and `--normal --oneshot` is refused with a message naming the option. Verify: `scripts/headless-check.sh` on sway, river and niri.
- [x] 2.3 The reported symptom, measured: two activations with the flag leave no process and no cursor ring behind, where without it the screen collects one ring per activation. Verify: screenshots counted for accent-coloured marker clusters.

## 3. Records

- [x] 3.1 README: the hotkey snippets use `--oneshot`, with the reason and the alternative single-session workflow next to them. Verify: the snippets match `--help`.
