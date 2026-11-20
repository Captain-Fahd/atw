//! Block device listing via `lsblk`, and classification into internal disks.

use serde::{Deserialize, Deserializer, Serialize};

use crate::runner::{CommandRunner, RunError};

/// Label of the ext4 data partition on the atw boot USB.
pub const DATA_LABEL: &str = "ATECH-DATA";
/// Mount point of the live medium under live-boot.
pub const LIVE_MEDIUM: &str = "/run/live/medium";

/// Arguments passed to `lsblk`.
pub const LSBLK_ARGS: &[&str] = &[
    "-J",
    "-b",
    "-o",
    "NAME,PATH,TYPE,TRAN,MODEL,SERIAL,SIZE,RM,HOTPLUG,ROTA,LABEL,UUID,MOUNTPOINTS",
];

/// One node in the `lsblk -J` tree.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BlockDevice {
    /// Kernel name, e.g. `sda`, `nvme0n1`.
    pub name: String,
    /// Device path, e.g. `/dev/sda`.
    pub path: String,
    /// `disk`, `part`, `loop`, `rom`, …
    #[serde(rename = "type")]
    pub kind: String,
    /// Transport: `sata`, `nvme`, `usb`, `mmc`, …
    pub tran: Option<String>,
    /// Model string.
    pub model: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
    /// Size in bytes.
    pub size: Option<u64>,
    /// Removable flag.
    #[serde(deserialize_with = "flex_bool", default)]
    pub rm: bool,
    /// Hot-plug flag.
    #[serde(deserialize_with = "flex_bool", default)]
    pub hotplug: bool,
    /// Rotational (HDD) flag.
    #[serde(deserialize_with = "flex_bool", default)]
    pub rota: bool,
    /// Filesystem label.
    pub label: Option<String>,
    /// Filesystem UUID.
    pub uuid: Option<String>,
    /// Mount points (lsblk emits `[null]` when unmounted).
    #[serde(default)]
    pub mountpoints: Vec<Option<String>>,
    /// Partitions and other children.
    #[serde(default)]
    pub children: Vec<BlockDevice>,
}

#[derive(Deserialize)]
struct LsblkJson {
    blockdevices: Vec<BlockDevice>,
}

/// Older lsblk prints `"0"`/`"1"`, newer prints `false`/`true`.
fn flex_bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum B {
        Bool(bool),
        Str(String),
        Null(()),
    }
    Ok(match B::deserialize(d)? {
        B::Bool(b) => b,
        B::Str(s) => s == "1",
        B::Null(()) => false,
    })
}

/// Errors from listing block devices.
#[derive(Debug, thiserror::Error)]
pub enum LsblkError {
    /// lsblk could not be run.
    #[error(transparent)]
    Run(#[from] RunError),
    /// lsblk exited non-zero.
    #[error("lsblk failed: {0}")]
    Failed(String),
    /// lsblk output was not the expected JSON.
    #[error("could not parse lsblk output: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Parses `lsblk -J` output.
///
/// # Errors
/// [`LsblkError::Parse`] if the JSON doesn't match.
pub fn parse(json: &str) -> Result<Vec<BlockDevice>, LsblkError> {
    Ok(serde_json::from_str::<LsblkJson>(json)?.blockdevices)
}

/// Runs lsblk and returns the raw output and the parsed tree.
///
/// # Errors
/// [`LsblkError`] if lsblk can't run, fails, or prints unexpected output.
pub fn list(runner: &dyn CommandRunner) -> Result<(String, Vec<BlockDevice>), LsblkError> {
    let out = runner.run("lsblk", LSBLK_ARGS)?;
    if !out.success() {
        return Err(LsblkError::Failed(out.stderr.trim().to_string()));
    }
    let devs = parse(&out.stdout)?;
    Ok((out.stdout, devs))
}

/// Kind of storage, from the transport and device name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Media {
    /// SATA/ATA disk (HDD or SSD; see `rotational`).
    Ata,
    /// NVMe namespace.
    Nvme,
    /// eMMC user area.
    Mmc,
    /// USB-attached disk.
    Usb,
    /// Anything else (virtio, SCSI, unknown).
    Other,
}

impl BlockDevice {
    /// Media kind of this device.
    pub fn media(&self) -> Media {
        match self.tran.as_deref() {
            Some("sata" | "ata") => Media::Ata,
            Some("nvme") => Media::Nvme,
            Some("usb") => Media::Usb,
            Some("mmc") => Media::Mmc,
            _ if self.name.starts_with("mmcblk") => Media::Mmc,
            _ if self.name.starts_with("nvme") => Media::Nvme,
            _ => Media::Other,
        }
    }

