# atw build plan

Work one phase per session, on its own branch (`phase-<N>-<slug>`). Commit and push at every passing step (see CLAUDE.md, Git workflow).

Each phase ends at a **STOP**:

1. Run the checks listed in CLAUDE.md.
2. Make sure the docs the phase lists are up to date.
3. Update `docs/PROGRESS.md`.
4. Commit and push.
5. Tell Omar exactly what to run or test, then wait.

Never start the next phase without his go-ahead.

## Process flow (text version of the SPEC diagram)

1. Intake ID entered
2. Boot the USB (Secure Boot on)
3. Unlock the signing key with the PIN
4. Detect internal drives (excluding the boot USB)
5. Prepare each drive: hidden areas (HPA/DCO), frozen state, locks
6. Purge with the strongest supported method
7. Full read-back verification
8. On pass, sign the record and build a draft PDF. On failure (no Purge method, or verification fails twice), sign a failure record and issue no certificate.
9. Queue the record on the USB
10. When online, sync to the Edge Function, which checks the signature and the key's status
11. The admin page issues the final certificate after sync
12. The public QR page shows the record's status

## Repo layout (target)

```
crates/atw-core      record schema, canonical JSON, signing, key file crypto
crates/atw-drive     CommandRunner, detection, capability parsing, prepare, purge
crates/atw-verify    fingerprinting, full read-back, pass/fail rules
crates/atw-pdf       certificate PDF, QR code, embedded record
crates/atw           binary: TUI, subcommands (probe, wipe, sync, report)
crates/atw-provision binary run on Omar's machine: key generation and registration
image/               Containerfile, live-build config, build.sh, make-usb.sh, qemu-test.sh
supabase/            migrations, functions/submit-wipe-record, tests
testdata/            fixtures (tool outputs), vectors (canonical JSON + signatures), TEST-ONLY keys
tools/               seed-and-check.sh (Omar runs it on hardware)
README.md
docs/                SPEC.md, PLAN.md, PROGRESS.md, DEVELOPMENT.md, ARCHITECTURE.md,
                     TECHNICIAN-MANUAL.md, adr/, hw/phaseN/ (Omar's results)
```

---

## Phase 1: Boot image and probe spike

**Goal:** prove that a Debian live USB boots with Secure Boot on and that the drive tools can talk to drives under kernel lockdown. Nothing destructive happens in this phase.

1. Bootstrap the repo: `flake.nix`, the workspace skeleton, the `CommandRunner` trait with `SystemRunner` and `FakeRunner`, a `.gitignore` (`target/`, `build/`, `*.key` except `testdata/*-TEST-ONLY*`), and the first versions of the docs:
   - `README.md`
   - `docs/DEVELOPMENT.md`
   - `docs/ARCHITECTURE.md`
   - `docs/TECHNICIAN-MANUAL.md` (headings only for now)
   - `docs/adr/0001-initial-decisions.md`
   - `docs/PROGRESS.md`
2. Build the image: `image/Containerfile` (Debian stable plus live-build) and `image/build.sh`, which builds inside the container and writes `build/atw-live.*`. Include:
   - Boot chain: `shim-signed`, `grub-efi-amd64-signed`, `linux-image-amd64`
   - Firmware from non-free-firmware for common laptop Wi-Fi, storage and GPUs
   - Tools: `hdparm`, `nvme-cli`, `mmc-utils`, `smartmontools`, `network-manager` (`nmtui`), `mokutil`
   - Kernel cmdline: `atw.live=1`
   - Root autologin on tty1 that starts `/usr/local/bin/atw`
   - The `atw` binary, copied in from the musl build
3. Data partition: decide between a hybrid ISO and an `hdd` image, depending on which lets `make-usb.sh` add an ext4 partition labelled `ATECH-DATA` reliably. Record the choice as an ADR. `make-usb.sh <device>` must refuse unless the target is a removable USB device (`lsblk -o TRAN,RM`), and must make Omar type the device's serial to confirm. **Omar runs it, not you.**
4. Add `atw probe`, which is read-only. It reports:
   - The lockdown state (`/sys/kernel/security/lockdown`) and the Secure Boot state
   - For each internal disk: transport, model, serial, size
   - The output of the read-only identify command for each drive type (`hdparm -I`, `nvme id-ctrl`/`id-ns`, `mmc extcsd read`)
   - Whether the ATA passthrough identify succeeded
   It writes JSON to `ATECH-DATA/probe/<host>-<ts>.json`.
