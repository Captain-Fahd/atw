//! `atw probe`: a read-only survey of the host and its drives.
//!
//! The report keeps every tool's raw output so it can be saved as a parser
//! fixture (see `docs/DEVELOPMENT.md`).

use serde::{Deserialize, Serialize};

use crate::host::HostInfo;
use crate::lsblk::{self, BlockDevice, LsblkError, Media};
use crate::runner::{CommandRunner, command_line};

/// Schema tag written into every probe report.
pub const SCHEMA: &str = "atw-probe/1";

/// One identify command and what it returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifyRun {
    /// Command line that was run.
    pub command: String,
    /// Exit code, or `None` if it couldn't start or was killed.
    pub code: Option<i32>,
    /// Raw stdout.
    pub stdout: String,
    /// Raw stderr, or the start error.
    pub stderr: String,
    /// Whether the output looks like a real identify response.
    pub ok: bool,
}

/// Probe result for one disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskProbe {
    /// Device path, e.g. `/dev/nvme0n1`.
    pub path: String,
    /// Media kind.
    pub media: Media,
    /// lsblk transport.
    pub tran: Option<String>,
    /// Model string.
    pub model: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
    /// Size in bytes as reported by the kernel.
    pub size_bytes: Option<u64>,
    /// Rotational (HDD).
    pub rotational: bool,
    /// Treated as internal (a wipe candidate in later phases).
    pub internal: bool,
    /// Identified as the atw boot USB.
    pub boot_device: bool,
    /// Identify commands run against the disk (internal disks only).
    pub identify: Vec<IdentifyRun>,
    /// All identify commands succeeded, so passthrough works under the
    /// current lockdown mode. `None` if nothing was attempted.
    pub passthrough_ok: Option<bool>,
}

/// A full probe report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeReport {
    /// Always [`SCHEMA`].
    pub schema: String,
    /// atw version that wrote the report.
    pub tool_version: String,
    /// RFC 3339 UTC time the probe ran.
    pub generated_at: String,
    /// Host facts.
    pub host: HostInfo,
    /// Every whole disk lsblk reported.
    pub disks: Vec<DiskProbe>,
    /// Raw lsblk JSON, kept as a fixture.
    pub lsblk_raw: String,
}

/// The identify commands for a disk: `(program, args)`.
pub fn identify_commands(dev: &BlockDevice) -> Vec<(&'static str, Vec<String>)> {
    match dev.media() {
        Media::Ata => vec![("hdparm", vec!["-I".into(), dev.path.clone()])],
        Media::Nvme => {
            let ctrl = nvme_controller(&dev.path);
            vec![
                (
                    "nvme",
                    vec!["id-ctrl".into(), ctrl, "-o".into(), "json".into()],
                ),
                (
                    "nvme",
                    vec!["id-ns".into(), dev.path.clone(), "-o".into(), "json".into()],
                ),
            ]
        }
        Media::Mmc => vec![(
            "mmc",
            vec!["extcsd".into(), "read".into(), dev.path.clone()],
        )],
        Media::Usb | Media::Other => vec![],
    }
}

/// `/dev/nvme0n1` → `/dev/nvme0`. Returns the input if it has no namespace suffix.
pub fn nvme_controller(ns_path: &str) -> String {
    match ns_path.rfind('n') {
        Some(i)
            if i > "/dev/nvme".len()
                && ns_path[i + 1..].chars().all(|c| c.is_ascii_digit())
                && i + 1 < ns_path.len() =>
        {
            ns_path[..i].to_string()
        }
        _ => ns_path.to_string(),
    }
}

/// Whether identify output looks genuine, not just an exit code of 0.
fn looks_ok(program: &str, stdout: &str) -> bool {
    match program {
        "hdparm" => stdout.contains("Model Number:"),
        "nvme" => serde_json::from_str::<serde_json::Value>(stdout)
            .map(|v| v.is_object())
            .unwrap_or(false),
        "mmc" => stdout.contains("EXT_CSD") || stdout.contains("Extended CSD"),
        _ => false,
    }
}

fn run_identify(runner: &dyn CommandRunner, program: &str, args: &[String]) -> IdentifyRun {
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    let command = command_line(program, &argv);
    match runner.run(program, &argv) {
        Ok(out) => IdentifyRun {
            ok: out.success() && looks_ok(program, &out.stdout),
            command,
            code: out.code,
            stdout: out.stdout,
            stderr: out.stderr,
        },
        Err(e) => IdentifyRun {
            command,
            code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            ok: false,
        },
    }
}

