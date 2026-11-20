# AccessibleTech Wipe Tool: Architecture & Spec

Exported 19 Nov 2026 from the claude.ai spec doc. Diagrams have been replaced with text versions.

## Summary and scope

AccessibleTech Wipe is a Rust program on a Debian live USB. It purges a laptop's internal drive using the drive's own sanitize commands, then reads the whole drive back to check it. If the check passes, it issues a signed PDF certificate whose QR code can be checked on accessibletech.com.au. It is meant to replace BitRaser for AccessibleTech's own stock.

**In scope for v1**

- Working x86_64 UEFI laptops, booted from the USB with Secure Boot left on
- Internal SATA HDD, SATA SSD, NVMe SSD and eMMC storage
- Full read-back verification, signed records, PDF certificates, offline queue, Supabase sync, public verification page
- Apple devices: a log of the manual Apple erase and a certificate for it (the tool does not wipe them itself)

**Out of scope for v1**

- Bench mode for drives taken out of laptops, and any Windows version
- Laptops that don't boot: they are repaired first, then wiped through the USB
- Physical destruction, phones, removable SD cards

The certificates are self-attested. No third party has certified the tool, unlike BitRaser or Blancco. See Limitations and risks.

## Decisions log

All decisions made, 17–19 Nov 2026.

| Area | Decision | Why |
| --- | --- | --- |
| Role | Full replacement for BitRaser | Owner decision; see Limitations on third-party certification |
| How it runs | Boot USB only; no bench mode, no Windows | Intake is almost all working laptops; Windows restricts drive erase commands |
| Boot base | Debian live | Only option tested that boots with Secure Boot on without changing each laptop's firmware |
| Secure Boot | Left on; boots through Debian's Microsoft-signed shim | No BIOS changes on each laptop |
| Media | SATA HDD, SATA SSD, NVMe, eMMC | Covers the laptops AccessibleTech takes in |
| Method | Purge only, using the drive's own sanitize commands | Rev 2 warns that Clear can leave data recoverable on modern media |
| HDD with no Purge command | Replace the drive; destroy the old HDD and record it as destroyed | Every certificate stays Purge; no second, weaker certificate |
| Verification | Full read-back of every sector | Easiest result to defend |
| Language | Rust | Owner decision |
| Drive command layer | Call `hdparm`, `nvme-cli` and `mmc-utils` in v1; native ioctls later | Proven tools, faster to build |
| Certificate | PDF carrying the signed record, plus a QR code checked against Supabase (option A); no Adobe-level PDF signature in v1 | Free and works offline; add a certificate on Adobe's trusted list (option C), applied on the server, if SME clients ask |
| Certificate issue | Only after sync, downloaded from the admin page | Every certificate handed out verifies |
| Certificate IDs | Intake ID only; no personal names | The verification page is public; keeps names out of it under the APPs |
| Sync | Queue on the USB, upload when online | Wipes must not depend on having internet |
| Signing key | Separate Ed25519 key per USB, revocable in Supabase | A lost USB can be cut off without affecting the others |
| Key protection | Technician PIN at boot unlocks the key | A copied USB can't sign without the PIN |
| Technician identity | One shared ID | Two-person business. Each record still shows which USB key signed it. Revisit if staff are added, since Rev 2 expects clear roles |
| Record retention | As law and standards require; stored in Supabase Storage | See Record retention |
| Standard cited | Rev 2, with a note that the method also meets Rev 1 Purge | Many buyer checklists still quote Rev 1 |
| Apple | Technician does the Apple erase; records entered on the store's admin page | A third-party program can't purge iPads or T2/Apple Silicon Macs |
| Laptops that won't boot | Repair first, then wipe through the USB | Owner decision |

## Standards basis

