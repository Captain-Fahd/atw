//! Read-only facts about the running machine: Secure Boot, kernel lockdown,
//! kernel command line and DMI identity.
//!
//! All paths are read relative to a root so tests can use a temp directory.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::safety;

/// EFI variable holding the Secure Boot state (EFI global variable GUID).
const SECURE_BOOT_VAR: &str =
    "sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c";

/// Facts about the host, as recorded in a probe report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostInfo {
    /// Booted via UEFI (`/sys/firmware/efi` exists).
    pub uefi: bool,
    /// Secure Boot enabled, or `None` if the variable can't be read.
    pub secure_boot: Option<bool>,
    /// Active kernel lockdown mode (`none`, `integrity`, `confidentiality`),
    /// or `None` if securityfs has no lockdown file.
    pub lockdown: Option<String>,
    /// Full kernel command line.
    pub cmdline: String,
    /// Whether the command line carries `atw.live=1`.
    pub atw_live_flag: bool,
    /// Kernel release (`uname -r`).
    pub kernel: String,
    /// DMI system vendor.
    pub vendor: Option<String>,
    /// DMI product name.
    pub product: Option<String>,
    /// DMI product serial (root only).
    pub serial: Option<String>,
    /// DMI BIOS version.
    pub bios_version: Option<String>,
}

/// Reads [`HostInfo`] from the filesystem under `root` (normally `/`).
///
/// Missing files become `None` or empty strings; this never fails.
pub fn read(root: &Path) -> HostInfo {
    let at = |p: &str| -> PathBuf { root.join(p) };
    let text = |p: &str| read_trimmed(&at(p));
    let cmdline = text("proc/cmdline").unwrap_or_default();
    HostInfo {
        uefi: at("sys/firmware/efi").is_dir(),
        secure_boot: fs::read(at(SECURE_BOOT_VAR))
            .ok()
            .and_then(|b| parse_secure_boot(&b)),
        lockdown: text("sys/kernel/security/lockdown").and_then(|s| parse_lockdown(&s)),
        atw_live_flag: safety::cmdline_has_live_flag(&cmdline),
        cmdline,
        kernel: text("proc/sys/kernel/osrelease").unwrap_or_default(),
        vendor: text("sys/class/dmi/id/sys_vendor"),
        product: text("sys/class/dmi/id/product_name"),
        serial: text("sys/class/dmi/id/product_serial"),
        bios_version: text("sys/class/dmi/id/bios_version"),
    }
}

fn read_trimmed(p: &Path) -> Option<String> {
    let s = fs::read_to_string(p).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Parses the efivarfs `SecureBoot` file: 4 attribute bytes, then 1 data byte.
pub fn parse_secure_boot(bytes: &[u8]) -> Option<bool> {
    bytes.get(4).map(|&b| b == 1)
}

/// Parses `/sys/kernel/security/lockdown`, e.g. `none [integrity] confidentiality`,
/// returning the bracketed (active) mode.
pub fn parse_lockdown(s: &str) -> Option<String> {
    s.split_whitespace()
        .find_map(|w| w.strip_prefix('[')?.strip_suffix(']'))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(root: &Path, rel: &str, data: &[u8]) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }

    #[test]
    fn parses_lockdown_and_secure_boot() {
        assert_eq!(
            parse_lockdown("none [integrity] confidentiality\n").as_deref(),
            Some("integrity")
        );
        assert_eq!(parse_lockdown("none integrity"), None);
        assert_eq!(parse_secure_boot(&[6, 0, 0, 0, 1]), Some(true));
        assert_eq!(parse_secure_boot(&[6, 0, 0, 0, 0]), Some(false));
        assert_eq!(parse_secure_boot(&[6, 0]), None);
    }

    #[test]
    fn reads_fake_root() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        put(r, SECURE_BOOT_VAR, &[6, 0, 0, 0, 1]);
        put(
            r,
            "sys/kernel/security/lockdown",
            b"none [integrity] confidentiality\n",
        );
        put(r, "proc/cmdline", b"boot=live atw.live=1 quiet\n");
        put(r, "proc/sys/kernel/osrelease", b"6.12.0-1-amd64\n");
        put(r, "sys/class/dmi/id/sys_vendor", b"LENOVO\n");
        put(r, "sys/class/dmi/id/product_name", b"  \n");
        let h = read(r);
        assert!(h.uefi && h.atw_live_flag);
        assert_eq!(h.secure_boot, Some(true));
        assert_eq!(h.lockdown.as_deref(), Some("integrity"));
        assert_eq!(h.vendor.as_deref(), Some("LENOVO"));
        assert_eq!(h.product, None);
        assert_eq!(h.kernel, "6.12.0-1-amd64");
    }

    #[test]
    fn empty_root_is_all_unknown() {
        let t = tempfile::tempdir().unwrap();
        let h = read(t.path());
        assert!(!h.uefi && !h.atw_live_flag);
        assert_eq!((h.secure_boot, h.lockdown), (None, None));
    }
}