5. `image/qemu-test.sh`: boots the image in QEMU with OVMF Secure Boot (Microsoft keys enrolled), an emulated NVMe disk and an emulated SATA disk, all file-backed. Runs probe and saves its JSON. Tell Omar if this needs root.

**Docs:**
- `DEVELOPMENT.md`: building the image and testing in QEMU
- `TECHNICIAN-MANUAL.md`: flashing the USB and running probe
- `ARCHITECTURE.md`: the boot chain and the USB layout

**Done when:** the image builds in the container; it boots in QEMU with Secure Boot on; probe reports both emulated disks; all the checks in CLAUDE.md pass.

**STOP 1:** Omar flashes a USB and runs probe on at least 3 real laptops (NVMe, SATA, and eMMC if available), then puts the JSONs in `docs/hw/phase1/`. Next session, analyse them first. If lockdown blocks passthrough or any tool, stop and set out options before Phase 2. Save each tool's real output as a parser fixture in `testdata/fixtures/`.

---

## Phase 2: Wipe engine

Read SPEC sections "Drive handling by media type", "Verification" and "Failures and exceptions".

1. **Detect:** media type, identity, native vs reported capacity, and supported sanitize features. Parse from the fixtures. Exclude the boot device by the USB's label and UUID.
2. **Prepare:** detect and remove HPA/DCO (recording before and after); detect a frozen drive and unfreeze it with a suspend/resume cycle (`rtcwake`); detect ATA security and Opal locks and fail with PSID-revert guidance.
3. **Purge:** use the method table from the spec, strongest method first, and never fall back to Clear. An HDD with no Purge method fails with "replace drive". The ATA temporary password is set and cleared but never stored. Poll sanitize progress. If a sanitize was interrupted, detect it, resume or wait, and record that it happened.
4. **Verify:** before the purge, take SHA-256 fingerprints of 4,096 evenly spaced sectors and scan for filesystem signatures. After it, read the full native capacity with `O_DIRECT` and large aligned buffers, showing progress and throughput. Pass rules depend on the method family: a uniform pattern, or the crypto-erase check. Retry the purge once; a second failure produces a failure record.
5. **TUI:** enter the intake ID; show the drive list; require the typed confirmation `WIPE <last 4 of serial>`; show progress and the result. PIN unlock is a placeholder until Phase 3.
6. **Tests:**
   - Method selection, run as a table covering every media type and capability combination
   - Parsers, against the Phase 1 fixtures
   - Verify, on temp files: all zeros, all 0xFF, random data, and random data with a leftover marker (must fail)
   - A full run through `FakeRunner`
7. `tools/seed-and-check.sh <device>` (**Omar runs it**). Before the wipe it writes marker files and a repeating pattern. After the wipe it scans the raw device for the markers and runs `photorec`. It prints PASS or FAIL.

**Docs:**
- `TECHNICIAN-MANUAL.md`: the full wipe procedure, plus a table of every failure message with its meaning and what to do
- `ARCHITECTURE.md`: the method-selection flow
- `DEVELOPMENT.md`: adding drive fixtures, and running the seed test
- An ADR for any change to the method order

**STOP 2:** Omar wipes sacrificial drives (at least 2 per media type) and runs the seed test, then puts the results in `docs/hw/phase2/`. The phase passes only if no markers are recovered on any media type.

---

## Phase 3: Records and certificates

Read SPEC sections "Wipe record and certificate" and "Signing and key management".

