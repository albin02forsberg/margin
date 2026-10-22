#!/usr/bin/env bash
# Keeps /autopilot running across session limits and crashes. State lives in
# GitHub/git; /autopilot step 0 adopts unfinished work on every restart.
#
# Run:      scripts/autopilot-forever.sh            (foreground, shows a live dashboard)
# Install:  edit the paths in scripts/com.margin.autopilot.plist, copy it to
#           ~/Library/LaunchAgents/ and `launchctl load` it.
# Stop:     touch ~/.cache/margin-autopilot/STOP   (or kill the process)
# Env:      AUTOPILOT_ARGS        (default "--hours 5")
#           AUTOPILOT_MODEL       coordinator model (default sonnet)
#           AUTOPILOT_WEEKLY_CAP  pause new runs at this weekly utilization, 0-1 (default 0.95,
#                                 leaves a little for your own sessions; 1 = use everything)
#           CLAUDE_BIN            (default claude)
# Limits: runs report exact 5-hour and weekly reset times; when blocked the runner sleeps until the
#         reset (days for the weekly limit), then restarts. Status shows in the countdown line.
set -u
cd "$(dirname "$0")/.."
dir="$HOME/.cache/margin-autopilot"
mkdir -p "$dir"
log="$dir/run.log"
lock="$dir/lock"
limits="$dir/limits.json"   # last rate-limit info, written by autopilot-ui.mjs

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
cap=${AUTOPILOT_WEEKLY_CAP:-0.95}
export AUTOPILOT_LIMITS="$limits"
AUTOPILOT_MAX=$(sed -nE 's/.*--max[ =]([0-9]+).*/\1/p' <<<"$args"); export AUTOPILOT_MAX=${AUTOPILOT_MAX:-3}

# Seconds until the reset time named in a limit message ("resets 3pm"), else empty. Fallback only.
reset_wait() {
  local t now target
  t=$(grep -ioE 'resets? (at )?[0-9]{1,2}(:[0-9]{2})? ?(am|pm)?' "$1" | tail -1 | grep -oE '[0-9]{1,2}(:[0-9]{2})? ?(am|pm)?')
  [ -n "$t" ] || return 0
  now=$(date +%s)
  target=$(date -j -f '%I:%M%p' "$(echo "$t" | tr -d ' ' | tr a-z A-Z | sed -E 's/^([0-9]+)(AM|PM)$/\1:00\2/')" +%s 2>/dev/null) || return 0
  [ "$target" -le "$now" ] && target=$((target + 86400))
  echo $((target - now + 120))
}

# Short summary of the last known limits for the countdown line.
limit_summary() {
  [ -s "$limits" ] || return 0
  jq -r '.unifiedWindows // {} | "5h \((.five_hour.utilization // 0) * 100 | round)% · week \((.seven_day.utilization // 0) * 100 | round)%"' "$limits" 2>/dev/null
}

# countdown <seconds> <why>: one updating line, honours STOP.
countdown() {
  local until left sum
  until=$(( $(date +%s) + $1 ))
  sum=$(limit_summary)
  while [ "$(date +%s)" -lt "$until" ] && [ ! -e "$dir/STOP" ]; do
    left=$(( until - $(date +%s) ))
    printf '\r\033[K💤 %s - next run in %dd %dh %02dm %02ds%s (STOP file: ~/.cache/margin-autopilot/STOP)' "$2" \
      $((left / 86400)) $((left % 86400 / 3600)) $((left % 3600 / 60)) $((left % 60)) "${sum:+ · $sum}"
    sleep 5
  done
  printf '\r\033[K'
}

# Weekly cap: before a run, wait out the week if the last known utilization is at the cap.
weekly_gate() {
  [ -s "$limits" ] || return 1
  local now u r
  now=$(date +%s)
  u=$(jq -r '.unifiedWindows.seven_day.utilization // 0' "$limits" 2>/dev/null)
  r=$(jq -r '.unifiedWindows.seven_day.resetsAt // 0' "$limits" 2>/dev/null)
  [ "${r:-0}" -gt "$now" ] 2>/dev/null || return 1
  awk -v u="$u" -v c="$cap" 'BEGIN{exit !(u >= c)}' || return 1
  echo "[$(date '+%F %T')] weekly usage $u >= cap $cap: pausing until $(date -r "$r" '+%F %T')" >> "$log"
  osascript -e "display notification \"Weekly usage at $(awk -v u="$u" 'BEGIN{printf "%d", u*100}')%. Resuming $(date -r "$r" '+%a %H:%M')\" with title \"Margin autopilot paused\"" 2>/dev/null
  countdown $((r - now + 120)) "weekly cap reached"
  return 0
}

while [ ! -e "$dir/STOP" ]; do
  weekly_gate && continue
  out=$(mktemp "$dir/out.XXXXXX")
  echo "[$(date '+%F %T')] starting: /autopilot $args" >> "$log"
  caffeinate -i "$claude_bin" -p "/autopilot $args" --model "${AUTOPILOT_MODEL:-sonnet}" --permission-mode auto \
    --output-format stream-json --verbose 2>&1 | tee "$out" | AUTOPILOT_LOG="$log" node scripts/autopilot-ui.mjs
  code=${PIPESTATUS[0]}
  echo "[$(date '+%F %T')] exited $code" | tee -a "$log"
  tailout=$(tail -n 5 "$out")   # limit/auth errors show up at the end of the stream
  info=$(jq -R -c 'fromjson? | select(.type=="rate_limit_event") | .rate_limit_info' "$out" 2>/dev/null | tail -1)
  status=$(jq -r '.status // empty' <<<"$info" 2>/dev/null); rtype=$(jq -r '.rateLimitType // empty' <<<"$info" 2>/dev/null)
  rat=$(jq -r '.resetsAt // empty' <<<"$info" 2>/dev/null); now=$(date +%s)
  if [ "$status" = "rejected" ] || grep -qiE 'usage limit|rate limit|session limit|limit reached|resets' <<<"$tailout"; then
    if [ -n "$rat" ] && [ "$rat" -gt "$now" ] 2>/dev/null; then wait=$((rat - now + 120)); else wait=$(reset_wait "$out"); wait=${wait:-1800}; fi
    maxwait=21600; case "$rtype" in seven_day*) maxwait=$((8 * 86400));; esac
    [ "$wait" -gt "$maxwait" ] && wait=1800
    why="usage limit (${rtype:-unknown window})"
    case "$rtype" in seven_day*) osascript -e "display notification \"Weekly limit hit. Resuming $(date -v+${wait}S '+%a %H:%M')\" with title \"Margin autopilot paused\"" 2>/dev/null;; esac
    echo "[$(date '+%F %T')] limit ($rtype): sleeping ${wait}s" >> "$log"
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
  countdown "$wait" "$why"
done
echo "[$(date '+%F %T')] STOP found, exiting" >> "$log"