    /// True if this device or any child carries the atw boot USB's markers:
    /// the `ATECH-DATA` label or the live medium mount point.
    pub fn is_boot_device(&self) -> bool {
        let here = self.label.as_deref() == Some(DATA_LABEL)
            || self.mountpoints.iter().flatten().any(|m| m == LIVE_MEDIUM);
        here || self.children.iter().any(BlockDevice::is_boot_device)
    }

    /// True for whole disks that are not pseudo-devices (zram, eMMC boot/RPMB
    /// areas).
    pub fn is_real_disk(&self) -> bool {
        self.kind == "disk"
            && !self.name.starts_with("zram")
            && !(self.name.starts_with("mmcblk")
                && (self.name.contains("boot") || self.name.contains("rpmb")))
    }

    /// True for disks atw treats as internal: real, not the boot USB, not USB
    /// or removable.
    pub fn is_internal(&self) -> bool {
        self.is_real_disk() && !self.is_boot_device() && self.media() != Media::Usb && !self.rm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"blockdevices":[
      {"name":"sda","path":"/dev/sda","type":"disk","tran":"sata","model":"ST500LM021","serial":"W1A2B3","size":500107862016,"rm":false,"hotplug":false,"rota":true,"label":null,"uuid":null,"mountpoints":[null]},
      {"name":"sdb","path":"/dev/sdb","type":"disk","tran":"usb","model":"Cruzer","serial":"U1","size":32000000000,"rm":"1","hotplug":"1","rota":"0","label":null,"uuid":null,"mountpoints":[null],
       "children":[
         {"name":"sdb1","path":"/dev/sdb1","type":"part","tran":null,"model":null,"serial":null,"size":2000000000,"rm":true,"hotplug":true,"rota":false,"label":"ATW-BOOT","uuid":"1234-ABCD","mountpoints":["/run/live/medium"]},
         {"name":"sdb2","path":"/dev/sdb2","type":"part","tran":null,"model":null,"serial":null,"size":30000000000,"rm":true,"hotplug":true,"rota":false,"label":"ATECH-DATA","uuid":"u-u-i-d","mountpoints":["/mnt/data"]}
       ]},
      {"name":"nvme0n1","path":"/dev/nvme0n1","type":"disk","tran":"nvme","model":"Samsung 980","serial":"S1","size":1000204886016,"rm":false,"hotplug":false,"rota":false,"label":null,"uuid":null,"mountpoints":[null]},
      {"name":"mmcblk0","path":"/dev/mmcblk0","type":"disk","tran":null,"model":null,"serial":"0x1234","size":62537072640,"rm":false,"hotplug":false,"rota":false,"label":null,"uuid":null,"mountpoints":[null]},
      {"name":"mmcblk0boot0","path":"/dev/mmcblk0boot0","type":"disk","tran":null,"model":null,"serial":null,"size":4194304,"rm":false,"hotplug":false,"rota":false,"label":null,"uuid":null,"mountpoints":[null]},
      {"name":"zram0","path":"/dev/zram0","type":"disk","tran":null,"model":null,"serial":null,"size":1000,"rm":false,"hotplug":false,"rota":false,"label":null,"uuid":null,"mountpoints":["[SWAP]"]},
      {"name":"loop0","path":"/dev/loop0","type":"loop","tran":null,"model":null,"serial":null,"size":1000,"rm":false,"hotplug":false,"rota":false,"label":null,"uuid":null,"mountpoints":["/run/live/rootfs/filesystem.squashfs"]}
    ]}"#;

    #[test]
    fn parses_and_classifies() {
        let devs = parse(SAMPLE).unwrap();
        let internal: Vec<_> = devs
            .iter()
            .filter(|d| d.is_internal())
            .map(|d| (d.name.as_str(), d.media()))
            .collect();
        assert_eq!(
            internal,
            vec![
                ("sda", Media::Ata),
                ("nvme0n1", Media::Nvme),
                ("mmcblk0", Media::Mmc)
            ]
        );
        let usb = &devs[1];
        assert!(usb.is_boot_device() && usb.rm && !usb.rota);
        assert!(devs[0].rota);
    }

    #[test]
    fn boot_marker_on_non_usb_disk_still_excludes_it() {
        let json = r#"{"blockdevices":[{"name":"sda","path":"/dev/sda","type":"disk","tran":"sata","size":1,"rm":false,
          "children":[{"name":"sda2","path":"/dev/sda2","type":"part","label":"ATECH-DATA","mountpoints":[null]}]}]}"#;
        let d = &parse(json).unwrap()[0];
        assert!(d.is_boot_device() && !d.is_internal());
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(parse("not json"), Err(LsblkError::Parse(_))));
    }
}