/// Runs the probe: lists disks, then runs identify on each internal disk.
///
/// # Errors
/// [`LsblkError`] if the disks can't be listed. Identify failures are
/// recorded in the report, not returned.
pub fn run(
    runner: &dyn CommandRunner,
    host: HostInfo,
    generated_at: String,
) -> Result<ProbeReport, LsblkError> {
    let (lsblk_raw, devs) = lsblk::list(runner)?;
    let disks = devs
        .iter()
        .filter(|d| d.is_real_disk())
        .map(|d| {
            let internal = d.is_internal();
            let identify: Vec<IdentifyRun> = if internal {
                identify_commands(d)
                    .iter()
                    .map(|(p, a)| run_identify(runner, p, a))
                    .collect()
            } else {
                vec![]
            };
            let passthrough_ok = (!identify.is_empty()).then(|| identify.iter().all(|r| r.ok));
            DiskProbe {
                path: d.path.clone(),
                media: d.media(),
                tran: d.tran.clone(),
                model: d.model.as_ref().map(|s| s.trim().to_string()),
                serial: d.serial.as_ref().map(|s| s.trim().to_string()),
                size_bytes: d.size,
                rotational: d.rota,
                internal,
                boot_device: d.is_boot_device(),
                identify,
                passthrough_ok,
            }
        })
        .collect();
    Ok(ProbeReport {
        schema: SCHEMA.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at,
        host,
        disks,
        lsblk_raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{CommandOutput, FakeRunner};

    const LSBLK: &str = r#"{"blockdevices":[
      {"name":"sda","path":"/dev/sda","type":"disk","tran":"sata","model":"QEMU HARDDISK  ","serial":"ATWSATA0001","size":8589934592,"rm":false,"hotplug":false,"rota":true,"mountpoints":[null]},
      {"name":"nvme0n1","path":"/dev/nvme0n1","type":"disk","tran":"nvme","model":"QEMU NVMe Ctrl","serial":"ATWNVME0001","size":8589934592,"rm":false,"hotplug":false,"rota":false,"mountpoints":[null]},
      {"name":"sdb","path":"/dev/sdb","type":"disk","tran":"usb","size":2000000000,"rm":true,"hotplug":true,"rota":false,"mountpoints":[null],
       "children":[{"name":"sdb2","path":"/dev/sdb2","type":"part","label":"ATECH-DATA","mountpoints":["/mnt/data"]}]}
    ]}"#;

    #[test]
    fn nvme_controller_paths() {
        assert_eq!(nvme_controller("/dev/nvme0n1"), "/dev/nvme0");
        assert_eq!(nvme_controller("/dev/nvme12n3"), "/dev/nvme12");
        assert_eq!(nvme_controller("/dev/nvme0"), "/dev/nvme0");
        assert_eq!(nvme_controller("/dev/nvme0n"), "/dev/nvme0n");
    }

    #[test]
    fn probe_runs_identify_on_internal_disks_only() {
        let fake = FakeRunner::new()
            .with(
                &command_line("lsblk", lsblk::LSBLK_ARGS),
                CommandOutput::ok(LSBLK),
            )
            .with(
                "hdparm -I /dev/sda",
                CommandOutput::ok("\n/dev/sda:\n\nATA device, with non-removable media\n\tModel Number:       QEMU HARDDISK\n"),
            )
            .with(
                "nvme id-ctrl /dev/nvme0 -o json",
                CommandOutput::ok(r#"{"vid":6966,"sn":"ATWNVME0001"}"#),
            )
            .with(
                "nvme id-ns /dev/nvme0n1 -o json",
                CommandOutput::failed(1, "Operation not permitted"),
            );
        let host = crate::host::read(std::path::Path::new("/nonexistent"));
        let r = run(&fake, host, "2026-11-20T00:00:00Z".into()).unwrap();
        assert_eq!(r.schema, SCHEMA);
        assert_eq!(r.disks.len(), 3);
        let (sda, nvme, usb) = (&r.disks[0], &r.disks[1], &r.disks[2]);
        assert_eq!(sda.passthrough_ok, Some(true));
        assert_eq!(sda.model.as_deref(), Some("QEMU HARDDISK"));
        assert_eq!(nvme.passthrough_ok, Some(false));
        assert!(!nvme.identify[1].ok && nvme.identify[0].ok);
        assert!(usb.boot_device && !usb.internal && usb.passthrough_ok.is_none());
        assert!(fake.calls().iter().all(|c| !c.contains("sdb")));
    }

    #[test]
    fn exit_zero_without_identify_data_is_not_ok() {
        assert!(!looks_ok("hdparm", "\n/dev/sda:\n"));
        assert!(!looks_ok("nvme", "not json"));
        assert!(looks_ok(
            "mmc",
            "=============================================\n  Extended CSD rev 1.8 (MMC 5.1)\n"
        ));
    }
}
