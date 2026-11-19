# atw: AccessibleTech Wipe

A Rust tool on a Debian live USB. It purges laptop drives (NIST SP 800-88 Rev 2, Purge), checks the result by reading back every sector, signs each record (Ed25519), queues records on the USB, and syncs them to Supabase.

- Spec (source of truth): `docs/SPEC.md`
- Phase plan: `docs/PLAN.md`
- Resume state: `docs/PROGRESS.md`
- Design decisions and deviations from the spec: `docs/adr/`

## Safety: no exceptions

This repo builds software that destroys data. You run on Omar's dev machine (NixOS, dual-booted with Windows). Never put his data at risk.

- NEVER run a command that writes to, erases, sanitizes, formats, discards, partitions, or changes the security or power state of a real block device. That rules out: `hdparm --security-*`, `--sanitize-*`, `--dco-*`, `-N`; `nvme format`/`sanitize`; `mmc erase`/`sanitize`; `blkdiscard`, `wipefs`, `mkfs*`, `parted`, `sgdisk`, `dd of=/dev/*`; `rtcwake`; and anything that opens `/dev/sd*`, `/dev/nvme*` or `/dev/mmcblk*` for writing.
- Never run `image/make-usb.sh`, `tools/seed-and-check.sh`, `sudo`, or a rootful/privileged container yourself. Print the exact command for Omar to run instead.
- Destructive code only compiles with the cargo feature `live`, which is off by default. At runtime it also refuses unless all of these hold: the kernel cmdline contains `atw.live=1`; the target is not the boot device; the technician typed the confirmation.
- All drive commands go through the `CommandRunner` trait. Tests use `FakeRunner` with recorded fixtures. Test verification code against regular files, never devices.
- Never commit real private keys or secrets. Test keys live in `testdata/` and are named `*-TEST-ONLY*`.

## Token efficiency

- Never read `docs/SPEC.md` whole. List its sections with `grep -n '^##' docs/SPEC.md`, then read only the section you need, using offset/limit.
- In `docs/PLAN.md`, read only the current phase's section.
- Don't re-read a file you just wrote or edited. Make small targeted edits; don't rewrite whole files.
- Keep command output short: `cargo check -q`, `cargo test -q 2>&1 | tail -n 40`, `cargo clippy -q 2>&1 | head -n 60`. Send long builds (live-build, QEMU) to `build/logs/*.log` and show only `tail -n 30`, and only on failure.
- Use `rg` and `grep` directly. Don't spawn subagents for searches or simple tasks.
- Keep replies short: a plan in 15 lines or fewer, a summary in 10 lines or fewer. No pasting files or logs into chat.
- Pick small dependencies. Don't add a crate for something that takes under 50 lines.
- Use `git log --oneline -20` and `git diff --stat` to orient yourself instead of re-reading files.
- At every STOP point, rewrite `docs/PROGRESS.md` (40 lines max: phase, what works, what doesn't, next step, open questions) so a fresh session can resume without exploring the code.
- If the spec is ambiguous, contradicts itself, or doesn't match reality, stop and ask Omar. Don't guess.

## Stack

- Rust stable, Cargo workspace in `crates/`, built as a static binary for `x86_64-unknown-linux-musl`. Use `rustls` everywhere, never OpenSSL.
- Suggested crates: `ed25519-dalek`, `serde`/`serde_json` plus an RFC 8785 canonicaliser (checked against the RFC test vectors), `argon2`, `chacha20poly1305`, `ulid`, `qrcode`, a lightweight PDF crate (no headless browser, no typst), `ratatui`, `reqwest` (rustls), `thiserror`/`anyhow`.
- Boot image: current Debian stable built with `live-build`, run inside a Podman container (`image/Containerfile`). Secure Boot through `shim-signed`, `grub-efi-amd64-signed` and Debian's signed kernel.
- Server: Supabase migrations and a Deno Edge Function in `supabase/`. The Next.js pages go in the separate store repo; ask Omar for its path before touching it.
- Dev shell: `flake.nix` with the Rust toolchain plus the musl target, podman, qemu, and OVMF firmware with Secure Boot support.

## Git workflow

- Work on a branch for each phase: `phase-<N>-<slug>`, e.g. `phase-1-boot-spike`. Create it from `main` at the start of the phase. Never commit straight to `main`.
- **Commit at every passing step:** each time a change compiles, `cargo fmt` has been run, and the relevant tests pass. Don't let more than one task's worth of work go uncommitted. Also commit at every STOP.
- Never commit code that's broken or doesn't compile. If you have to save work in progress, use `git stash`, not a commit.
- Use conventional commits (`feat(drive): parse NVMe sanitize capabilities`, `test(verify): …`, `docs(manual): …`). Keep the subject to 72 characters or fewer. Only add a body when the reason isn't obvious from the diff.
- Commit a change and its documentation update together.
- Push after each commit: `git push -u origin <phase branch>`. Never push to `main`, never force-push, never rewrite pushed history. Omar merges each phase into `main` himself after review.
- If there's no `origin` remote, or a push fails, tell Omar and carry on committing locally. Don't try to fix the remote or the credentials yourself.
- Before every STOP: `cargo fmt`, `cargo clippy -- -D warnings` clean, `cargo test -q` passing, `cargo doc --no-deps` building with no warnings.

## Documentation

Docs are part of every change, not a separate task at the end. Update only the sections a change affects, and keep each one short and specific.

| File | Audience | Contents |
| --- | --- | --- |
| Rustdoc (`///`, `//!`) | Developers | Every public item: what it does, its errors, and a `# Safety` section for anything that touches a device. Each crate's `lib.rs` opens with a `//!` overview. Set `#![warn(missing_docs)]` in every crate. |
| `README.md` | Anyone opening the repo | What atw is and isn't (self-attested, not third-party certified), a status table by phase, quick start, links to the other docs. Under 80 lines. |
| `docs/DEVELOPMENT.md` | Developers | Dev shell, build, test, building the image in Podman, QEMU testing, repo layout, how the `live` feature and runtime safety checks work, how to add a drive fixture. |
| `docs/TECHNICIAN-MANUAL.md` | Omar and his dad at the bench | Numbered steps in plain words: preparing a laptop, booting the USB (Secure Boot on), entering the PIN, intake ID, confirming the wipe, reading the results, what each failure message means and what to do, Apple manual erase, sync and certificate issue, what to do if a USB is lost. No jargon without a one-line explanation. |
| `docs/ARCHITECTURE.md` | Developers and reviewers | How the crates, boot image, USB data partition, Supabase and the store fit together, with a Mermaid diagram. The data flow from wipe to verified certificate. The trust model: what is signed, by which key, and what the server checks. |
| `docs/adr/NNNN-title.md` | Future maintainers | One ADR (Architecture Decision Record) per design decision or deviation from the spec: Context, Decision, Consequences, 20 lines or fewer. Numbered in order and never edited after the fact; replace an old one with a new one that says it supersedes it. ADR 0001 records the decisions already made in `SPEC.md`'s Decisions log. |

- Write user-facing docs (README, manual) in Australian English.
- Code examples in the docs must be real commands that have been run, or be marked as untested.
- Fix a doc that has drifted from the code in the same commit that caused the drift.
