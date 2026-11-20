# Progress

**Phase:** 1 (boot image and probe spike), at **STOP 1**. Branch `phase-1-boot-spike`.

## Works
- Dev shell (`flake.nix`): Rust 1.100 + musl, Podman, QEMU, OVMF (MS keys), e2fsprogs, jq, socat, shellcheck.
- `atw-drive`: `CommandRunner`, read-only `SystemRunner` (allowlist in `safety.rs`), `FakeRunner`; lsblk parsing and boot-USB exclusion (label `ATECH-DATA` or `/run/live/medium`); host facts (Secure Boot, lockdown, DMI, `atw.live=1`); probe.
- `atw probe` (also the default with no args): summary on screen, JSON (`atw-probe/1`, raw tool output kept) to `/mnt/data/probe/<vendor-product>-<ts>.json`.
- Image: `image/build.sh` builds in rootless Podman (`--cap-add SYS_ADMIN,MKNOD` inside the userns, no devices). Debian 13 trixie, live-build 20250505, shim-signed + grub-efi-amd64-signed + signed 6.12 kernel, all non-free firmware.
- USB layout: GPT image, FAT32 ESP + ext4 ATECH-DATA (ADR 0002). `make-usb.sh` written, **not run** (Omar runs it).
- QEMU (`image/qemu-test.sh`, rootless, KVM): boots with Secure Boot on; lockdown `integrity`; `hdparm -I` (SATA) and `nvme id-ctrl/id-ns` both succeed; boot USB excluded.
- Checks: fmt, clippy `-D warnings`, 17 tests (incl. the QEMU probe fixture), `cargo doc` clean.

## Doesn't / not yet
- eMMC path untested (QEMU has no eMMC). `mmc extcsd read` only parsed by a smoke check.
- No parser fixtures from real hardware yet (only `testdata/fixtures/probe-qemu-q35-secureboot.json`).
- `make-usb.sh` untested on a real stick. It requires `RM=1`; some sticks report `RM=0` and will be refused.
- Only `atw-drive` and `atw` crates exist; others come in later phases.

## Next step (Omar)
1. `nix develop`, `image/build.sh > build/logs/image-build.log 2>&1`
2. `sudo image/make-usb.sh /dev/sdX`
3. Boot real laptops with Secure Boot on; copy `probe/*.json` to `docs/hw/phase1/`. Stick made 21 Nov (make-usb.sh worked).
   Omar has only 2 NVMe laptops: SATA and eMMC hardware runs are pending (probe them if one comes through intake). STOP 2 needs ≥2 drives per media type.

Next session: analyse those JSONs; if lockdown blocks any tool, stop and set out options. Save real outputs as fixtures in `testdata/fixtures/`.

## Open questions
- Is granting `CAP_SYS_ADMIN` inside the rootless build container OK with you? (It isn't `--privileged` or rootful.)
- Fail-safe GRUB entry has no `atw.live=1`, so it can never wipe. OK?
