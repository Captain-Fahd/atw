//! Drive access for atw.
//!
//! - [`runner`]: the [`runner::CommandRunner`] trait. All drive commands go
//!   through it; tests use [`runner::FakeRunner`].
//! - [`safety`]: the read-only allowlist and the `atw.live=1` check.
//! - [`lsblk`]: listing disks and telling internal disks from the boot USB.
//! - [`host`]: Secure Boot, lockdown and DMI facts.
//! - [`probe`]: the read-only `atw probe` survey.
//!
//! Nothing in this crate writes to a drive unless the `live` cargo feature is
//! on. In Phase 1 there is no destructive code at all.
#![warn(missing_docs)]

pub mod host;
pub mod lsblk;
pub mod probe;
pub mod runner;
pub mod safety;
