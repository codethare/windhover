#!/bin/sh
# Headless end-to-end check for windhover.
#
# Brings up sway on the headless wlroots backend and drives windhover through a
# virtual keyboard. A headless compositor has no input devices, so its seat
# advertises no keyboard until a `zwp_virtual_keyboard_v1` client attaches one;
# that same fact is the first check below.
#
# Usage: scripts/headless-check.sh [sway|river]
#
# Builds windhover and the helper examples, then needs the compositor.
# sway also needs swaymsg. river needs a river-window-management-v1 window
# manager; point TAILRACE at one (defaults to the usual local checkout).
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
RUNTIME=$(mktemp -d)
FAILURES=0
PIDS=""

export XDG_RUNTIME_DIR="$RUNTIME"
export WLR_BACKENDS=headless
export WLR_RENDERER=pixman
export WLR_LIBINPUT_NO_DEVICES=1
export WLR_HEADLESS_OUTPUTS=1

# Never `pkill -f`: a pattern that also appears in this script's own command
# line kills the check itself. Track PIDs instead.
cleanup() {
  for pid in $PIDS; do
    kill "$pid" 2>/dev/null || true
  done
  exec 3>&- 2>/dev/null || true
  rm -rf "$RUNTIME"
}
trap cleanup EXIT INT TERM

pass() { echo "  ok   $1"; }
fail() {
  echo "  FAIL $1"
  FAILURES=$((FAILURES + 1))
}

echo "building"
cargo build --quiet --manifest-path "$ROOT/Cargo.toml" \
  --bin windhover --example virtual_keyboard --example cursor_probe
WINDHOVER="$ROOT/target/debug/windhover"
INJECTOR="$ROOT/target/debug/examples/virtual_keyboard"
PROBE="$ROOT/target/debug/examples/cursor_probe"

COMPOSITOR=${1:-sway}
TAILRACE=${TAILRACE:-/mnt/projects/workspace/tailrace/target/release/tailrace}

case "$COMPOSITOR" in
  sway)
    printf 'output * bg #112233 solid_color\n' >"$RUNTIME/sway.conf"
    sway -c "$RUNTIME/sway.conf" >>"$RUNTIME/harness.log" 2>&1 &
    PIDS="$PIDS $!"
    ;;
  river)
    if [ ! -x "$TAILRACE" ]; then
      echo "river needs a window manager; set TAILRACE to one (looked at $TAILRACE)"
      exit 1
    fi
    # river is only the compositor; the window management policy lives in the
    # window manager given to -c.
    river -c "$TAILRACE" >>"$RUNTIME/harness.log" 2>&1 &
    PIDS="$PIDS $!"
    ;;
  niri)
    # niri has no headless backend of its own, so it is run nested inside a
    # headless sway; it picks the winit backend and puts up a window there.
    printf 'output * bg #112233 solid_color\n' >"$RUNTIME/sway.conf"
    sway -c "$RUNTIME/sway.conf" >>"$RUNTIME/harness.log" 2>&1 &
    HOST_PID=$!
    PIDS="$PIDS $HOST_PID"
    for _ in $(seq 1 80); do
      HOST_SOCK=$(ls "$RUNTIME"/wayland-* 2>/dev/null | grep -v '\.lock$' | head -1) || true
      [ -n "${HOST_SOCK:-}" ] && break
      sleep 0.1
    done
    [ -n "${HOST_SOCK:-}" ] || { echo "the host compositor did not come up"; cat "$RUNTIME/harness.log"; exit 1; }
    HOST_DISPLAY=$(basename "$HOST_SOCK")
    mkdir -p "$RUNTIME/niri"
    printf 'output "winit" {\n    mode "1280x720"\n}\n' >"$RUNTIME/niri/config.kdl"
    WAYLAND_DISPLAY="$HOST_DISPLAY" LIBGL_ALWAYS_SOFTWARE=1 \
      niri -c "$RUNTIME/niri/config.kdl" >>"$RUNTIME/harness.log" 2>&1 &
    NIRI_PID=$!
    PIDS="$PIDS $NIRI_PID"
    # niri opens its own socket, so wait for a display that is not the host's.
    NIRI_DISPLAY=""
    for _ in $(seq 1 120); do
      for socket in "$RUNTIME"/wayland-*; do
        case "$socket" in *.lock) continue ;; esac
        candidate=$(basename "$socket")
        if [ "$candidate" != "$HOST_DISPLAY" ]; then
          NIRI_DISPLAY=$candidate
        fi
      done
      if [ -n "$NIRI_DISPLAY" ] && kill -0 "$NIRI_PID" 2>/dev/null; then
        sleep 2
        break
      fi
      sleep 0.1
    done
    if [ -z "${NIRI_DISPLAY:-}" ] || ! kill -0 "$NIRI_PID" 2>/dev/null; then
      echo "niri did not come up:"
      tail -30 "$RUNTIME/harness.log"
      exit 1
    fi
    ;;

