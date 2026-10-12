#!/bin/sh
# Render every mockup to docs/mockups/png/ at the README screenshot size, 1280x800: round 1
# (index.html, 3 directions) and round 3 (b.html, 5 card variants), Today/Time x light/dark.
# Needs Chrome/Chromium; set CHROME to override the path.
set -e
cd "$(dirname "$0")"
CHROME=${CHROME:-$(command -v chromium || command -v google-chrome || echo "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")}
mkdir -p png
shot() { # shot <name> <page?query> <light|dark>
  flag=; [ "$3" = dark ] && flag=--force-dark-mode
  "$CHROME" --headless=new --hide-scrollbars --force-device-scale-factor=1 --window-size=1280,800 \
    --virtual-time-budget=2000 --allow-file-access-from-files $flag \
    --screenshot="$PWD/png/$1-$3.png" "file://$PWD/$2" 2>/dev/null
  echo "png/$1-$3.png"
}
for v in today time; do for t in light dark; do
  for d in a b c; do shot "$d-$v" "index.html?d=$d&v=$v" $t; done
  for n in 1 2 3 4 5; do shot "b$n-$v" "b.html?b=$n&v=$v" $t; done
done; done
