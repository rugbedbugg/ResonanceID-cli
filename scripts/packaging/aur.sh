#!/usr/bin/env bash
set -euo pipefail
# Execute only in the disposable Arch validation container.
pacman -Syu --noconfirm --needed base-devel git rust namcap alsa-lib
useradd -m builder
chown -R builder:builder prepared/aur
cd prepared/aur
su builder -c 'makepkg --printsrcinfo' > .SRCINFO
su builder -c 'makepkg --verifysource'
su builder -c 'makepkg --cleanbuild --noconfirm'
namcap PKGBUILD
namcap ./*.pkg.tar.zst
pacman -U --noconfirm ./*.pkg.tar.zst
resonanceid-cli --help
pacman -R --noconfirm resonanceid-cli
