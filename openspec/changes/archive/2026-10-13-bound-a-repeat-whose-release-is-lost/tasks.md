# Tasks

## 1. Bound the repeat

- [x] 1.1 `src/session.rs` holds the limit and, in `tick`, treats a hold past it as a lost release: stop every repeat, release a drag's button, and end the session with one line on standard error. Verify: `cargo test`.
- [x] 1.2 Choose the limit from what movement actually needs: the pointer crosses the output in under a second at the accelerated rate, so five seconds is far past any real hold. Verify: the unit test holds just under the limit and asserts the repeat still runs, then just over it and asserts the session ends.
- [x] 1.3 Leave a hold within the limit alone, including the wheel, whose release still stops it. Verify: the existing repeat and wheel tests pass unchanged.

## 2. Cover it on every compositor

- [x] 2.1 End-to-end: a `down:` with no `up:` driven through the virtual keyboard, so the check does not depend on which compositor swallows releases. It has to release the key afterwards, or a session started later would seed it from `Enter` and then ignore that key. Verify: `scripts/headless-check.sh` on sway, river and niri.

## 3. The reproduction it came from

- [x] 3.1 Reproduce the reported hang: chromium, select text, then `v` and one direction key. It hangs on river (97% of a core, no Escape) and not on sway or niri. Verify: the river run ends itself after the change; the chromium harness stays out of the suite because it is not what the suite is for and chromium is not a dependency of it.
