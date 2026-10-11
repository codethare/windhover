#!/bin/sh
# Capture one screenshot per mode, for looking at rather than asserting on.
#
# Usage: sh scripts/capture.sh [output directory]
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:-/tmp/windhover-shots}
RUNTIME=$(mktemp -d)
PIDS=""

export XDG_RUNTIME_DIR="$RUNTIME"
export WLR_BACKENDS=headless
export WLR_RENDERER=pixman
export WLR_LIBINPUT_NO_DEVICES=1
export WLR_HEADLESS_OUTPUTS=1

cleanup() {
  for pid in $PIDS; do
    kill "$pid" 2>/dev/null || true
  done
  exec 3>&- 2>/dev/null || true
  rm -rf "$RUNTIME"
}
trap cleanup EXIT INT TERM

cargo build --quiet --manifest-path "$ROOT/Cargo.toml" --bins --example virtual_keyboard

mkdir -p "$OUT"
printf 'output * bg #101014 solid_color\n' >"$RUNTIME/sway.conf"
sway -c "$RUNTIME/sway.conf" >"$RUNTIME/sway.log" 2>&1 &
PIDS="$PIDS $!"
for _ in $(seq 1 80); do
  SOCKET=$(ls "$RUNTIME"/wayland-* 2>/dev/null | grep -v '\.lock$' | head -1) || true
  [ -n "${SOCKET:-}" ] && break
  sleep 0.1
done
export WAYLAND_DISPLAY=$(basename "$SOCKET")
echo "sway up on $WAYLAND_DISPLAY"

mkfifo "$RUNTIME/inject"
"$ROOT/target/debug/examples/virtual_keyboard" --stdin <"$RUNTIME/inject" >/dev/null 2>&1 &
PIDS="$PIDS $!"
exec 3>"$RUNTIME/inject"
sleep 0.5

shot() {
  sleep 0.5
  grim "$OUT/$1.png"
  echo "  $OUT/$1.png"
}

# A session per mode: the mode flag is what a compositor hotkey would use.
session() {
  "$ROOT/target/debug/windhover" "$@" >"$RUNTIME/session.out" 2>&1 &
  PIDS="$PIDS $!"
  sleep 1.2
}

echo "capturing"
session --normal
printf 'tap:5\n' >&3
shot normal-multiplier
printf 'tap:Escape\n' >&3
sleep 0.5

session --grid
shot grid
printf 'tap:w\n' >&3
shot grid-shrunk
printf 'tap:Escape\n' >&3
sleep 0.5

session --hint
shot hint
printf 'tap:a\n' >&3
shot hint-filtered
printf 'tap:Escape\n' >&3
sleep 0.5

# And once with a light accent, to see that the theme is really one source.
printf 'accent: FF9500\npanel: F2F2F7E6\ntext: 1C1C1E\n' >"$RUNTIME/alt-config"
"$ROOT/target/debug/windhover" --config "$RUNTIME/alt-config" --grid >"$RUNTIME/session.out" 2>&1 &
PIDS="$PIDS $!"
sleep 1.2
shot grid-light-theme
printf 'tap:Escape\n' >&3

# And over a light background: a full-screen white layer under the overlay, so
# the badges can be judged against the worst case for a light-on-dark pill.
"$ROOT/target/debug/examples/cursor_probe" --layer bottom --fill FFFFFF \
  --timeout 30000 --settle 25000 >/dev/null 2>&1 &
PIDS="$PIDS $!"
sleep 1
"$ROOT/target/debug/windhover" --hint >"$RUNTIME/session.out" 2>&1 &
PIDS="$PIDS $!"
sleep 1.2
shot hint-on-white
printf 'tap:a\n' >&3
shot hint-on-white-filtered
printf 'tap:Escape\n' >&3
sleep 0.4

echo "done"
