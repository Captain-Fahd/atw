#!/usr/bin/env bash
# Boots build/atw-usb.img in QEMU with OVMF Secure Boot (Microsoft keys
# enrolled), an emulated NVMe disk and an emulated SATA disk, all backed by
# files under build/qemu/. Waits for atw probe to finish, then extracts its
# JSON from the image's ATECH-DATA partition and checks it.
# Rootless: needs /dev/kvm access for speed, nothing else. Run from the dev
# shell (OVMF_CODE / OVMF_VARS come from flake.nix).
set -euo pipefail
cd "$(dirname "$0")/.."

: "${OVMF_CODE:?run inside nix develop}" "${OVMF_VARS:?run inside nix develop}"
IMG=build/atw-usb.img
Q=build/qemu
TIMEOUT=${TIMEOUT:-900}
[ -f "$IMG" ] || { echo "missing $IMG: run image/build.sh first" >&2; exit 1; }

rm -rf "$Q" && mkdir -p "$Q"
cp "$OVMF_VARS" "$Q/vars.fd" && chmod u+w "$Q/vars.fd"
cp --sparse=always "$IMG" "$Q/usb.img"
truncate -s 4G "$Q/nvme.img" "$Q/sata.img"

ACCEL=tcg
[ -w /dev/kvm ] && ACCEL=kvm

qemu-system-x86_64 \
	-machine q35,smm=on,accel=$ACCEL -m 2048 -smp 2 \
	-global driver=cfi.pflash01,property=secure,value=on \
	-drive if=pflash,format=raw,unit=0,readonly=on,file="$OVMF_CODE" \
	-drive if=pflash,format=raw,unit=1,file="$Q/vars.fd" \
	-device qemu-xhci,id=xhci \
	-drive if=none,id=usb,format=raw,file="$Q/usb.img" \
	-device usb-storage,bus=xhci.0,drive=usb,removable=on,bootindex=0 \
	-drive if=none,id=nvme0,format=raw,file="$Q/nvme.img" \
	-device nvme,serial=ATWNVME0001,drive=nvme0 \
	-drive if=none,id=sata0,format=raw,file="$Q/sata.img" \
	-device ide-hd,bus=ide.0,drive=sata0,serial=ATWSATA0001,model=ATW-TEST-SATA \
	-display none -serial file:"$Q/serial.log" \
	-monitor unix:"$Q/monitor.sock",server,nowait \
	-pidfile "$Q/qemu.pid" -daemonize

cleanup() { [ -f "$Q/qemu.pid" ] && kill "$(cat "$Q/qemu.pid")" 2>/dev/null || true; }
trap cleanup EXIT

echo "booting (accel=$ACCEL), waiting up to ${TIMEOUT}s for probe..."
for _ in $(seq "$TIMEOUT"); do
	grep -q ATW-PROBE-DONE "$Q/serial.log" 2>/dev/null && break
	sleep 1
done
if ! grep -q ATW-PROBE-DONE "$Q/serial.log"; then
	# Save a screenshot of the display (tty1) for debugging.
	echo "screendump $PWD/$Q/screen.ppm" | socat - UNIX-CONNECT:"$Q/monitor.sock" >/dev/null 2>&1 || true
	sleep 1
	echo "FAIL: probe did not finish; see $Q/serial.log and $Q/screen.ppm" >&2
	exit 1
fi
sleep 2
cleanup
sleep 1

# Pull probe/ out of partition 2 without mounting.
read -r START SIZE < <(sfdisk -J "$Q/usb.img" | jq -r '.partitiontable.partitions[1] | "\(.start) \(.size)"')
dd if="$Q/usb.img" of="$Q/data.img" bs=512 skip="$START" count="$SIZE" status=none
# QEMU was killed, so replay the ext4 journal on the copy first.
e2fsck -fy "$Q/data.img" >/dev/null 2>&1 || [ $? -le 1 ]
debugfs -R "rdump /probe $Q" "$Q/data.img" 2>/dev/null
JSON=$(ls "$Q"/probe/*.json | head -n1)
echo "probe JSON: $JSON"

jq -e '
	.host.secure_boot == true and
	([.disks[] | select(.internal)] | length) == 2 and
	([.disks[] | select(.internal and .media == "nvme" and .passthrough_ok)] | length) == 1 and
	([.disks[] | select(.internal and .media == "ata" and .passthrough_ok)] | length) == 1 and
	([.disks[] | select(.boot_device)] | length) == 1
' "$JSON" >/dev/null && echo "PASS: Secure Boot on, lockdown=$(jq -r .host.lockdown "$JSON"), NVMe + SATA identified" || {
	echo "FAIL: unexpected probe result:" >&2
	jq -c '{sb: .host.secure_boot, lockdown: .host.lockdown, disks: [.disks[] | {path, media, internal, boot_device, passthrough_ok}]}' "$JSON" >&2
	exit 1
}