esac

if [ "$COMPOSITOR" = "niri" ]; then
  WAYLAND_DISPLAY=$NIRI_DISPLAY
  export WAYLAND_DISPLAY
  echo "niri up (nested in sway): WAYLAND_DISPLAY=$WAYLAND_DISPLAY"
fi

for _ in $(seq 1 80); do
  SOCKET=$(ls "$RUNTIME"/wayland-* 2>/dev/null | grep -v '\.lock$' | head -1) || true
  if [ -n "${SOCKET:-}" ] && [ "$COMPOSITOR" != "niri" ]; then
    WAYLAND_DISPLAY=$(basename "$SOCKET")
    if [ "$COMPOSITOR" = "sway" ]; then
      SWAYSOCK=$(ls "$RUNTIME"/sway-ipc.* 2>/dev/null | head -1) || true
      if [ -n "${SWAYSOCK:-}" ] && { swaymsg -t get_version >/dev/null 2>&1 || true; }; then
        export SWAYSOCK
        break
      fi
    else
      # river has no IPC socket to poll; give the window manager time to attach.
      sleep 2
      break
    fi
  elif [ "$COMPOSITOR" = "niri" ]; then
    break
  fi
  sleep 0.1
done
if [ -z "${WAYLAND_DISPLAY:-}" ]; then
  echo "$COMPOSITOR did not come up:"
  cat "$RUNTIME/harness.log"
  exit 1
fi
export WAYLAND_DISPLAY
echo "$COMPOSITOR up: WAYLAND_DISPLAY=$WAYLAND_DISPLAY"

# Starts a session, waits for it to settle, prints its pid.
session_start() {
  "$WINDHOVER" "$@" >"$RUNTIME/session.out" 2>&1 &
  SESSION_PID=$!
  sleep 1
}

session_alive() { kill -0 "$SESSION_PID" 2>/dev/null; }

# Waits for the current session to end and returns its status.
session_status() {
  STATUS=0
  wait "$SESSION_PID" 2>/dev/null || STATUS=$?
  echo "$STATUS"
}

echo "check: a seat with no keyboard is refused rather than half-started"
if [ "$COMPOSITOR" = "niri" ]; then
  # niri advertises a keyboard on its seat whether or not one is attached, so
  # this premise only exists on the wlroots compositors.
  echo "  skip  niri always advertises a keyboard capability"
elif "$WINDHOVER" --grid >"$RUNTIME/nokbd.out" 2>&1; then
  fail "expected a non-zero exit when the seat has no keyboard"
elif grep -q keyboard "$RUNTIME/nokbd.out"; then
  pass "names the missing capability — $(head -1 "$RUNTIME/nokbd.out")"
else
  fail "error does not name the missing capability: $(cat "$RUNTIME/nokbd.out")"
fi

# One long-lived virtual keyboard for the whole run: the seat loses its keyboard
# capability the moment the last one disconnects. Commands arrive over a FIFO.
FIFO="$RUNTIME/inject"
mkfifo "$FIFO"
"$INJECTOR" --stdin <"$FIFO" >/dev/null 2>&1 &
PIDS="$PIDS $!"
exec 3>"$FIFO"
sleep 0.5

inject() {
  printf '%s\n' "$*" >&3
  sleep "${SETTLE:-0.35}"
}

echo "check: session starts, ignores unrelated keys, exits 0 on Escape"
session_start --normal
if session_alive; then
  pass "session is running"
else
  fail "session exited immediately: $(cat "$RUNTIME/session.out")"
fi

inject "tap:h tap:5"
if session_alive; then
  pass "unrelated keys do not end the session"
else
  fail "session ended on an unrelated key"
fi

inject "tap:Escape"
sleep 0.4
if session_alive; then
  fail "Escape did not end the session"
  kill "$SESSION_PID" 2>/dev/null || true
else
  STATUS=$(session_status)
  if [ "$STATUS" = "0" ]; then
    pass "Escape exits with status 0"
  else
    fail "Escape exits with status $STATUS"
  fi
fi

