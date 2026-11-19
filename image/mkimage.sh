#!/bin/bash
# Builds the USB disk image from the live ISO. Works on regular files only;
# needs no root. Layout (GPT):
#   1  ATW-BOOT    FAT32 EFI system partition holding the ISO's files
#   2  ATECH-DATA  ext4 data partition (small; make-usb.sh grows it)
# Usage: mkimage.sh <atw-live.iso> <out.img>
set -euo pipefail

ISO=$1
IMG=$2
DATA_MB=64
for f in "$ISO" "$IMG"; do
	if [ -e "$f" ] && [ ! -f "$f" ]; then
		echo "refusing: $f is not a regular file" >&2
		exit 1
	fi
done

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

xorriso -osirrox on -indev "$ISO" -extract / "$WORK/iso" 2>/dev/null
chmod -R u+w "$WORK/iso"
# FAT has no symlinks; drop any (live-build ISOs use none that boot needs).
find "$WORK/iso" -type l -print -delete

ISO_MB=$(du -sm --apparent-size "$WORK/iso" | cut -f1)
BOOT_MB=$(( ISO_MB + ISO_MB / 10 + 64 ))

# Partition 1: FAT32 with the ISO contents.
mkfs.vfat -F 32 -n ATW-BOOT -C "$WORK/p1.img" $(( BOOT_MB * 1024 )) >/dev/null
find "$WORK/iso" -mindepth 1 -maxdepth 1 -exec mcopy -s -Q -i "$WORK/p1.img" {} ::/ \;

# Partition 2: ext4 ATECH-DATA with the expected folders.
mkdir -p "$WORK"/skel/{keys,queue,sent,pdf,probe}
truncate -s "${DATA_MB}M" "$WORK/p2.img"
mkfs.ext4 -q -F -L ATECH-DATA -d "$WORK/skel" "$WORK/p2.img"

# Assemble: 1 MiB lead-in, the two partitions, 1 MiB for the backup GPT.
rm -f "$IMG"
truncate -s $(( (1 + BOOT_MB + DATA_MB + 1) ))M "$IMG"
sfdisk -q "$IMG" <<EOT
label: gpt
start=2048, size=$(( BOOT_MB * 2048 )), type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B, name="ATW-BOOT"
size=$(( DATA_MB * 2048 )), type=0FC63DAF-8483-4772-8E7D-693D4765C9E7, name="ATECH-DATA"
EOT
dd if="$WORK/p1.img" of="$IMG" bs=1M seek=1 conv=notrunc status=none
dd if="$WORK/p2.img" of="$IMG" bs=1M seek=$(( 1 + BOOT_MB )) conv=notrunc status=none
echo "wrote $IMG (${BOOT_MB} MiB boot + ${DATA_MB} MiB data)"
