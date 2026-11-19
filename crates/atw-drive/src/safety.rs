//! Safety checks shared by everything that touches drives.

/// Kernel command-line flag that must be present before atw will do anything
/// destructive.
pub const LIVE_FLAG: &str = "atw.live=1";

/// True if the kernel command line contains [`LIVE_FLAG`] as a whole word.
pub fn cmdline_has_live_flag(cmdline: &str) -> bool {
    cmdline.split_whitespace().any(|t| t == LIVE_FLAG)
}

/// True if `program args…` is a command atw knows to be read-only.
///
/// This is an allowlist: anything not listed is treated as destructive and
/// refused by [`crate::runner::SystemRunner`]. Keep each entry as narrow as
/// possible (exact argument shapes, not prefixes).
pub fn is_read_only(program: &str, args: &[&str]) -> bool {
    match (program, args) {
        // lsblk only lists block devices.
        ("lsblk", _) => true,
        // ATA IDENTIFY DEVICE.
        ("hdparm", ["-I", dev]) => is_dev_path(dev),
        // NVMe Identify Controller / Namespace, JSON output.
        ("nvme", ["id-ctrl", dev, "-o", "json"]) | ("nvme", ["id-ns", dev, "-o", "json"]) => {
            is_dev_path(dev)
        }
        // eMMC EXT_CSD register read.
        ("mmc", ["extcsd", "read", dev]) => is_dev_path(dev),
        _ => false,
    }
}

fn is_dev_path(p: &str) -> bool {
    p.starts_with("/dev/") && !p.contains("..") && !p.contains(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_flag_is_whole_word() {
        assert!(cmdline_has_live_flag("boot=live atw.live=1 quiet"));
        assert!(!cmdline_has_live_flag("boot=live atw.live=10"));
        assert!(!cmdline_has_live_flag("xatw.live=1"));
        assert!(!cmdline_has_live_flag(""));
    }

    #[test]
    fn allowlist_is_narrow() {
        assert!(is_read_only("hdparm", &["-I", "/dev/sda"]));
        assert!(is_read_only(
            "nvme",
            &["id-ctrl", "/dev/nvme0", "-o", "json"]
        ));
        assert!(is_read_only("mmc", &["extcsd", "read", "/dev/mmcblk0"]));
        assert!(!is_read_only("hdparm", &["-I", "/dev/sda", "--yes-i-know"]));
        assert!(!is_read_only("hdparm", &["-N", "p100", "/dev/sda"]));
        assert!(!is_read_only("nvme", &["sanitize", "/dev/nvme0"]));
        assert!(!is_read_only("mmc", &["erase", "secure-erase", "0", "1"]));
        assert!(!is_read_only("hdparm", &["-I", "../dev/sda"]));
        assert!(!is_read_only("sh", &["-c", "true"]));
    }
}