echo "check: a session started while the hotkey modifiers are held still works"
inject "down:Super_L down:Alt_L"
session_start --normal
if session_alive; then
  pass "session started with Super and Alt already down"
else
  fail "session refused to start with modifiers held: $(cat "$RUNTIME/session.out")"
fi
inject "tap:h"
if session_alive; then
  pass "keys still resolve while the modifiers are held"
else
  fail "session ended while the hotkey modifiers were held"
fi
inject "up:Super_L up:Alt_L"
if session_alive; then
  pass "releasing the hotkey modifiers is not an action"
else
  fail "releasing the hotkey modifiers ended the session"
fi
inject "tap:Escape"
sleep 0.4
if session_alive; then
  fail "Escape did not end the modifier session"
  kill "$SESSION_PID" 2>/dev/null || true
else
  pass "session exits once the modifiers are back up"
fi

echo "check: SIGTERM tears the session down and leaves nothing blocking"
session_start --normal
if session_alive; then
  kill -TERM "$SESSION_PID"
  sleep 0.6
  if session_alive; then
    fail "session ignored SIGTERM"
    kill -KILL "$SESSION_PID" 2>/dev/null || true
  else
    pass "SIGTERM ends the session"
  fi
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

session_start --normal
if session_alive; then
  pass "a later session still starts (no leftover overlay or grab)"
  inject "tap:Escape"
  sleep 0.4
  if session_alive; then
    fail "the later session would not exit"
    kill "$SESSION_PID" 2>/dev/null || true
  else
    STATUS=$(session_status)
    if [ "$STATUS" = "0" ]; then
      pass "and it exits cleanly"
    else
      fail "later session exited with status $STATUS"
    fi
  fi
else
  fail "a later session could not start: $(cat "$RUNTIME/session.out")"
fi

if [ -s "$RUNTIME/session.out" ]; then
  fail "session wrote to stderr: $(cat "$RUNTIME/session.out")"
else
  pass "sessions stayed quiet"
fi

echo "check: the cursor actually lands where each mode means it to"
probe_start() {
  "$PROBE" --timeout 4000 ${1:-} >"$RUNTIME/probe.out" 2>&1 &
  PROBE_PID=$!
  sleep 0.6
}

probe_wait() {
  wait "$PROBE_PID" 2>/dev/null || true
}

close_enough() {
  awk -v a="$1" -v b="$2" 'BEGIN { d = a - b; if (d < 0) d = -d; exit !(d < 2) }'
}

# Evaluates an expectation expression with `W`, `H` (the output's logical size)
# and `s` (the normal-mode step) in scope, so the checks do not hard-code a
# resolution and can run against niri's nested output too.
want() {
  # `-` means "do not care about this axis".
  [ "$3" = "-" ] && return 0
  awk -v W="$1" -v H="$2" "BEGIN { s = H / 72; if (s < 4) s = 4; print ($3) }"
}

# cursor_check <name> <mode flag> <keys> <x expression|-> <y expression|->
#
# `H` pins x to 0 (Shift+h, since plain h moves), and grid and hint position
# absolutely by construction, so every expectation below is independent of where
# the session started.
cursor_check() {
  NAME=$1
  FLAG=$2
  KEYS=$3
  WANT_X_EXPR=$4
  WANT_Y_EXPR=$5
  EXTRA=${6:-}

  session_start "$FLAG"
  if ! session_alive; then
    fail "$NAME: session did not start: $(cat "$RUNTIME/session.out")"
    return
  fi
  # The probe has to be up before the keys are sent: sway does not re-run
  # pointer focus for a freshly mapped surface, so it only reports motion that
  # happens while it is up.
  probe_start "$EXTRA"
  inject "$KEYS"
  probe_wait

  # shellcheck disable=SC2046
  set -- $(cat "$RUNTIME/probe.out")
  GOT_X=$1
  GOT_Y=$2
  W=${3:-0}
  H=${4:-0}
  if [ -z "$GOT_Y" ] || [ "$W" = "0" ]; then
    fail "$NAME: the probe saw no pointer at all (is the mode flag right?)"
  else
    EXPECT_X=$(want "$W" "$H" "${WANT_X_EXPR}")
    EXPECT_Y=$(want "$W" "$H" "${WANT_Y_EXPR}")
    if { [ "$WANT_X_EXPR" = "-" ] || close_enough "$GOT_X" "$EXPECT_X"; } &&
       { [ "$WANT_Y_EXPR" = "-" ] || close_enough "$GOT_Y" "$EXPECT_Y"; }; then
      pass "$NAME lands at $GOT_X $GOT_Y (wanted ${EXPECT_X} ${EXPECT_Y} on ${W}x${H})"
    else
      fail "$NAME landed at $GOT_X $GOT_Y, wanted ${EXPECT_X} ${EXPECT_Y} on ${W}x${H}"
    fi
  fi

  inject "tap:Escape"
  sleep 0.4
  if session_alive; then
    fail "$NAME: Escape did not end the session"
    kill "$SESSION_PID" 2>/dev/null || true
  else
    STATUS=$(session_status)
    [ "$STATUS" = "0" ] || fail "$NAME: exit status $STATUS"
  fi
}

