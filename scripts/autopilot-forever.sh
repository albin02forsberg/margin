#!/usr/bin/env bash
# Keeps /autopilot running across session limits and crashes. State lives in
# GitHub/git; /autopilot step 0 adopts unfinished work on every restart.
#
# Run:      scripts/autopilot-forever.sh            (foreground)
# Install:  edit the paths in scripts/com.margin.autopilot.plist, copy it to
#           ~/Library/LaunchAgents/ and `launchctl load` it.
# Stop:     touch ~/.cache/margin-autopilot/STOP   (or kill the process)
# Env:      AUTOPILOT_ARGS (default "--hours 5"), CLAUDE_BIN (default claude)
set -u
cd "$(dirname "$0")/.."
dir="$HOME/.cache/margin-autopilot"
mkdir -p "$dir"
log="$dir/run.log"
lock="$dir/lock"

if ! mkdir "$lock" 2>/dev/null; then
  pid=$(cat "$lock/pid" 2>/dev/null || true)
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    echo "already running (pid $pid)"; exit 0
  fi
  rm -rf "$lock"; mkdir "$lock"
fi
echo $$ > "$lock/pid"
trap 'rm -rf "$lock"' EXIT

claude_bin=${CLAUDE_BIN:-claude}
args=${AUTOPILOT_ARGS:---hours 5}
why=""

# Seconds until the reset time named in a limit message ("resets 3pm", "resets at 15:00"), else empty.
reset_wait() {
  local t now target
  t=$(grep -ioE 'resets? (at )?[0-9]{1,2}(:[0-9]{2})? ?(am|pm)?' "$1" | tail -1 | grep -oE '[0-9]{1,2}(:[0-9]{2})? ?(am|pm)?')
  [ -n "$t" ] || return 0
  now=$(date +%s)
  target=$(date -j -f '%I:%M%p' "$(echo "$t" | tr -d ' ' | tr a-z A-Z | sed -E 's/^([0-9]+)(AM|PM)$/\1:00\2/')" +%s 2>/dev/null) || return 0
  [ "$target" -le "$now" ] && target=$((target + 86400))
  echo $((target - now + 120))
}

while [ ! -e "$dir/STOP" ]; do
  out=$(mktemp "$dir/out.XXXXXX")
  echo "[$(date '+%F %T')] starting: /autopilot $args" >> "$log"
  caffeinate -i "$claude_bin" -p "/autopilot $args" --model "${AUTOPILOT_MODEL:-sonnet}" --permission-mode auto \
    --output-format stream-json --verbose 2>&1 | tee "$out" | AUTOPILOT_LOG="$log" node scripts/autopilot-ui.mjs
  code=${PIPESTATUS[0]}
  echo "[$(date '+%F %T')] exited $code" | tee -a "$log"
  tailout=$(tail -n 5 "$out")   # limit/auth errors show up at the end of the stream
  if grep -qiE 'usage limit|rate limit|session limit|limit reached|resets' <<<"$tailout"; then
    wait=$(reset_wait "$out"); wait=${wait:-1800}
    [ "$wait" -gt 21600 ] && wait=1800
    why="usage limit"
    echo "[$(date '+%F %T')] limit: sleeping ${wait}s" >> "$log"
  elif grep -qiE 'failed to authenticate|oauth|not logged in|/login' <<<"$tailout"; then
    osascript -e 'display notification "Run `claude` and /login (or `claude setup-token`)" with title "Margin autopilot: auth expired"' 2>/dev/null
    echo "[$(date '+%F %T')] auth expired: retrying in 300s" >> "$log"
    wait=300; why="auth expired: run 'claude' and /login"
  elif [ "$code" -eq 0 ]; then
    wait=60; why="run finished"
  else
    wait=900; why="run failed (exit $code)"
  fi
  rm -f "$out"
  # countdown in short slices so STOP is honoured
  until=$(( $(date +%s) + wait ))
  while [ "$(date +%s)" -lt "$until" ] && [ ! -e "$dir/STOP" ]; do
    left=$(( until - $(date +%s) ))
    printf '\r\033[K💤 %s - next run in %dm%02ds (touch %s/STOP to quit)' "$why" $((left / 60)) $((left % 60)) "$dir"
    sleep 5
  done
  printf '\r\033[K'
done
echo "[$(date '+%F %T')] STOP found, exiting" >> "$log"
