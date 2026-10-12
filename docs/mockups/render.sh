#!/bin/sh
# Render every mockup (3 directions x Today/Time x light/dark) to docs/mockups/png/ at the
# README screenshot size, 1280x800. Needs Chrome/Chromium; set CHROME to override the path.
set -e
cd "$(dirname "$0")"
CHROME=${CHROME:-$(command -v chromium || command -v google-chrome || echo "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")}
mkdir -p png
for d in a b c; do for v in today time; do for t in light dark; do
  flag=; [ $t = dark ] && flag=--force-dark-mode
  "$CHROME" --headless=new --hide-scrollbars --force-device-scale-factor=1 --window-size=1280,800 \
    --virtual-time-budget=2000 --allow-file-access-from-files $flag \
    --screenshot="$PWD/png/$d-$v-$t.png" "file://$PWD/index.html?d=$d&v=$v" 2>/dev/null
  echo "png/$d-$v-$t.png"
done; done; done