cursor_check "normal: jump to the left edge" --normal \
  "mods:shift tap:h mods:" 0 -
cursor_check "normal: three steps right of the left edge" --normal \
  "mods:shift tap:h mods: tap:l tap:l tap:l" "3*s" -
cursor_check "normal: a 10x prefix" --normal \
  "mods:shift tap:h mods: tap:1 tap:0 tap:l" "10*s" -
# The modifier only reaches the session through the compositor's own modifier
# events, so this is the check that the fast step is wired up.
cursor_check "normal: the fast modifier doubles the step" --normal \
  "mods:shift tap:h mods: mods:ctrl tap:l mods:" "2*s" -
# A preset scales the base step rather than replacing it, and it outlives the
# press that selected it, so the move after the selector is the one that shows it.
cursor_check "normal: the coarse preset multiplies the step" --normal \
  "mods:shift tap:h mods: tap:bracketright tap:l" "4*s" -
cursor_check "normal: the fine preset divides the step" --normal \
  "mods:shift tap:h mods: tap:bracketleft tap:l" "s/10" -
# `W - 1`: the session addresses the last pixel column, not one past it.
cursor_check "normal: jump to the right edge" --normal \
  "mods:shift tap:l mods:" "W-1" -
cursor_check "grid: a single quadrant" --grid "tap:s" "W*3/4" "H*3/4"
cursor_check "grid: two levels down" --grid "tap:w tap:a" "W*5/8" "H*3/8"
cursor_check "grid: warp to the region centre" --grid "tap:w tap:Return" "W*3/4" "H/4"
# `space` warps, clicks through the suspended overlay and returns to normal
# mode; the position proves the batch survived taking the overlay down.
cursor_check "grid: warp and click through" --grid "tap:w tap:space" "W*3/4" "H/4"
# The default lattice fills the output with as many cells as the alphabet can
# name in two characters, so the spacing is `sqrt(W*H/400)` and the first label
# (`aa`) sits at the centre of the first cell.
cursor_check "hint: the first label" --hint "tap:a tap:a" \
  "(W/(int(W/sqrt(W*H/400))>0?int(W/sqrt(W*H/400)):1))*0.5" \
  "(H/(int(H/sqrt(W*H/400))>0?int(H/sqrt(W*H/400)):1))*0.5"
# `H` is bound in normal mode only, so the cursor landing on the left edge is
# what proves the selection handed over to normal mode instead of staying in the
# mode it was made in. The same key proves it for a grid warp.
cursor_check "hint: a selected label lands in normal mode" --hint \
  "tap:a tap:a mods:shift tap:h mods:" 0 -
cursor_check "grid: a warp lands in normal mode" --grid \
  "tap:w tap:Return mods:shift tap:h mods:" 0 -

echo "check: one session carries several mode round trips"
# The workflow the README documents: a single session, with `x` and `g` entering
# the other modes from inside it. What this checks is that the session survives
# the hand-overs, because a session left alive per activation was what put a
# cursor ring at every place the user had been.
session_start --normal
if session_alive; then
  probe_start "--min-lifetime 3000 --settle 600"
  inject "tap:f tap:a tap:a"        # hint, first label
  inject "tap:f tap:a tap:s"        # hint again, same session
  inject "tap:g tap:w tap:Return"   # grid warp, same session
  # `H` is bound in normal mode alone, so landing on the left edge proves the
  # session came back to normal mode and is still taking keys.
  inject "mods:shift tap:h mods:"
  probe_wait
  X=$(head -1 "$RUNTIME/probe.out" | cut -d' ' -f1)
  if ! session_alive; then
    fail "the session did not survive the mode round trips"
  elif close_enough "${X:-999}" 0; then
    pass "one session took hint, hint again and grid, and still answers in normal mode"
  else
    fail "normal mode after three hand-overs moved to x=$X, wanted 0 (probe: $(cat "$RUNTIME/probe.out" | tr '\n' '|'))"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: a click reaches what is under the overlay"
