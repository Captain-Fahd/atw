# 0002: USB layout is a GPT disk image, not a hybrid ISO or lb hdd image

Status: accepted (20 Nov 2026).

## Context
The stick needs a second, writable ext4 partition labelled `ATECH-DATA` that `make-usb.sh` can add reliably. PLAN.md offered a hybrid ISO or live-build's `hdd` image.
- Hybrid ISO: the ISO9660/MBR/GPT overlay is read-only and awkward to extend; adding or growing a partition after `dd` is fragile.
- live-build `hdd`: needs loop devices (root) at build time, and its UEFI Secure Boot support is less exercised than the ISO path.

## Decision
A third option. live-build makes the usual `iso-hybrid` (well-trodden Secure Boot path: shim-signed → grub-efi-amd64-signed → signed kernel). `image/mkimage.sh` then builds a plain GPT disk image file, rootless: partition 1 FAT32 ESP `ATW-BOOT` holding the ISO's files, partition 2 ext4 `ATECH-DATA` (64 MiB). `make-usb.sh` writes it, moves the backup GPT, grows partition 2 to fill the stick and gives it a fresh UUID.

## Consequences
- What QEMU boots (`build/atw-usb.img`) is byte-for-byte what goes on the stick.
- No root or loop devices at build time; only `make-usb.sh` needs sudo.
- FAT can't hold symlinks (none are needed to boot) and files are capped at 4 GiB (squashfs is far smaller).
- Each stick's ATECH-DATA UUID is unique, so the boot device can be identified by label and UUID.
