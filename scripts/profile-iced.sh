#!/usr/bin/env bash
# Profile the installed qlaunch-iced build. Never installs or swaps APKs.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SERIAL=${SERIAL:-}
LAYER=${LAYER:-}
REFRESH=${REFRESH:-90}
RUNS=${RUNS:-5}

ADB=(adb)
if [ -n "$SERIAL" ]; then
  ADB+=(-s "$SERIAL")
fi

"${ADB[@]}" get-state >/dev/null

if [ -z "$LAYER" ]; then
  # `--list` is empty on some Android 14 builds; fall back to the layer table
  # in the full dump. The suffix (#id) is part of the latency layer name.
  LAYER=$(
    "${ADB[@]}" shell dumpsys SurfaceFlinger --list 2>/dev/null \
      | tr -d '\r' \
      | grep -i 'bitshift.qlaunch' \
      | grep -vi 'Background' \
      | head -1 || true
  )
  if [ -z "$LAYER" ]; then
    LAYER=$(
      "${ADB[@]}" shell dumpsys SurfaceFlinger \
        | tr -d '\r' \
        | grep -o 'Layer \[[0-9]*\] bitshift\.qlaunch[^ ]*' \
        | head -1 \
        | sed 's/^Layer \[[0-9]*\] //' || true
    )
  fi
fi
if [ -z "$LAYER" ]; then
  echo "profile-iced: no SurfaceFlinger layer matches bitshift.qlaunch; set LAYER" >&2
  exit 1
fi

# `--latency` is disabled on some Android 14 builds: it returns only the
# refresh period line and no intervals (even for system layers). Fall back to
# the per-layer TimeStats present2present histogram, which the device fills
# only while the layer keeps producing frames.
latency_rows() {
  "${ADB[@]}" shell "dumpsys SurfaceFlinger --latency '$LAYER'" \
    | tr -d '\r' \
    | awk 'NR > 1 && NF >= 2 && $2 != "0" && $2 != "9223372036854775807" { n++ } END { print n + 0 }'
}

SOURCE=latency
if [ "$(latency_rows)" -lt 2 ]; then
  SOURCE=timestats
fi

echo "profile-iced: layer=$LAYER refresh=${REFRESH}Hz runs=$RUNS source=$SOURCE"

"${ADB[@]}" shell input keyevent KEYCODE_WAKEUP >/dev/null
"${ADB[@]}" shell wm dismiss-keyguard >/dev/null
"${ADB[@]}" shell am start -W -n bitshift.qlaunch/.MainActivity >/dev/null
sleep 2

# Only report frame-work samples produced by this run, not a previous process.
"${ADB[@]}" logcat -c

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

swipe() {
  "${ADB[@]}" shell input touchscreen swipe "$@" >/dev/null
}

# Warm up: one short drag so the first measured run does not start from a cold
# app that has not presented a frame since launch.
swipe 850 1200 700 1200 400
sleep 1

for run in $(seq 1 "$RUNS"); do
  if [ "$SOURCE" = timestats ]; then
    "${ADB[@]}" shell "dumpsys SurfaceFlinger --timestats -clear" >/dev/null
  else
    "${ADB[@]}" shell "dumpsys SurfaceFlinger --latency-clear '$LAYER'" >/dev/null
  fi

  # One continuous 2 s drag per run: the pages follow the finger and then
  # settle. A continuous gesture keeps every present inside the animation, so
  # the presentation interval is not inflated by idle time between swipes.
  # Alternate the direction so wrapping paging is exercised both ways.
  if [ $((run % 2)) -eq 1 ]; then
    swipe 900 1200 200 1200 2000
  else
    swipe 200 1200 900 1200 2000
  fi
  sleep 1

  if [ "$SOURCE" = timestats ]; then
    # Keep only this layer's block; the dump lists every layer.
    "${ADB[@]}" shell "dumpsys SurfaceFlinger --timestats -dump" \
      | tr -d '\r' \
      | awk -v layer="$LAYER" '
          index($0, "layerName = " layer) == 1 { inside = 1; print; next }
          inside && /^layerName = / { exit }
          inside { print }
        ' >"$TMP/run$run.txt"
  else
    "${ADB[@]}" shell "dumpsys SurfaceFlinger --latency '$LAYER'" >"$TMP/run$run.txt"
  fi
  echo "profile-iced: captured run $run"
done

python3 "$ROOT/scripts/latency.py" --refresh "$REFRESH" "$TMP"/run*.txt

echo "profile-iced: recent app-side frame-work samples"
"${ADB[@]}" logcat -d -s qlaunch-iced | grep frame-work | tail -20 || true
