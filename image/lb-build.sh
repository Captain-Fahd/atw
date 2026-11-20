#!/bin/bash
# Runs INSIDE the builder container (see build.sh). Builds the live ISO with
# live-build, then the USB disk image. Repo is mounted at /work, the
# live-build work dir (with its package cache) at /build.
set -euo pipefail

BIN=/work/target/x86_64-unknown-linux-musl/release/atw
OUT=/work/build
LB=/build/lb
[ -x "$BIN" ] || { echo "missing $BIN: run cargo build first" >&2; exit 1; }

mkdir -p "$LB" "$OUT"
cd "$LB"
lb clean >/dev/null 2>&1 || true   # keeps cache/ between builds
rm -rf config auto

lb config \
	--distribution trixie \
	--archive-areas "main contrib non-free-firmware" \
	--binary-image iso-hybrid \
	--bootloaders grub-efi \
	--uefi-secure-boot enable \
	--debian-installer none \
	--memtest none \
	--firmware-chroot true \
	--firmware-binary false \
	--apt-recommends false \
	--apt-indices false \
	--iso-volume ATW-LIVE \
	--iso-application "AccessibleTech Wipe" \
	--image-name atw-live \
	--bootappend-live "boot=live live-config.noautologin atw.live=1 quiet console=ttyS0,115200 console=tty0" \
	--bootappend-live-failsafe "boot=live live-config.noautologin nomodeset noapic nosplash"

# Fail-safe mode deliberately has no atw.live=1, so it can never wipe.
# Start from live-build's default GRUB templates, then apply ours.
cp -a /usr/share/live/build/bootloaders/grub-pc config/bootloaders/
cp -a /work/image/live/config/. config/
install -D -m 0755 "$BIN" config/includes.chroot/usr/local/bin/atw

lb build

cp atw-live-amd64.hybrid.iso "$OUT/atw-live.iso"
/work/image/mkimage.sh "$OUT/atw-live.iso" "$OUT/atw-usb.img"
(cd "$OUT" && sha256sum atw-live.iso atw-usb.img > SHA256SUMS)
