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

# One line per event: timestamp, subagent marker, text or tool call.
filter='. as $l | (try ($l|fromjson) catch {type:"raw",line:$l}) as $e
  | (now|strflocaltime("%H:%M:%S")) as $t
  | (if $e.parent_tool_use_id then "   ↳ " else "" end) as $p
  | if $e.type=="assistant" then
      $e.message.content[]?
      | if .type=="text" then "\($t) \($p)💬 \(.text|gsub("\n";" ")|.[0:240])"
        elif .type=="tool_use" then "\($t) \($p)🔧 \(.name) \((.input.description // .input.command // .input.prompt // .input.file_path // .input.pattern // "")|tostring|gsub("\n";" ")|.[0:160])"
        else empty end
    elif $e.type=="result" then "\($t) ✅ run finished: \(($e.result // $e.subtype)|tostring|gsub("\n";" ")|.[0:300])"
    elif $e.type=="raw" then "\($t) \($e.line)"
    else empty end'

while [ ! -e "$dir/STOP" ]; do
  out=$(mktemp "$dir/out.XXXXXX")
  echo "[$(date '+%F %T')] starting: /autopilot $args" >> "$log"
  caffeinate -i "$claude_bin" -p "/autopilot $args" --model "${AUTOPILOT_MODEL:-sonnet}" --permission-mode auto \
    --output-format stream-json --verbose 2>&1 | tee "$out" | jq -R --unbuffered -r "$filter" | tee -a "$log"
  code=${PIPESTATUS[0]}
  echo "[$(date '+%F %T')] exited $code" | tee -a "$log"
  tailout=$(tail -n 5 "$out")   # limit/auth errors show up at the end of the stream
  if grep -qiE 'usage limit|rate limit|session limit|limit reached|resets' <<<"$tailout"; then
    wait=$(reset_wait "$out"); wait=${wait:-1800}
    [ "$wait" -gt 21600 ] && wait=1800
    echo "[$(date '+%F %T')] limit: sleeping ${wait}s" >> "$log"
  elif grep -qiE 'failed to authenticate|oauth|not logged in|/login' <<<"$tailout"; then
    osascript -e 'display notification "Run `claude` and /login (or `claude setup-token`)" with title "Margin autopilot: auth expired"' 2>/dev/null
    echo "[$(date '+%F %T')] auth expired: retrying in 300s" >> "$log"
    wait=300
  elif [ "$code" -eq 0 ]; then
    wait=60
  else
    wait=900
  fi
  rm -f "$out"
  # sleep in short slices so STOP is honoured
  while [ "$wait" -gt 0 ] && [ ! -e "$dir/STOP" ]; do
    s=$(( wait < 30 ? wait : 30 )); sleep "$s"; wait=$((wait - s))
  done
done
echo "[$(date '+%F %T')] STOP found, exiting" >> "$log"
