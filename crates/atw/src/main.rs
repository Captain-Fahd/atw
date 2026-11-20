//! `atw`: the AccessibleTech Wipe binary that runs on the boot USB.
//!
//! Phase 1 has one subcommand, `probe`, which is read-only. Running `atw`
//! with no arguments runs `probe`.
#![warn(missing_docs)]

mod util;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use atw_drive::host;
use atw_drive::probe::{self, ProbeReport};
use atw_drive::runner::SystemRunner;

/// Where the ATECH-DATA partition is mounted on the live image.
const DATA_MOUNT: &str = "/mnt/data";
/// Printed after a probe so test harnesses can tell it finished.
const DONE_MARKER: &str = "ATW-PROBE-DONE";

const USAGE: &str = "\
usage: atw [probe [--out-dir DIR]]
       atw --version

  probe    Read-only survey of Secure Boot, lockdown and internal drives.
           Writes JSON to /mnt/data/probe/ (the ATECH-DATA partition).";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    match argv.as_slice() {
        [] | ["probe"] => cmd_probe(None),
        ["probe", "--out-dir", dir] => cmd_probe(Some(PathBuf::from(dir))),
        ["--version" | "-V"] => {
            println!("atw {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["--help" | "-h" | "help"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn cmd_probe(out_dir: Option<PathBuf>) -> ExitCode {
    let now = util::now_secs();
    let host = host::read(Path::new("/"));
    let report = match probe::run(&SystemRunner::read_only(), host, util::rfc3339(now)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("probe failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    print_summary(&report);

    let dir = out_dir.unwrap_or_else(|| {
        if Path::new(DATA_MOUNT).is_dir() {
            Path::new(DATA_MOUNT).join("probe")
        } else {
            eprintln!("warning: {DATA_MOUNT} not found (ATECH-DATA not mounted); saving to /tmp");
            PathBuf::from("/tmp/atw-probe")
        }
    });
    let host_name = util::slug(&format!(
        "{} {}",
        report.host.vendor.as_deref().unwrap_or(""),
        report.host.product.as_deref().unwrap_or("")
    ));
    let host_name = if host_name.is_empty() {
        "unknown".into()
    } else {
        host_name
    };
    let path = dir.join(format!("{host_name}-{}.json", util::compact(now)));
    let json = serde_json::to_vec_pretty(&report).expect("report serialises");
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|()| util::write_atomic(&path, &json)) {
        eprintln!("could not save {}: {e}", path.display());
        return ExitCode::FAILURE;
    }
    println!("saved {}", path.display());
    println!("{DONE_MARKER} {}", path.display());
    ExitCode::SUCCESS
}

fn print_summary(r: &ProbeReport) {
    let yn = |b: Option<bool>| match b {
        Some(true) => "yes",
        Some(false) => "NO",
        None => "unknown",
    };
    println!("atw probe {} at {}", r.tool_version, r.generated_at);
    println!(
        "  Secure Boot: {}   lockdown: {}   atw.live=1: {}",
        yn(r.host.secure_boot),
        r.host.lockdown.as_deref().unwrap_or("unknown"),
        yn(Some(r.host.atw_live_flag))
    );
    for d in &r.disks {
        let role = if d.boot_device {
            "boot USB (skipped)"
        } else if d.internal {
            "internal"
        } else {
            "external (skipped)"
        };
        let gb = d.size_bytes.unwrap_or(0) as f64 / 1e9;
        println!(
            "  {:<14} {:?} {:>8.1} GB  {} / {}  {}  passthrough: {}",
            d.path,
            d.media,
            gb,
            d.model.as_deref().unwrap_or("?"),
            d.serial.as_deref().unwrap_or("?"),
            role,
            yn(d.passthrough_ok)
        );
    }
}