# The probe is put on the *bottom* layer, so it sits below the overlay: the
# click can only reach it if the session really did take its overlay down first.
session_start --grid
if session_alive; then
  probe_start "--layer bottom"
  inject "tap:w tap:space"
  probe_wait
  # shellcheck disable=SC2046
  set -- $(cat "$RUNTIME/probe.out")
  W=${3:-0}
  H=${4:-0}
  BUTTON=$(grep '^click ' "$RUNTIME/probe.out" | head -1 | cut -d' ' -f2)
  CLICK_X=$(grep '^click ' "$RUNTIME/probe.out" | head -1 | cut -d' ' -f3)
  CLICK_Y=$(grep '^click ' "$RUNTIME/probe.out" | head -1 | cut -d' ' -f4)
  if [ -z "$BUTTON" ]; then
    fail "the click never arrived below the overlay (probe output: $(cat "$RUNTIME/probe.out" | tr '\n' '|'))"
  elif [ "$BUTTON" != "272" ]; then
    fail "the click arrived as button $BUTTON, expected 272 (BTN_LEFT)"
  elif close_enough "$CLICK_X" "$(want "$W" "$H" "W*3/4")" &&
       close_enough "$CLICK_Y" "$(want "$W" "$H" "H/4")"; then
    pass "the left button reached the layer under the overlay at $CLICK_X $CLICK_Y"
  else
    fail "the click landed at $CLICK_X $CLICK_Y, wanted $(want "$W" "$H" "W*3/4") $(want "$W" "$H" "H/4")"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: every mouse button works from normal mode"
# The click-through check above uses grid mode's warp-and-click, so the three
# normal-mode button keys need their own check: 272 is BTN_LEFT, 273 BTN_RIGHT
# and 274 BTN_MIDDLE.
session_start --normal
if session_alive; then
  probe_start "--layer bottom"
  inject "tap:l tap:m tap:comma tap:period"
  probe_wait
  BUTTONS=$(grep '^click ' "$RUNTIME/probe.out" | cut -d' ' -f2 | tr '\n' ' ')
  if [ "$BUTTONS" = "272 274 273 " ]; then
    pass "m, , and . sent BTN_LEFT, BTN_MIDDLE and BTN_RIGHT"
  else
    fail "the button keys sent '$BUTTONS', expected '272 274 273 ' (probe output: $(cat "$RUNTIME/probe.out" | tr '\n' '|'))"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: hovering reaches what is under the overlay"
# An application only sees the pointer move if the overlay does not take pointer
# events, and hover-dependent UI — a video's controls, a tooltip, a menu — needs
# exactly that. A session that keeps the pointer to itself makes the cursor a
# thing that can click but cannot hover.
session_start --normal
if session_alive; then
  "$PROBE" --toplevel --min-lifetime 3000 --settle 900 >"$RUNTIME/hover.out" 2>&1 &
  HOVER_PID=$!
  sleep 1.2
  inject "mods:shift tap:h mods: tap:l tap:l tap:l tap:j tap:j"
  HOVER_MISSED=""
  wait "$HOVER_PID" 2>/dev/null || HOVER_MISSED=1
  HOVER_X=$(head -1 "$RUNTIME/hover.out" | cut -d' ' -f1)
  if [ -n "$HOVER_MISSED" ]; then
    fail "the window saw no pointer at all (probe output: $(cat "$RUNTIME/hover.out" | tr '\n' '|'))"
  elif awk -v x="${HOVER_X:-0}" 'BEGIN { exit !(x > 0) }'; then
    pass "the window saw the pointer move to x=$HOVER_X"
  else
    fail "the window saw the pointer at x=$HOVER_X, where it was not moved"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: a click reaches an ordinary window, not only a layer surface"
# A real xdg-shell toplevel. Layer surfaces live in the overlay stack, so the
# check above is not evidence that a click reaches an application window.
session_start --normal
if session_alive; then
  "$PROBE" --toplevel --min-lifetime 2500 --settle 900 >"$RUNTIME/toplevel.out" 2>&1 &
  TOPLEVEL_PID=$!
  # Long enough for the window to be mapped and tiled: the compositor sends a
  # `0x0` configure first, then the size it actually wants.
  sleep 1.2
  # Off the left edge and clear of the title bar: a click on a border or a title
  # bar belongs to the window manager rather than the window, and on river the
  # window is a tiled column rather than the whole output.
  inject "mods:shift tap:h mods: tap:l tap:l tap:l tap:j tap:j tap:m"
  wait "$TOPLEVEL_PID" 2>/dev/null || true
  if grep -q '^click 272 ' "$RUNTIME/toplevel.out"; then
    pass "the left click reached the window"
  else
    fail "the left click did not reach the window (probe output: $(cat "$RUNTIME/toplevel.out" | tr '\n' '|'))"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: a drag reaches what is under the overlay"
