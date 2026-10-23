#!/usr/bin/env bash
# Keeps /autopilot running across session limits and crashes. State lives in
# GitHub/git; /autopilot step 0 adopts unfinished work on every restart.
#
# Run:      scripts/autopilot-forever.sh            (foreground, shows a live dashboard)
# Install:  edit the paths in scripts/com.margin.autopilot.plist, copy it to
#           ~/Library/LaunchAgents/ and `launchctl load` it.
# Stop:     touch ~/.cache/margin-autopilot/STOP   (or kill the process)
# Env:      AUTOPILOT_ARGS         (default "--hours 5")
#           AUTOPILOT_MODEL        coordinator model (default sonnet)
#           AUTOPILOT_WEEKLY_CAP   pause new runs at this weekly utilization, 0-1 (default 0.95,
#                                  leaves a little for your own sessions; 1 = use everything)
#           AUTOPILOT_STALL        kill and restart a run with no events for this many seconds (default 1200)
#           AUTOPILOT_MIN_DISK_GB  clean the agents' build cache, then pause, below this (default 8)
#           AUTOPILOT_CONTEXT      coordinator context window in tokens for rotation (default 200000)
#           CLAUDE_BIN             (default claude)
# Guards:   limits (sleeps to the exact 5h/weekly reset), hang watchdog, fast-exit backoff,
#           context rotation (fresh coordinator at 70-90% context), main-CI circuit breaker
#           (3 red runs in a row: a fix-only run, then a pause), low-disk guard, log rotation.
set -u
cd "$(dirname "$0")/.."
dir="$HOME/.cache/margin-autopilot"
mkdir -p "$dir"
log="$dir/run.log"
lock="$dir/lock"
limits="$dir/limits.json"   # last rate-limit info, written by autopilot-ui.mjs
hb="$dir/heartbeat"         # touched by the dashboard on every event
tailf="$dir/tail.txt"       # last raw lines of the finished run
rotate="$dir/rotate"        # written by the dashboard when it ends a run for context rotation
stalled="$dir/stalled"
repo="albin02forsberg/margin"

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
stall=${AUTOPILOT_STALL:-1200}
min_disk=${AUTOPILOT_MIN_DISK_GB:-8}
export AUTOPILOT_LIMITS="$limits" AUTOPILOT_HEARTBEAT="$hb" AUTOPILOT_TAIL="$tailf" AUTOPILOT_ROTATE="$rotate"
AUTOPILOT_MAX=$(sed -nE 's/.*--max[ =]([0-9]+).*/\1/p' <<<"$args"); export AUTOPILOT_MAX=${AUTOPILOT_MAX:-3}

say() { echo "[$(date '+%F %T')] $*" >> "$log"; }
notify() { osascript -e "display notification \"$2\" with title \"Margin autopilot: $1\"" 2>/dev/null; }

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

# countdown <seconds> <why>: one updating line on a TTY, silent otherwise (launchd log); honours STOP.
countdown() {
  local until left sum
  until=$(( $(date +%s) + $1 ))
  sum=$(limit_summary)
  while [ "$(date +%s)" -lt "$until" ] && [ ! -e "$dir/STOP" ]; do
    left=$(( until - $(date +%s) ))
    [ -t 1 ] && printf '\r\033[K💤 %s - next run in %dd %dh %02dm %02ds%s (STOP file: ~/.cache/margin-autopilot/STOP)' "$2" \
      $((left / 86400)) $((left % 86400 / 3600)) $((left % 3600 / 60)) $((left % 60)) "${sum:+ · $sum}"
    sleep 5
  done
  [ -t 1 ] && printf '\r\033[K'
  return 0
}

# Gates return 0 when they paused (the caller restarts the loop), 1 when clear.
weekly_gate() {  # at the weekly cap: wait out the week
  [ -s "$limits" ] || return 1
  local now u r
  now=$(date +%s)
  u=$(jq -r '.unifiedWindows.seven_day.utilization // 0' "$limits" 2>/dev/null)
  r=$(jq -r '.unifiedWindows.seven_day.resetsAt // 0' "$limits" 2>/dev/null)
  [ "${r:-0}" -gt "$now" ] 2>/dev/null || return 1
  awk -v u="$u" -v c="$cap" 'BEGIN{exit !(u >= c)}' || return 1
  say "weekly usage $u >= cap $cap: pausing until $(date -r "$r" '+%F %T')"
  notify "paused" "Weekly usage at $(awk -v u="$u" 'BEGIN{printf "%d", u*100}')%. Resuming $(date -r "$r" '+%a %H:%M')"
  countdown $((r - now + 120)) "weekly cap reached"
}

free_gb() { df -k / | awk 'NR==2{printf "%d", $4/1048576}'; }
disk_gate() {  # low disk: drop the agents' build cache (safe to rebuild), else pause
  [ "$(free_gb)" -ge "$min_disk" ] && return 1
  say "low disk ($(free_gb) GB free): cleaning agent build cache"
  rm -rf "$HOME/.cache/margin-agents-target"; git worktree prune 2>/dev/null
  [ "$(free_gb)" -ge "$min_disk" ] && return 1
  notify "low disk" "$(free_gb) GB free after cleaning. Free some space; retrying in 30 min."
  countdown 1800 "low disk ($(free_gb) GB free)"
}

ci_state() {  # "red" when the last 3 completed ci.yml runs on main all failed
  gh run list -R "$repo" --branch main --workflow ci.yml --limit 6 --json status,conclusion 2>/dev/null |
    jq -r '[.[] | select(.status=="completed")][0:3] | map(.conclusion) | if length==3 and all(.=="failure") then "red" else "ok" end' 2>/dev/null
}

