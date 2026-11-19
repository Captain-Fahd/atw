# 0001: Initial decisions

Status: accepted (19 Nov 2026). Source: `docs/SPEC.md`, Decisions log.

## Context
atw replaces BitRaser for AccessibleTech's own stock. Two-person business, mostly working x86_64 UEFI laptops.

## Decision
- Boot USB only (Debian live, Secure Boot on via Debian's Microsoft-signed shim). No bench mode, no Windows.
- Media in scope: SATA HDD, SATA SSD, NVMe, eMMC. Laptops that won't boot are repaired first.
- NIST SP 800-88 Rev 2 Purge only, with the drive's own sanitize commands; never Clear. HDD with no Purge method: replace and destroy it.
- Full read-back of every sector. Rust. v1 calls `hdparm`, `nvme-cli`, `mmc-utils`; native ioctls later.
- Certificate: PDF carrying the signed record plus a QR code checked against Supabase; no Adobe PDF signature in v1. Issued only after sync, from the admin page. IDs are intake IDs, no names.
- Offline queue on the USB; Ed25519 key per USB, PIN-unlocked, revocable in Supabase. One shared technician ID.
- Apple devices: manual Apple erase, recorded on the admin page.

## Consequences
Certificates are self-attested, not third-party certified. Lockdown under Secure Boot must allow the drive tools (tested in Phase 1).
