#!/usr/bin/env bash
# Run the Iced port on the desktop for comparison with the Flutter build.
# Extra args go to cargo (e.g. ./run.sh --release).
set -euo pipefail
cd "$(dirname "$0")/rust"

# A desktop session normally exports these; fill them in for a bare shell.
if [[ -z "${XDG_RUNTIME_DIR:-}" ]]; then
  export XDG_RUNTIME_DIR="/run/user/$(id -u)"
fi
if [[ -z "${WAYLAND_DISPLAY:-}" && -S "$XDG_RUNTIME_DIR/wayland-1" ]]; then
  export WAYLAND_DISPLAY=wayland-1
fi

exec cargo run -p qlaunch_iced --no-default-features --bin qlaunch-iced "$@"