fixes=0; fast=0
while [ ! -e "$dir/STOP" ]; do
  [ "$(stat -f %z "$log" 2>/dev/null || echo 0)" -gt 5242880 ] && mv -f "$log" "$log.1"
  weekly_gate && continue
  disk_gate && continue

  prompt="/autopilot $args"; kind=normal
  if [ "$(ci_state)" = red ]; then
    if [ "$fixes" -ge 2 ]; then
      say "main CI still red after $fixes fix attempts: pausing 1h"
      notify "main CI is red" "Two automatic fix attempts failed. Needs a look; retrying in 1h."
      fixes=0; countdown 3600 "main CI red, fix attempts used up"; continue
    fi
    fixes=$((fixes + 1)); kind=fix
    notify "main CI is red" "3 failed runs in a row. Starting a fix-only run (attempt $fixes)."
    prompt="CI on main has failed 3 runs in a row. Find the failing run (gh run list / gh run view --log-failed), run the implementer agent to fix it in a PR, and squash-merge it on green CI. If it is only a flake, rerun it. Do nothing else, then exit."
  else
    fixes=0
  fi

  session=$(uuidgen | tr A-Z a-z); export AUTOPILOT_SESSION="$session"
  rm -f "$stalled" "$rotate" "$tailf"; : > "$hb"; started_at=$(date +%s)
  say "starting ($kind): $prompt"
  # watchdog: no events for $stall seconds means the run is hung
  poll=$((stall / 4)); [ "$poll" -gt 30 ] && poll=30; [ "$poll" -lt 1 ] && poll=1
  ( while sleep "$poll"; do
      [ $(( $(date +%s) - $(stat -f %m "$hb" 2>/dev/null || echo 0) )) -gt "$stall" ] && { touch "$stalled"; pkill -f -- "$session"; exit; }
    done ) &
  wd=$!
  caffeinate -i "$claude_bin" -p "$prompt" --session-id "$session" --model "${AUTOPILOT_MODEL:-sonnet}" --permission-mode auto \
    --output-format stream-json --verbose 2>&1 | AUTOPILOT_LOG="$log" node scripts/autopilot-ui.mjs "$session"
  code=${PIPESTATUS[0]}
  kill "$wd" 2>/dev/null; wait "$wd" 2>/dev/null
  sleep 1   # let the dashboard flush its tail file
  elapsed=$(( $(date +%s) - started_at ))
  say "exited $code after ${elapsed}s"

  # Only the final result line and plain (non-JSON) errors can announce a limit. Assistant/user/rate_limit
  # events are skipped: they hold agents' own text and "resetsAt", which must not look like a limit message.
  tailflt="$dir/tail.filtered"
  grep -vE '^\{"type":"(assistant|user|system|rate_limit_event|stream_event)"' "$tailf" > "$tailflt" 2>/dev/null
  tailout=$(cat "$tailflt" 2>/dev/null)
  # limit info from this run only
  info=$(jq -c --argjson s $((started_at * 1000)) 'select(.ts >= $s)' "$limits" 2>/dev/null)
  status=$(jq -r '.status // empty' <<<"$info" 2>/dev/null); rtype=$(jq -r '.rateLimitType // empty' <<<"$info" 2>/dev/null)
  rat=$(jq -r '.resetsAt // empty' <<<"$info" 2>/dev/null); now=$(date +%s)

  if [ -e "$rotate" ]; then
    wait=5; why="context rotation"; say "context rotation: fresh coordinator"
  elif [ -e "$stalled" ]; then
    wait=30; why="run stalled for ${stall}s, restarted"; say "watchdog: no events for ${stall}s, killed the run"
    notify "run stalled" "No activity for $((stall / 60)) min. Restarted."
  elif [ "$status" = "rejected" ] || grep -qiE 'usage limit|rate limit|session limit|limit reached|resets (at |in )?[0-9]' <<<"$tailout"; then
    if [ -n "$rat" ] && [ "$rat" -gt "$now" ] 2>/dev/null; then wait=$((rat - now + 120)); else wait=$(reset_wait "$tailflt"); wait=${wait:-1800}; fi
    maxwait=21600; case "$rtype" in seven_day*) maxwait=$((8 * 86400));; esac
    [ "$wait" -gt "$maxwait" ] && wait=1800
    why="usage limit (${rtype:-unknown window})"
    case "$rtype" in seven_day*) notify "paused" "Weekly limit hit. Resuming $(date -v+${wait}S '+%a %H:%M')";; esac
    say "limit ($rtype): sleeping ${wait}s"
  elif grep -qiE 'failed to authenticate|oauth|not logged in|/login' <<<"$tailout"; then
    notify "auth expired" 'Run `claude` and /login (or `claude setup-token`)'
    say "auth expired: retrying in 300s"
    wait=300; why="auth expired: run 'claude' and /login"
  else
    if [ "$code" -eq 0 ]; then wait=60; why="run finished"; else wait=900; why="run failed (exit $code)"; fi
    [ "$kind" = fix ] && { wait=600; why="waiting for CI after the fix attempt"; }
    if [ "$elapsed" -lt 300 ]; then   # exits within 5 min: back off so restarts do not burn tokens
      fast=$((fast + 1)); b=$((60 << (fast > 4 ? 4 : fast))); [ "$b" -gt 900 ] && b=900
      [ "$wait" -lt "$b" ] && wait=$b; why="$why (short run #$fast, backing off)"
    else
      fast=0
    fi
  fi
  countdown "$wait" "$why"
done
say "STOP found, exiting"
