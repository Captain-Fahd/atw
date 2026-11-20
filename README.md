# atw: AccessibleTech Wipe

atw is a tool on a bootable USB stick that securely erases the internal drives of used laptops. It uses each drive's own sanitise commands (NIST SP 800-88 Rev. 2, Purge), reads every sector back to check the result, and produces a signed record that can be checked online through a QR code.

**What it isn't:** atw's certificates are self-attested by AccessibleTech. The tool hasn't been certified by a third party (as BitRaser or Blancco are), and NIST doesn't certify tools.

## Status

| Phase | What | Status |
| --- | --- | --- |
| 1 | Boot image and read-only probe | In progress |
| 2 | Wipe engine (detect, prepare, purge, verify) | Not started |
| 3 | Signed records and PDF certificates | Not started |
| 4 | Sync, Supabase and verification page | Not started |
| 5 | Pilot support | Not started |

## Quick start

```sh
nix develop
cargo test -q
image/build.sh          # builds build/atw-usb.img in rootless Podman
image/qemu-test.sh      # boots it in QEMU with Secure Boot on
sudo image/make-usb.sh /dev/sdX   # untested by Claude: writes a real stick (asks for its serial)
```

## Docs

- [Spec](docs/SPEC.md) and [build plan](docs/PLAN.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Development](docs/DEVELOPMENT.md)
- [Technician manual](docs/TECHNICIAN-MANUAL.md)
- [Decision records](docs/adr/)
- [Progress](docs/PROGRESS.md)
