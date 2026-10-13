# Spec Delta

## ADDED Requirements

### Requirement: One-shot session

With `-o` / `--oneshot`, a session started in grid or hint mode SHALL exit when
that mode hands over, instead of switching to normal mode: the work of the
hand-over SHALL still happen first, and the exit SHALL remove the overlay and
return the keyboard exactly as any other exit does. `--oneshot` SHALL be refused
when combined with normal mode, which never hands over.

#### Scenario: A one-shot hint selection

- **WHEN** the user runs `windhover --hint --oneshot` and types the characters of
  a label
- **THEN** the pointer is warped to that target, the click is injected first if
  `click_after_warp` is on, and the session then exits with status 0
- **AND** no overlay is left mapped

#### Scenario: A one-shot grid warp

- **WHEN** the user runs `windhover --grid --oneshot` and warps to the region
- **THEN** the pointer is warped, and the session then exits with status 0

#### Scenario: One-shot with normal mode

- **WHEN** the user runs `windhover --normal --oneshot`
- **THEN** no session is started, and one message naming the reason is written to
  standard error

#### Scenario: Without the flag

- **WHEN** the user runs `windhover --hint` and types the characters of a label
- **THEN** the session warps, switches to normal mode, and stays alive, and one
  Escape exits it
