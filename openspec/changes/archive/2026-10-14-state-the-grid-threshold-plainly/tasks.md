# Tasks

## 1. The specification

- [x] 1.1 Replace the "twice the normal-mode step" parenthetical with the default it actually has, and say that it is independent of the step so the next reader does not look for the relationship again. Verify: `openspec validate --specs`.

## 2. The audit that found it

- [x] 2.1 Every consumer of the step: `src/mode/normal.rs` multiplies it, `src/session.rs` scales it for a held key, and nothing else reads it. `src/mode/grid.rs` mentions it only in a test fixture and a loop variable, so grid mode cannot see the change. Verify: `grep -n step src/mode/grid.rs`.
- [x] 2.2 Every expectation derived from the step: the end-to-end helper computes it from the output geometry, the unit tests take it from the constant, and the one configuration check that asserts a distance sets its own step. Verify: the whole suite passes on the derived value with no other edit.
- [x] 2.3 The remaining half of the change, the interval between repeats, is back at its original value and its unit tests read the constant, so the two are not entangled. Verify: the repeat tests pass unchanged in form.