# `v` holds the left button down while the direction keys drag the pointer, which
# is how a text selection is made. The press and every motion after it have to
# reach the window, or the drag selects nothing.
session_start --normal
if session_alive; then
  "$PROBE" --toplevel --min-lifetime 3000 --settle 900 >"$RUNTIME/drag.out" 2>&1 &
  DRAG_PID=$!
  sleep 1.2
  inject "mods:shift tap:h mods: tap:l tap:l tap:l tap:j tap:j"
  inject "tap:v"
  inject "tap:l tap:l tap:l"
  inject "tap:v"
  wait "$DRAG_PID" 2>/dev/null || true
  # Both are positions inside the window, so their difference is the ground the
  # drag covered, with no assumption about where the compositor put the window.
  DRAG_END_X=$(head -1 "$RUNTIME/drag.out" | cut -d' ' -f1)
  DRAG_PRESS_X=$(grep '^click 272 ' "$RUNTIME/drag.out" | head -1 | cut -d' ' -f3)
  if [ -z "$DRAG_PRESS_X" ]; then
    fail "the drag never reached the window (probe output: $(cat "$RUNTIME/drag.out" | tr '\n' '|'))"
  elif awk -v end="${DRAG_END_X:-0}" -v press="$DRAG_PRESS_X" 'BEGIN { exit !(end <= press) }'; then
    fail "the press reached the window at x=$DRAG_PRESS_X but the drag moved nothing (ended at x=$DRAG_END_X)"
  else
    pass "the drag pressed at $DRAG_PRESS_X and dragged the pointer to $DRAG_END_X in the window"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: the pointer stays with the application after a drag"
# The drag used to take the pointer back when it ended, which is what made a
# window see nothing move unless a click was in flight. It stays with the
# application now, so a move after a drag is seen like any other motion.
session_start --normal
if session_alive; then
  "$PROBE" --toplevel --min-lifetime 2500 --settle 900 >"$RUNTIME/after-drag.out" 2>&1 &
  AFTER_DRAG_PID=$!
  sleep 1.2
  inject "mods:shift tap:h mods: tap:l tap:l tap:l tap:j tap:j"
  inject "tap:v tap:v"
  # A relative move, not a jump to an edge: on a compositor that tiles the window
  # into a column, an edge of the output is an edge of nothing.
  inject "tap:l tap:l"
  wait "$AFTER_DRAG_PID" 2>/dev/null || true
  AFTER_X=$(head -1 "$RUNTIME/after-drag.out" | cut -d' ' -f1)
  AFTER_PRESS_X=$(grep '^click 272 ' "$RUNTIME/after-drag.out" | head -1 | cut -d' ' -f3)
  if [ -z "$AFTER_PRESS_X" ]; then
    fail "the window saw no press at all (probe output: $(cat "$RUNTIME/after-drag.out" | tr '\n' '|'))"
  elif awk -v end="${AFTER_X:-0}" -v press="$AFTER_PRESS_X" 'BEGIN { exit !(end > press) }'; then
    pass "the window saw the pointer move after the drag (from $AFTER_PRESS_X to $AFTER_X)"
  else
    fail "the window did not see the pointer move after a drag (press at $AFTER_PRESS_X, ended at $AFTER_X)"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: scrolling reaches what is under the overlay"
# The probe on the bottom layer takes the wheel like any other window: the
# overlay does not take pointer events. It has to be moved onto before it can
# report anything, so the pointer is moved first.
session_start --normal
if session_alive; then
  probe_start "--layer bottom"
  inject "tap:l tap:d"
  probe_wait
  AXIS=$(grep '^axis ' "$RUNTIME/probe.out" | cut -d' ' -f2)
  VALUE=$(grep '^axis ' "$RUNTIME/probe.out" | cut -d' ' -f3)
  if [ -z "$AXIS" ]; then
    fail "the scroll never arrived below the overlay (probe output: $(cat "$RUNTIME/probe.out" | tr '\n' '|'))"
  elif [ "$AXIS" != "0" ]; then
    fail "the scroll arrived on axis $AXIS, expected 0 (vertical)"
  elif close_enough "$VALUE" 15; then
    pass "the wheel reached the layer under the overlay"
  else
    fail "the scroll arrived as $VALUE, expected 15"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: a binding the layout cannot produce is reported"