1. **Record:** a versioned schema (`schema_version`) with every field from the spec table; ULID IDs; RFC 8785 canonical JSON; an Ed25519 signature; `record_sha256`. Failure records use the same schema with `result: fail`.
2. **Test vectors:** in `testdata/vectors/`, each with the record, its canonical bytes in hex, the signature, and a TEST-ONLY key pair. Rust and the Deno Edge Function both test against these vectors.
3. **`atw-provision`** (run on Omar's machine): generates the key pair, asks for the PIN, encrypts the private key (Argon2id at m=64 MiB, t=3, p=1, then XChaCha20-Poly1305), writes `keys/usb-NN.key` to the mount path it's given, and prints the `key_id` and public key. Registration with Supabase is added in Phase 4.
4. **Unlock at boot:** the PIN decrypts the key into memory only. After 5 wrong attempts, write a lockout marker that only reprovisioning clears.
5. **PDF:** one A4 page laid out as in the spec. It carries a QR code linking to `https://accessibletech.com.au/verify/<certificate_id>`, the record JSON and signature as an embedded attachment, the record fingerprint in the footer, and the watermark "DRAFT: not valid until issued". There is no Adobe-level PDF signature (that's option C, deferred).
6. **Queue:** `queue/` and `sent/` folders. Write every file atomically: temp file, fsync, rename.
7. **Tests:** changing any byte of a record makes verification fail; all vectors pass; the PDF parses and its attachment can be extracted; a text snapshot of the PDF matches.

**Docs:**
- `ARCHITECTURE.md`: the trust model (keys, signatures, what tampering breaks)
- `TECHNICIAN-MANUAL.md`: provisioning a USB, the PIN and lockout, and what to do if a USB is lost
- `DEVELOPMENT.md`: regenerating the test vectors
- An ADR for the record schema version 1

**STOP 3:** commit a sample PDF to `docs/samples/` for Omar to review.

---

## Phase 4: Sync, Supabase and verification page

Read SPEC sections "Sync and public verification" and "Record retention".

1. **Migrations:**
   - Tables: `signing_keys`, `wipe_records`, `intakes`, `apple_records`, `deletion_log`
   - Private storage buckets: `certificates`, `apple-evidence`
   - RLS: anonymous users can't read or write any table directly
   - `get_public_certificate(id)` (SECURITY DEFINER), which returns only the public fields plus the status (verified, not synced, key revoked, invalid)
2. **Edge Function `submit-wipe-record`:** reject unknown or revoked keys; check the signature using the same canonicalisation and the shared vectors; insert idempotently on `certificate_id`; store the server's `received_at`. Allow only expected payload sizes, and rate-limit by key.
3. **Before building certificate issuance, STOP and ask.** The spec says the server generates the final PDF. The alternative is for the server to check the PDF uploaded by the USB (that its embedded record matches the signed record) and store it as issued. Set out the trade-offs in 5 lines or fewer and wait for Omar's choice.
4. **Rust `atw sync`:** upload the queue when there's a network connection, with exponential backoff, and move a record to `sent/` only after a confirmed 2xx response. Add a `provision --register` step that registers the public key, using an admin token Omar supplies at runtime and never stores.
5. **Retention:** a scheduled job (`pg_cron`) that applies the spec's retention table and writes to `deletion_log` (ID and date only).
6. **Store repo (ask Omar for its path and a branch first):**
   - `/verify/[certificate_id]`: rendered on the server, re-checks the signature, shows four states
   - Admin page: records list and issued-PDF download; Apple record form with photo upload; register and revoke keys
7. **Tests:** Deno tests against the vectors; SQL tests that anonymous users can't read the tables; an end-to-end run of `atw sync` against local `supabase start`.

**Docs:**
- `ARCHITECTURE.md`: the Supabase schema, RLS and sync sequence, with a Mermaid diagram
- `DEVELOPMENT.md`: running Supabase locally and the deploy commands
- `TECHNICIAN-MANUAL.md`: syncing, issuing certificates from the admin page, recording Apple devices, revoking a key
- An ADR for the certificate-issuance choice
- Matching docs in the store repo for its pages

**STOP 4:** you don't deploy. Give Omar the exact `supabase` commands. Omar deploys and checks every state on the verification page.

---

## Phase 5: Pilot support

1. Embed the tool version and git commit at build time. At boot, compute a hash of the image (the squashfs) once and include it in every record.
2. `atw report`: a CSV summary of records, for the pilot comparison against BitRaser.
3. Draft `docs/Media-Sanitisation-Procedure.md` from the spec: roles, method selection, failure handling, drive replacement and destruction, key handling, retention. Omar reviews it; it isn't final.
4. Fix whatever the pilot turns up.
5. Docs pass: check every document against the code and remove anything stale. Bring `TECHNICIAN-MANUAL.md` to the point where someone new could follow it without help. Update the status table in `README.md`. Add a `CHANGELOG.md` built from the commit history, grouped by phase.

**STOP 5:** go-live is Omar's decision. Don't change any marketing or store copy.
