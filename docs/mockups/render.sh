#!/bin/sh
# Render every mockup to docs/mockups/png/ at the README screenshot size, 1280x800: round 1
# (index.html, 3 directions), round 3 (b.html, 5 card variants), Today/Time x light/dark, and
# round 4 (r4.html: 6 pages x B1/B2/B5 + the B1/B5 mix, light/dark) plus its motion GIFs in gif/.
# Usage: render.sh [all|r4|gif [demo]]. Needs Chrome/Chromium (set CHROME to override) and ffmpeg for GIFs.
set -e
cd "$(dirname "$0")"
CHROME=${CHROME:-$(command -v chromium || command -v google-chrome || echo "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")}
what=${1:-all} only=$2 # render.sh gif <demo> redoes one GIF
mkdir -p png gif
grab() { # grab <out.png> <page?query> <light|dark>
  flag=; [ "$3" = dark ] && flag=--force-dark-mode
  "$CHROME" --headless=new --hide-scrollbars --force-device-scale-factor=1 --window-size=1280,800 \
    --virtual-time-budget=2000 --allow-file-access-from-files $flag --screenshot="$1" "file://$PWD/$2" 2>/dev/null
}
shot() { grab "$PWD/png/$1-$3.png" "$2" "$3"; echo "png/$1-$3.png"; } # shot <name> <page?query> <light|dark>
if [ "$what" = all ]; then
  for v in today time; do for t in light dark; do
    for d in a b c; do shot "$d-$v" "index.html?d=$d&v=$v" $t; done
    for n in 1 2 3 4 5; do shot "b$n-$v" "b.html?b=$n&v=$v" $t; done
  done; done
fi
if [ "$what" = all ] || [ "$what" = r4 ]; then
  for t in light dark; do
    for p in editor tasks projects settings palette graph; do for s in 1 2 5; do shot "b$s-$p" "r4.html?s=$s&p=$p" $t; done; done
    shot "mix-tasks" "r4.html?s=mix&p=tasks" $t
  done
fi
# gif <demo> <loop ms> <light|dark> [ffmpeg crop] [width]: one frame per 40 ms (25 fps), each a paused point of the
# demo's animations (r4.html &t=ms), joined into a looping GIF scaled to WIDTH (800) px.
gif() {
  [ -z "$only" ] || [ "$only" = "$1" ] || return 0
  dir=$(mktemp -d) i=0
  while [ $((i * 40)) -lt "$2" ]; do
    grab "$dir/$(printf %03d $i).png" "r4.html?demo=$1&t=$((i * 40))" "$3" &
    i=$((i + 1)); [ $((i % 8)) -ne 0 ] || wait
  done; wait
  ffmpeg -v error -y -framerate 25 -i "$dir/%03d.png" -vf "${4:+$4,}scale=${5:-800}:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" -loop 0 "gif/$1-$3.gif"
  rm -r "$dir"; echo "gif/$1-$3.gif"
}
if [ "$what" = all ] || [ "$what" = gif ]; then
  for t in light dark; do
    gif view 4800 $t; gif hover 4800 $t; gif palette 6400 $t "" 720; gif move 4800 $t "crop=1280:560:0:0"; gif clock 4800 $t "crop=800:390:280:76"
  done
fi
