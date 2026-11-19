#!/usr/bin/env bash
# Writes build/atw-usb.img to a USB stick, grows ATECH-DATA to fill it and
# gives it a fresh UUID. DESTROYS everything on the target device.
# Omar runs this by hand. Never run it from automation.
#   Usage: sudo image/make-usb.sh /dev/sdX
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "make-usb: $*" >&2; exit 1; }
DEV=${1:-}
IMG=build/atw-usb.img

[ -n "$DEV" ] || die "usage: sudo image/make-usb.sh /dev/sdX"
[ "$(id -u)" = 0 ] || die "must run as root (sudo)"
[ -f "$IMG" ] || die "missing $IMG; run image/build.sh first"
[[ "$DEV" =~ ^/dev/sd[a-z]+$ ]] || die "$DEV is not a /dev/sdX whole-disk path"
[ -b "$DEV" ] || die "$DEV is not a block device"

read -r TYPE TRAN RM < <(lsblk -dno TYPE,TRAN,RM "$DEV")
[ "$TYPE" = disk ] || die "$DEV is not a disk"
[ "$TRAN" = usb ] || die "$DEV transport is '${TRAN:-none}', not usb"
[ "$RM" = 1 ] || die "$DEV is not marked removable (RM=$RM); refusing"
if lsblk -nro MOUNTPOINTS "$DEV" | grep -q .; then
	die "$DEV has mounted partitions; unmount them first"
fi
DEV_BYTES=$(lsblk -bdno SIZE "$DEV")
IMG_BYTES=$(stat -c %s "$IMG")
[ "$DEV_BYTES" -gt "$IMG_BYTES" ] || die "$DEV is smaller than the image"

SERIAL=$(lsblk -dno SERIAL "$DEV" | xargs)
[ -n "$SERIAL" ] || die "$DEV reports no serial number; refusing"

echo
lsblk -o NAME,TRAN,RM,SIZE,MODEL,SERIAL "$DEV"
echo
echo "EVERYTHING on $DEV will be destroyed."
read -r -p "Type the serial shown above to continue: " TYPED
[ "$TYPED" = "$SERIAL" ] || die "serial did not match; nothing was written"

dd if="$IMG" of="$DEV" bs=4M conv=fsync oflag=direct status=progress

# Move the backup GPT to the end of the stick and grow partition 2.
sfdisk --relocate gpt-bak-std "$DEV"
echo ", +" | sfdisk --no-reread -N 2 "$DEV"
partx -u "$DEV" 2>/dev/null || blockdev --rereadpt "$DEV"
udevadm settle

P2="${DEV}2"
e2fsck -fy "$P2" || [ $? -le 1 ]
resize2fs "$P2"
tune2fs -U random "$P2"
sync

echo
lsblk -o NAME,SIZE,FSTYPE,LABEL,UUID "$DEV"
echo "Done. ATECH-DATA UUID: $(blkid -s UUID -o value "$P2")"