The tool targets NIST SP 800-88 Rev 2 Purge. Rev 2 was published on 26 Sep 2025 and replaced Rev 1 from 2014 ([NIST](https://csrc.nist.gov/pubs/sp/800/88/r2/final)).

- **Three methods:** Rev 2 keeps Clear, Purge and Destroy. It warns that Clear on modern magnetic and flash media can leave data recoverable ([BitRaser summary](https://www.bitraser.com/article/nist-800-88-r2)). The tool therefore uses Purge wherever the drive supports it. Clear is never certified: an HDD without Purge support gets its drive replaced.
- **Program, not just a tool:** Rev 2 moves the focus from specific techniques to running an organisation-wide sanitisation program ([NIST draft notes](https://csrc.nist.gov/pubs/sp/800/88/r2/ipd)). AccessibleTech needs a short written Media Sanitisation Procedure to go with the tool: roles, method choice, what happens when verification fails, how long records are kept.
- **Technique detail:** Rev 2 refers to outside standards such as ISO/IEC 27040 for how to sanitise each media type. Check the exact clause references when writing the procedure.
- **Certificate wording:** "Sanitised in accordance with NIST SP 800-88 Rev. 2, Purge. The method used also corresponds to Purge under the withdrawn Rev. 1."

The certificate says the sanitisation was done *in accordance with* the standard. It never says the tool is *NIST certified*: NIST does not certify tools.

## End-to-end process

Every step runs on the laptop itself, booted from the USB.

1. **Intake:** log the intake ID.
2. **Boot the wipe USB**, with Secure Boot on.
3. **Detect drives:** model, serial, and supported sanitize features.
4. **Prepare each drive:** hidden areas (HPA/DCO), frozen state.
5. **Purge** using the drive's own sanitize command.
   - If the drive has no Purge method, write a **failure record** and issue no certificate.
6. **Read-back passes?**
   - Yes: go to step 7.
   - No: retry the Purge once. If it fails again, write a **failure record** and issue no certificate.
7. **Sign the record** and build a draft PDF with its QR code.
8. **Queue on the USB.** This works offline.
9. **Sync when online**, to Supabase.
10. **The server checks** the signature and the key's status, then stores the record.
11. **The certificate is issued** from the admin page, after sync.
12. **The QR page shows "Verified"**, with public fields only.

## System architecture

Everything that touches the drive runs on the USB, with no network needed. Supabase stores only records whose signatures check out. The public page reads them through an RPC that returns the public fields and nothing else.

```
Wipe USB (Debian live, Secure Boot on)
├── atw: Rust wipe tool
│   ├── detect   drive identity
│   ├── prepare  hidden areas
│   ├── purge    sanitize commands
│   ├── verify   full read-back
│   ├── record   sign, make PDF
│   └── sync     upload queue  ──HTTPS──▶  Supabase: submit-wipe-record
├── Drive tools: hdparm, nvme-cli, mmc-utils ──ATA / NVMe / MMC commands──▶ Internal drive
└── ATECH-DATA partition: keys/, queue/, sent/, pdf/

Supabase (Singapore)
├── Edge Function submit-wipe-record: checks signature + key, then stores the record
├── Postgres tables: signing_keys, wipe_records, intakes (admin only)
└── RPC get_public_certificate: returns public fields only
        │
        ▼
accessibletech.com.au (Next.js, separate store repo)
├── /verify/[id]: shows status
└── admin: keys, Apple records, issued certificates
        ▲
Buyer scans the QR code
```

- **Boot chain:** Debian's Microsoft-signed shim, then GRUB, then the Debian kernel. Secure Boot stays on, which turns on kernel lockdown. The tool runs as a normal program and needs no custom kernel modules.
- **Image build:** Debian `live-build`, with the `atw` binary built in and started automatically on tty1, plus a separate ext4 data partition. Record the image hash in every certificate.
- **Rust crates (suggested):** `ed25519-dalek` (signing), `serde_json` plus an RFC 8785 canonicaliser, a lightweight PDF crate (PDF), `qrcode`, `reqwest` (sync), `ratatui` (on-screen interface).

## Drive handling by media type

The tool picks the strongest Purge method the drive reports it supports, in the order below. If the drive supports none of them, the wipe fails; the tool never quietly drops to a weaker method.

| Media | Linux interface | Purge order (first one supported) | If none supported |
| --- | --- | --- | --- |
| SATA HDD | SG_IO, ATA PASS-THROUGH (16) | ATA Sanitize Overwrite → ATA Security Erase (Enhanced, then Normal) | Fail; replace the drive and destroy the old one |
| SATA SSD | SG_IO, ATA PASS-THROUGH (16) | ATA Sanitize Crypto Scramble → ATA Sanitize Block Erase → ATA Security Erase Enhanced | Fail; storage can't be certified |
| NVMe SSD | NVMe admin passthrough ioctl | Sanitize Crypto Erase → Sanitize Block Erase → Format NVM, crypto (SES=2) → Format NVM, user data (SES=1); all namespaces | Fail |
| eMMC | MMC_IOC_CMD ioctl | Secure erase of the user area → eMMC Sanitize (eMMC 4.5 and later) | Fail |

**Steps before every Purge**

1. Read and record identity: model, serial, firmware, reported capacity, true native capacity, and which sanitize features the drive supports.
2. Hidden areas on SATA drives: detect HPA and DCO, record the size before and after, and restore the full native capacity.
3. Frozen SATA security: suspend to RAM and resume with `rtcwake` to unfreeze. If it's still frozen, fail with instructions.
4. ATA Security Erase needs a temporary password, which the tool sets and clears. The password is logged as "set by tool" but never stored.
5. Self-encrypting drives (Opal) that are locked with an unknown password: fail and prompt a PSID revert. The PSID is printed on the drive label.
6. Sanitize that was interrupted (power loss): read the sanitize status, let the drive finish, and record that it was interrupted.
7. Every internal drive is wiped. The boot USB is identified by its own label and UUID and always excluded.

For v1, the tool calls `hdparm`, `nvme-cli` and `mmc-utils` and reads their output. Native Rust ioctls can replace them later.

## Verification

The tool reads every sector across the drive's full native capacity. A wipe passes only if every sector reads without error and matches the expected result.

**Before the Purge:** the tool fingerprints the drive. It takes SHA-256 hashes of 4,096 evenly spaced sectors and records any filesystem signatures it finds (partition table, NTFS, ext4, APFS, BitLocker header).

**What counts as a pass depends on the method:**

| Method family | Expected read-back | Pass rule |
| --- | --- | --- |
| Overwrite or block erase (ATA Sanitize Overwrite/Block Erase, NVMe Block Erase, Format SES=1, eMMC sanitize) | One uniform value, usually all 0x00 or all 0xFF | Every sector equals that value |
| Crypto erase (ATA Crypto Scramble, NVMe Crypto Erase, Format SES=2) | Data that looks random | None of the fingerprinted sector hashes appear again, and no filesystem signatures are found anywhere |

Crypto-erase verification is a heuristic: it proves the old data is no longer readable, not that the key was destroyed. To make the result deterministic, prefer Block Erase over Crypto Erase on drives that support both.

**Fail rules:** any read error, any sector that doesn't match, or a reported capacity below the native capacity. On failure the tool retries the Purge once. If it fails again, it writes a signed **failure record** and issues no certificate.

**Approximate time for the read-back** (estimates; real speed depends on the drive): 512 GB NVMe about 5 min, 128 GB eMMC about 10 min, 1 TB HDD about 2 h.

## Wipe record and certificate

The signed JSON record is the source of truth. The PDF is built from it and carries a copy of the record and its signature, so the certificate can be checked without trusting the PDF itself.

| Field | Example | PDF | Public page |
| --- | --- | --- | --- |
| certificate_id (ULID) | 01JD8X… | Yes | Yes |
| intake_id | AT-2026-0142 | Yes | No |
| Device make, model, serial (from DMI) | Lenovo T480, PF1ABC23 | Yes | Model only |
| Drive model, serial, firmware | Samsung PM981, S4DY… | Yes | Yes |
| Media type and interface | NVMe SSD, PCIe | Yes | Yes |
| Capacity: reported and native | 512,110,190,592 bytes | Yes | No |
| HPA/DCO found and removed | none | Yes | No |
| Method used | NVMe Sanitize, Block Erase | Yes | Yes |
| NIST category and revision | Purge, SP 800-88 Rev 2 (also Rev 1 Purge) | Yes | Yes |
| Start and end time (UTC, shown in Melbourne time) | 2026-11-17 14:02 AEDT | Yes | Date only |
| Verification result | Full read-back, 0 mismatches | Yes | Yes |
| Technician | Shared technician ID | Yes | No |
| Tool version, git commit, boot image hash | 0.1.0, a1b2c3d | Yes | No |
| Signing key ID | usb-03 | Yes | Yes, with key status |
| Signature (Ed25519 over RFC 8785 canonical JSON) | base64 | Fingerprint only | Checked by the server |

**PDF layout:** one A4 page. Header with AccessibleTech details and ABN. A Result box (PASS, method, date). Device and drive tables. The verification summary. The standard statement. A QR code. A footer with the record fingerprint. The JSON record and signature are attached inside the PDF.

**QR target:** `https://accessibletech.com.au/verify/<certificate_id>`

If several internal drives are wiped, the certificate has one drive block for each.

## Signing and key management

Each USB gets its own Ed25519 key pair, locked with a technician PIN. The public key is registered in Supabase so that one lost USB can be revoked without affecting the others.

1. **Provisioning:** on Omar's machine, a `provision` command generates the key pair. It encrypts the private key with the technician PIN (Argon2id key derivation, XChaCha20-Poly1305), writes it to the USB's data partition, and registers the public key in Supabase as `usb-NN`.
2. **Unlocking:** at boot the technician enters the PIN. The key is decrypted into memory only and never written to disk unencrypted. After 5 wrong PINs, the tool locks until the USB is reprovisioned.
3. **Signing:** every record, pass or fail, is signed on the USB when it is created.
4. **Revocation:** marking a key `revoked` in Supabase sets `revoked_at`. Records the server *received* before that time stay valid. Records received after it show as invalid. The server uses its own receipt time because the time on a USB record can be faked.
5. **Rotation:** reprovision each USB yearly or when it changes hands; the old key is retired, not revoked.

The remaining risk: someone who has both a copy of the USB and the PIN can sign records until the key is revoked. Keep the PIN separate from the USB and change it when you reprovision.

## Sync and public verification

Records queue on the USB and upload to a Supabase Edge Function. The function checks each signature before storing anything, so the USB never holds a Supabase service key.

**On the USB:** an ext4 partition labelled `ATECH-DATA` holds `keys/`, `queue/` (records not yet sent), `sent/` and `pdf/`. Whenever there is a network connection, the tool tries to upload the queue. Each upload is retried until the server confirms it.

**Edge Function `submit-wipe-record`:** looks up the key, rejects it if unknown or revoked, checks the signature against the canonical JSON, then inserts the record. Sending the same `certificate_id` twice is treated as one upload.

| Table | Key columns | Access |
| --- | --- | --- |
| `signing_keys` | key_id, public_key, label, status, created_at, revoked_at | Admin only |
| `wipe_records` | certificate_id, key_id, intake_id, record (jsonb), signature, record_sha256, result, received_at | Insert through the Edge Function only; read through the RPC below |
| `intakes` | intake_id, source details, names and contacts | Admin only, never public |

**Verification page `/verify/[certificate_id]`:** a Next.js server route calls a `get_public_certificate` RPC. The RPC returns only the fields marked Public in the record table. The page checks the signature again on the server and shows one of:

- **Verified:** the signature is valid and the key was active when the record was received
- **Not yet synced:** no record found; the USB may not have uploaded it yet
- **Key revoked:** the record arrived after its key was revoked
- **Invalid:** the signature doesn't match the record

**Issuing certificates:** the PDF on the USB is only a draft. Once the server accepts a record, it generates the final PDF and stores it in the `certificates` bucket. The technician downloads it from the admin page, and only that PDF goes to the customer.

## Record retention

Wipe records and certificate PDFs are kept for 7 years in Supabase. Personal details from intake are kept only as long as the law requires, then deleted. This is a working position, not legal advice; confirm it with your lawyer alongside the Asset Exchange Agreement brief.

| Record | Kept for | Basis | Where |
| --- | --- | --- | --- |
| Wipe records (JSON) and certificate PDFs | 7 years from the wipe | In Victoria, contract and tort claims generally have 6 years, as do ACL misleading-conduct claims ([Sprintlaw](https://sprintlaw.com.au/articles/victoria-statute-of-limitations-key-time-limits/)); 1 year added as a buffer | `wipe_records`; PDFs in a private Supabase Storage bucket `certificates` |
| Failure records | 7 years | Same; shows what happened to a unit that didn't pass | `wipe_records` |
| Apple evidence photos | 7 years | Same as certificates | Private Storage bucket `apple-evidence` |
| Intake records with personal details | The longest legal minimum, then delete or de-identify | Second-hand dealers must record the seller's name, address and ID details ([CAV](https://www.consumer.vic.gov.au/businesses/registered-businesses/second-hand-dealers-and-pawnbrokers/running-your-business/keeping-records)); CAV's page gives no retention period, so check the 2018 Regulations. Most ATO records: 5 years ([business.gov.au](https://business.gov.au/finance/payments-and-invoicing/record-keeping)). APP 11.2 then requires deletion or de-identification | `intakes` (admin only) |
| Public signing keys | Permanently | Needed to check old certificates | `signing_keys` |

A scheduled Supabase job deletes or de-identifies records past their date. Each deletion is logged (record ID and date only).

## Apple devices

The technician erases iPads and T2 or Apple Silicon Macs with Apple's own erase. The tool only records the erase and issues a certificate marked **technician-attested**. It never claims to have checked the result itself.

1. Remove Activation Lock and Find My, or confirm they are already off.
2. Erase with Apple's tool: Erase All Content and Settings (iPad), Erase Assistant (Mac), or a DFU restore from Apple Configurator.
3. Capture evidence: a photo of the setup or Hello screen showing the serial, and a photo of Activation Lock showing as off.
4. Enter the record: serial, model, method, technician, intake ID and the photos. It is entered on the store's admin page.
5. The certificate method reads "Manufacturer cryptographic erase, technician-attested", Purge category, and states that the tool did not verify it.

Intel Macs without a T2 chip can boot the USB with Secure Boot off and go through the normal process; test this before relying on it.

## Failures and exceptions

Any failure produces a signed failure record and no certificate. A unit with a failure record can't be listed for sale until it has a passing certificate.

| Situation | Tool action | Outcome |
| --- | --- | --- |
| Laptop won't boot from the USB | Not reached | Repair, then wipe through the USB |
| HDD with no Purge command | Fail; flag for drive replacement | Fit a new SSD; destroy the old HDD and record it as destroyed against the intake ID |
| SSD, NVMe or eMMC with no Purge command | Fail | Storage can't be certified; replace a removable drive, or reject the unit if the storage is soldered |
| SATA drive stays frozen after suspend and resume | Fail with instructions | Try a different SATA mode in the BIOS, then try again |
| ATA security password or Opal lock that isn't known | Fail; prompt PSID revert | Try again after the revert, or reject the unit |
| Read errors or mismatches during verification | Retry the Purge once | Second failure: failure record, reject or replace the storage |
| Power lost during sanitize | Detect sanitize status when it next boots | The drive finishes; the record notes the interruption |
| Capacity reported lower than native after removing HPA/DCO | Fail | Inspect; possible firmware tampering |
| Firmware on ARM, or a Chromebook | Can't boot the USB | Out of scope for v1 |

## Testing and validation

The tool isn't used on stock for sale until every check below passes on at least two drives of each media type.

- [ ] **Boot:** the Debian live USB boots with Secure Boot on across the laptop models AccessibleTech usually takes in
- [ ] **Lockdown:** with Secure Boot on, kernel lockdown still allows the drive commands (`hdparm`, `nvme-cli`, `mmc-utils`). Test this first: if lockdown blocks them, the design changes
- [ ] **Forensic seed test:** before wiping, fill the drive with known marker files and a repeating pattern. After wiping, scan the raw device for the markers and run `photorec`. Pass means nothing is recovered
- [ ] **Cross-check:** wipe drives of the same model with BitRaser (while the licence is still active) and compare which method each tool chose and what each reported
- [ ] **HPA/DCO:** hide a marked area with an HPA, wipe, and confirm the area was restored and wiped
- [ ] **Frozen and locked drives:** confirm the suspend-and-resume unfreeze works, and that a locked drive fails cleanly
- [ ] **Power cut:** pull power during a sanitize and confirm the tool picks it up correctly on the next boot
- [ ] **Records:** unit tests for canonical JSON and signatures, a test that a tampered record is rejected, and checks that the PDF renders correctly
- [ ] **Sync:** offline queue, uploading the same record twice, a revoked key, and every state the verification page can show

## Limitations and risks

The biggest risk is commercial, not technical: SME and corporate buyers often ask for erasure software certified by a third party, and this tool has none.

| Risk | Effect | Mitigation |
| --- | --- | --- |
| No third-party certification (ADISA, Common Criteria, and similar) | SME procurement may refuse self-attested certificates | Don't market it as equal to BitRaser or Blancco; keep BitRaser available for SME contracts until this is resolved |
| Drive firmware that claims to sanitize but doesn't do it fully | Data left behind with a passing result from the drive | Full read-back; forensic seed test on each new drive model |
| Crypto-erase verification is a heuristic | Proves the data can't be read, not that the key is gone | Prefer Block Erase where the drive supports it |
| Signing key stored on the USB | A copied USB can issue certificates that verify | Separate key per USB, revocation, technician PIN |
| Kernel lockdown under Secure Boot | Could block drive commands | Test first (Testing) |
| Sole trader with no limit on liability | One data breach claim falls on Omar personally | Insurance; certificate terms limiting liability; a written procedure |
| Marketing claims ahead of the tool | Breaches the project's rule that a claim isn't made until it is operational | No claims about wiping or certificates until testing passes |

## Decision notes

Nothing is open. The notes below record why decisions 1, 3 and 5 went the way they did; all three followed the recommendation on 19 Nov 2026.

### 1. HDDs with no Purge command

**Decided: replace the drive; don't issue Clear certificates.**

- This should be rare. Most SATA HDDs support ATA Security Erase, which counts as Purge. The case is mainly very old drives, or drives behind a bridge chip that blocks commands.
- Every certificate AccessibleTech issues then says Purge. You never have to explain to a buyer why some certificates say Clear, and you avoid the method Rev 2 warns about.
- Fit a new SATA SSD, which also sells better than an HDD. Destroy the old HDD (drill or shred it, which is Rev 2's Destroy method), record it as destroyed against the intake ID, and send it to an AS 5377 recycler.
- The alternative was to accept a full overwrite, certified as Clear, for consumer stock only. It's cheaper per unit, but it creates a second, weaker kind of certificate.

### 3. PDF signature

**Decided: option A for v1; consider C later, on the server, if SME clients ask for it.**

A PDF signature is a separate thing from the record signature. The record signature is what the QR page checks. A PDF signature is what Adobe Reader shows in its signature panel when someone opens the file.

| Option | What the buyer sees | Cost | Works with offline USB signing | Effort |
| --- | --- | --- | --- | --- |
| **A. Signed record embedded in the PDF, plus the QR check** | No signature panel in Adobe. Scanning the QR shows "Verified" and the real record, so any edit to the PDF shows up as a mismatch | $0 | Yes | Low |
| **B. PDF signature with a self-issued certificate** | Adobe shows a signature but says the signer's identity is unknown or not trusted. To a non-technical buyer this reads as a warning | $0 | Partly; each USB would need its own certificate | Medium; Rust support for PDF signatures (PAdES) is thin, so it would likely call an external tool |
| **C. PDF signature with a certificate from a CA on Adobe's trusted list (AATL)** | Adobe shows "signed and valid" with AccessibleTech's verified name, and flags any later edit | From about US$299/yr for an organisation certificate ([SSL.com](https://ssl.com/products/document-trust/document-signing/ov/)) | No; the key must sit in protected hardware or a cloud signing service, not on a USB, so signing happens on the server after sync | Medium-high |

A is enough for consumer buyers and costs nothing. C is what corporate procurement teams recognise. It fits naturally because certificates are issued after sync (decision 5): the server would add the Adobe signature when it generates the final PDF.

### 5. When certificates are issued

**Decided: issue certificates only after sync.**

The USB builds the PDF as soon as the wipe passes. The record only reaches Supabase when the USB is next online. Until then, scanning the QR shows "Not yet synced".

- **Issue before sync (rejected):** the buyer can have the PDF straight away. But if they scan it before the USB has synced, it looks like a certificate that can't be verified. And if the USB is lost or fails before syncing, that record is gone for good and the certificate will never verify.
- **Issue after sync (chosen):** the USB's PDF is kept as a draft. The certificate the customer gets is downloaded from the admin page once the server has accepted the record, and a copy goes into Supabase Storage. Every certificate you hand out verifies, and this is the step where option C's Adobe signature would be added. The cost is waiting until the USB next syncs. Wiping happens before a laptop is listed, so the wait shouldn't hold up sales.

## Build plan

Five phases, each ending in a gate. No dates are set yet.

| Phase | Builds | Gate to pass before the next phase |
| --- | --- | --- |
| 1. Spike | A Debian live image that boots with Secure Boot on; drive commands tested under kernel lockdown | Drive commands work under lockdown |
| 2. Wipe engine | detect, prepare, purge and verify for SATA, NVMe and eMMC; failure handling and power-cut recovery | The forensic seed test passes on all four media types |
| 3. Records and certificates | Canonical JSON, Ed25519 signing, PDF with QR code; USB provisioning and the offline queue | Tampered records are rejected; PDFs render correctly |
| 4. Sync and verification page | Edge Function, tables, RPC, the `/verify/[id]` page; key revocation and the admin page (keys, Apple records) | Every verification page state has been tested |
| 5. Pilot | Run alongside BitRaser on real stock; write the Media Sanitisation Procedure | **Go-live:** wiping and certificates may be advertised |

Phase 1 comes first because kernel lockdown is the one unknown that could change the design. The boot image alone is about 2–4 days of part-time work; the wipe engine is the largest phase.
