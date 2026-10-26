#!/bin/sh
# Headless two-output check: an overlay that is not on the layout's origin.
#
# The virtual pointer addresses the compositor's layout, and an overlay-local
# point has to be moved to where its output sits. With one output at (0, 0) that
# translation is invisible, which is why the main suite cannot see a bug in it:
# this puts both outputs away from the origin (at x = 1280 and x = 2560), so
# "jump to the left edge" of the overlay reaches the overlay's own output only if
# that output's position was added. Without the translation the pointer goes to
# layout x = 0, where no output is, and the probe — which sits on the overlay's
# output — is told nothing.
#
# sway only: it is the one compositor here that can place a headless output.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
RUNTIME=$(mktemp -d)
PIDS=""
FAILURES=0

export XDG_RUNTIME_DIR="$RUNTIME"
export WLR_BACKENDS=headless
export WLR_RENDERER=pixman
export WLR_LIBINPUT_NO_DEVICES=1
# Two outputs, both at the origin until the config below moves one.
export WLR_HEADLESS_OUTPUTS=2

cleanup() {
  for pid in $PIDS; do
    kill "$pid" 2>/dev/null || true
  done
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

# HEADLESS-1 is the output sway focuses, so the overlay (a layer surface with no
# output of its own) lands on it — away from the layout's origin.
printf 'output * bg #112233 solid_color\n' >"$RUNTIME/sway.conf"
sway -c "$RUNTIME/sway.conf" >>"$RUNTIME/harness.log" 2>&1 &
PIDS="$PIDS $!"

SWAYSOCK=""
for _ in $(seq 1 80); do
  SWAYSOCK=$(ls "$RUNTIME"/sway-ipc.* 2>/dev/null | head -1) || true
  if [ -n "${SWAYSOCK:-}" ] && swaymsg -t get_version >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done
if [ -z "${SWAYSOCK:-}" ]; then
  echo "sway did not come up:"
  cat "$RUNTIME/harness.log"
  exit 1
fi
export SWAYSOCK
SOCKET=$(ls "$RUNTIME"/wayland-* 2>/dev/null | grep -v '\.lock$' | head -1)
WAYLAND_DISPLAY=$(basename "$SOCKET")
export WAYLAND_DISPLAY

echo "check: an overlay on an output away from the layout's origin"
# Whichever output sway hands the overlay to, its origin is not the layout's.
swaymsg output HEADLESS-1 position 1280 0 >/dev/null
swaymsg output HEADLESS-2 position 2560 0 >/dev/null
sleep 0.5

# The seat has no keyboard until one attaches, and a session refuses to start
# without one, so the injector is started first and keeps the device alive.
"$INJECTOR" --linger 8000 "sleep:1500" "mods:shift" "tap:h" "mods:" \
  >"$RUNTIME/keys.out" 2>&1 &
PIDS="$PIDS $!"
sleep 0.8

"$PROBE" --layer bottom --min-lifetime 3000 --settle 900 \
  >"$RUNTIME/probe.out" 2>&1 &
PROBE_PID=$!
"$WINDHOVER" --normal >"$RUNTIME/session.out" 2>&1 &
SESSION_PID=$!
PIDS="$PIDS $SESSION_PID"

wait "$PROBE_PID" 2>/dev/null || true
sleep 0.3
kill "$SESSION_PID" 2>/dev/null || true

# shellcheck disable=SC2046
set -- $(cat "$RUNTIME/probe.out")
GOT_X=${1:-}
GOT_Y=${2:-}
WIDTH=${3:-0}
HEIGHT=${4:-0}

if [ -z "$GOT_X" ]; then
  fail "the probe on the overlay's output saw no pointer at all (probe: $(cat "$RUNTIME/probe.out" | tr '\n' '|'))"
elif [ "$WIDTH" != "1280" ] || [ "$HEIGHT" != "720" ]; then
  fail "the probe is not on one 1280x720 output: it reports ${WIDTH}x${HEIGHT}"
elif awk -v x="$GOT_X" 'BEGIN { exit !(x < 2) }'; then
  pass "the overlay's output is not at the layout's origin, and its left edge is where the pointer landed ($GOT_X $GOT_Y)"
else
  fail "the pointer is not at the overlay's left edge ($GOT_X $GOT_Y in the probe); the overlay's origin was not added"
fi

if [ "$FAILURES" -ne 0 ]; then
  echo "the layout translation is wrong; the session said: $(cat "$RUNTIME/session.out")"
  exit 1
fi
echo
echo "all checks passed"
