# Development

## Dev shell

`nix develop` gives the Rust toolchain (with the `x86_64-unknown-linux-musl` target), Podman, QEMU, OVMF (Secure Boot, Microsoft keys), e2fsprogs, jq, socat and shellcheck. Every command below assumes you're in it.

## Build and test

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test -q
cargo doc --no-deps
cargo build --release --target x86_64-unknown-linux-musl   # static binary for the USB
```

## Repo layout

| Path | What |
| --- | --- |
| `crates/atw-drive` | `CommandRunner`, safety allowlist, lsblk, host facts, probe |
| `crates/atw` | The binary on the USB (`atw probe`) |
| `image/` | Containerfile, live-build config (`live/config`), `build.sh`, `mkimage.sh`, `make-usb.sh`, `qemu-test.sh` |
| `testdata/fixtures/` | Real tool output used by parser tests |
| `docs/` | Spec, plan, progress, ADRs, manuals |

More crates (`atw-core`, `atw-verify`, `atw-pdf`, `atw-provision`) arrive in later phases.

## Safety: how destructive code is kept out

- Every external command goes through `CommandRunner`. Tests use `FakeRunner`, which replays recorded output.
- `SystemRunner::read_only()` refuses any command not on the allowlist in `atw-drive/src/safety.rs` (currently `lsblk`, `hdparm -I`, `nvme id-ctrl|id-ns -o json`, `mmc extcsd read`).
- Destructive code will only compile with the cargo feature `live` (off by default; currently gates nothing). At runtime it will also need `atw.live=1` on the kernel command line, a target that isn't the boot USB, and the technician's typed confirmation.
- Verification code is tested on regular files, never devices.

## Building the image

```sh
image/build.sh > build/logs/image-build.log 2>&1
```

It builds the musl binary, then the builder image (`image/Containerfile`, Debian trixie plus live-build), then runs `image/lb-build.sh` inside it. This needs **rootless** Podman only; the container gets `CAP_SYS_ADMIN` inside its own user namespace so live-build can mount `/proc` in its chroot. No host devices are passed in. The live-build work dir and package cache live in the Podman volume `atw-lb-work`.

Outputs: `build/atw-live.iso`, `build/atw-usb.img` (the stick layout, see ADR 0002) and `build/SHA256SUMS`.

`image/policy.json` is the container signature policy (NixOS has no `/etc/containers/policy.json` by default). It only accepts `docker.io/library/debian` and local images.

## Testing in QEMU

```sh
image/qemu-test.sh
```

Boots `build/atw-usb.img` as a USB stick under OVMF with Secure Boot on and Microsoft keys enrolled, plus a 4 GB NVMe disk and a 4 GB SATA disk backed by files in `build/qemu/`. It waits for `ATW-PROBE-DONE` on the serial port, pulls the probe JSON out of the ATECH-DATA partition with `debugfs` (no mounting), and checks Secure Boot, both disks and passthrough. No root needed; uses KVM if `/dev/kvm` is writable. Serial log: `build/qemu/serial.log`.

## Adding a drive fixture

Probe JSON keeps every tool's raw output (`disks[].identify[].stdout`, `lsblk_raw`). To add a fixture from a hardware run in `docs/hw/phaseN/` (untested until real results arrive):

```sh
jq -r '.disks[] | select(.internal) | .identify[0].stdout' docs/hw/phase1/<file>.json > testdata/fixtures/<tool>-<model>.txt
```

Name fixtures `<tool>-<subcommand>-<vendor>-<model>.txt` and add a parser test that loads them.