# `eacute` is a real keysym, so the configuration accepts it, but a layout
# without a key for it can never produce it. The binding that ends the session is
# the one that matters here: without a word about it, a session nothing can leave
# looks like a working one.
printf 'scroll_up: eacute\nquit: F13\n' >"$RUNTIME/dead-keys"
"$WINDHOVER" --config "$RUNTIME/dead-keys" --normal >"$RUNTIME/session.out" 2>&1 &
SESSION_PID=$!
sleep 1
if session_alive; then
  if grep -q "cannot produce" "$RUNTIME/session.out"; then
    pass "the session names the bindings that cannot be pressed — $(head -1 "$RUNTIME/session.out")"
  else
    fail "the session said nothing about the bindings: $(cat "$RUNTIME/session.out")"
  fi
  kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: --trace prints what the session saw"
# The failures that matter are the ones the client never sees, so a report of one
# has to come with the session's own account of it.
session_start --normal --trace
if session_alive; then
  inject "tap:l"
  sleep 0.4
  inject "tap:Escape"
  sleep 0.6
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
  if ! grep -q "^TRACE key XK_l down" "$RUNTIME/session.out"; then
    fail "the trace does not show the key: $(cat "$RUNTIME/session.out")"
  elif ! grep -q "^TRACE focus enter" "$RUNTIME/session.out"; then
    fail "the trace does not show the keyboard focus: $(cat "$RUNTIME/session.out")"
  elif ! grep -q "^TRACE turn " "$RUNTIME/session.out"; then
    fail "the trace does not show the event loop: $(cat "$RUNTIME/session.out")"
  else
    pass "the trace shows the focus, the keys and the loop"
  fi
else
  fail "session did not start: $(cat "$RUNTIME/session.out")"
fi

echo "check: a one-shot session ends itself"
# The workflow the README documents is one session per hotkey press. Without
# --oneshot every one of them stays alive in normal mode after the label is
# activated, so the screen collects their overlays: one cursor ring each, and one
# Escape per leftover session. Twice in a row is what a hotkey does.
oneshot_check() {
  NAME=$1
  KEYS=$2
  "$WINDHOVER" --hint --oneshot >"$RUNTIME/oneshot.out" 2>&1 &
  ONESHOT_PID=$!
  sleep 1.2
  inject "$KEYS"
  sleep 1
  if kill -0 "$ONESHOT_PID" 2>/dev/null; then
    kill "$ONESHOT_PID" 2>/dev/null || true
    fail "$NAME: still running after the label was activated"
    return
  fi
  STATUS=0
  wait "$ONESHOT_PID" 2>/dev/null || STATUS=$?
  if [ "$STATUS" = "0" ]; then
    pass "$NAME"
  else
    fail "$NAME: exit status $STATUS"
  fi
}
oneshot_check "a one-shot hint session ends itself" "tap:a tap:a"
oneshot_check "and the next one leaves nothing behind" "tap:a tap:s"

echo "check: a hint lattice too dense to label is refused"
# Twenty characters name 8000 cells in three characters; ten-pixel cells on this
# output are 9216. A session that dropped targets or repeated labels instead
# would look like it worked, so the refusal is what is being checked, not the
# exit status on its own.
printf 'hint_cell: 10\n' >"$RUNTIME/dense-hint"
DENSE_STATUS=0
timeout 5 "$WINDHOVER" --config "$RUNTIME/dense-hint" --hint >"$RUNTIME/dense-hint.out" 2>&1 || DENSE_STATUS=$?
if [ "$DENSE_STATUS" = "124" ]; then
  fail "a lattice needing four characters started a session and kept running"
  pkill -f "$RUNTIME/dense-hint" 2>/dev/null || true
elif grep -q "hint_cell" "$RUNTIME/dense-hint.out"; then
  pass "a lattice too dense to label is refused — $(head -1 "$RUNTIME/dense-hint.out")"
else
  fail "the refusal does not name the option: $(cat "$RUNTIME/dense-hint.out")"
fi

echo "check: a mode that never hands over is refused"
if "$WINDHOVER" --normal --oneshot >"$RUNTIME/oneshot-bad.out" 2>&1; then
  fail "\`--normal --oneshot\` started a session"
elif grep -q -- "--oneshot" "$RUNTIME/oneshot-bad.out"; then
  pass "one-shot with normal mode is refused — $(head -1 "$RUNTIME/oneshot-bad.out")"
