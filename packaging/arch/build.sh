#!/bin/bash
# Builds margin-bin for VERSION in DIR, which holds Margin_<VERSION>_amd64.deb (#127). Runs as root in an
# archlinux container (release.yml, ci.yml); makepkg itself refuses root, so it runs as a build user.
# Leaves the .pkg.tar.zst and the PKGBUILD with pkgver and checksum filled in DIR.
set -euo pipefail
version=$1 dir=$(realpath "$2")
pacman -Syu --noconfirm --needed base-devel pacman-contrib namcap
id builder &>/dev/null || useradd -m builder
sed "s/^pkgver=.*/pkgver=$version/" "$(dirname "$0")/PKGBUILD" > "$dir/PKGBUILD"
chown -R builder "$dir"
cd "$dir"
su builder -c 'updpkgsums && makepkg -f --noconfirm'
namcap PKGBUILD ./*.pkg.tar.zst
# Install it and check the binary finds every library the depends pull in.
pacman -U --noconfirm ./*.pkg.tar.zst
! ldd /usr/bin/margin | grep 'not found'