else
  fail "the refusal does not name the option: $(cat "$RUNTIME/oneshot-bad.out")"
fi

echo "check: a release that never arrives does not repeat forever"
# A compositor can swallow a key's release without saying anything — river does
# when a drag turns into a drag-and-drop — and then the repeat would travel
# forever with the button still down and no key able to stop it. Nothing tells
# that apart from a user really holding the key, so the hold is bounded.
session_start --normal
if session_alive; then
  inject "mods:shift tap:h mods:"
  inject "down:l"
  sleep 7
  if session_alive; then
    fail "the session kept repeating a key whose release never arrived"
    kill "$SESSION_PID" 2>/dev/null || true
  elif grep -q "release never arrived" "$RUNTIME/session.out"; then
    pass "the session ended itself — $(head -1 "$RUNTIME/session.out")"
  else
    fail "the session ended without saying why: $(cat "$RUNTIME/session.out")"
  fi
  # The key is still down as far as the compositor is concerned, and a session
  # started later would seed it from `Enter` and then ignore that key.
  inject "up:l"
fi

echo "check: a session that loses the keyboard ends itself"
# The seat can take the keyboard away from a session: another layer surface
# asking for it exclusively is enough. No key reaches the session from then on,
# so a held key would keep moving the pointer forever, a drag would keep a window
# dragging, and there would be no Escape to leave by: only a signal would end it.
if [ "$COMPOSITOR" = "niri" ]; then
  echo "  skip  niri keeps the keyboard with the session that holds it"
else
  session_start --normal
  if session_alive; then
    "$WINDHOVER" --normal >"$RUNTIME/usurper.out" 2>&1 &
    USURPER_PID=$!
    PIDS="$PIDS $USURPER_PID"
    sleep 2
    if session_alive; then
      fail "the session kept going after the keyboard was taken away"
      kill "$SESSION_PID" 2>/dev/null || true
    elif grep -q "took the keyboard away" "$RUNTIME/session.out"; then
      pass "the session ended itself — $(head -1 "$RUNTIME/session.out")"
    else
      fail "the session ended without saying why: $(cat "$RUNTIME/session.out")"
    fi
    kill "$USURPER_PID" 2>/dev/null || true
  else
    fail "session did not start: $(cat "$RUNTIME/session.out")"
  fi
fi

echo "check: configuration"
# session_start_config <config path> <mode flag>
session_start_config() {
  "$WINDHOVER" --config "$1" "$2" >"$RUNTIME/session.out" 2>&1 &
  SESSION_PID=$!
  sleep 1
}

printf 'step: 50\nnormal_right: n\n' >"$RUNTIME/config"
session_start_config "$RUNTIME/config" --normal
if session_alive; then
  probe_start
  inject "mods:shift tap:h mods: tap:n tap:n"
  probe_wait
  GOT=$(cat "$RUNTIME/probe.out")
  # Pin x with `H`, then two steps of 50 on the rebound key. If either the step
  # or the binding were ignored, `n` would do nothing and x would stay 0.
  if close_enough "${GOT%% *}" 100; then
    pass "a configured step and binding take effect (x = ${GOT%% *})"
  else
    fail "configured step and binding ignored, landed at $GOT"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "session did not start with a config file: $(cat "$RUNTIME/session.out")"
fi

if "$WINDHOVER" --config "$RUNTIME/there-is-no-such-file" --normal \
    >"$RUNTIME/missing.out" 2>&1; then
  fail "a named config file that does not exist should be an error"
elif grep -q "there-is-no-such-file" "$RUNTIME/missing.out"; then
  pass "a named config file that does not exist names the path and fails"
else
  fail "the error does not name the path: $(cat "$RUNTIME/missing.out")"
fi

printf 'nonsense: 3\n' >"$RUNTIME/noisy-config"
session_start_config "$RUNTIME/noisy-config" --normal
if session_alive; then
  pass "an unknown option does not stop the session"
  if grep -q "nonsense: 3" "$RUNTIME/session.out"; then
    pass "the warning quotes the offending line"
  else
    fail "no warning quoted the line: $(cat "$RUNTIME/session.out")"
  fi
  inject "tap:Escape"
  sleep 0.4
  session_alive && kill "$SESSION_PID" 2>/dev/null || true
else
  fail "an unknown option ended the session: $(cat "$RUNTIME/session.out")"
fi


echo
if [ "$FAILURES" -eq 0 ]; then
  echo "all checks passed"
else
  echo "$FAILURES check(s) failed"
fi
exit "$FAILURES"
