//! Drive access for atw.
//!
//! - [`runner`]: the [`runner::CommandRunner`] trait. All drive commands go
//!   through it; tests use [`runner::FakeRunner`].
//! - [`safety`]: the read-only allowlist and the `atw.live=1` check.
//!
//! Nothing in this crate writes to a drive unless the `live` cargo feature is
//! on. In Phase 1 there is no destructive code at all.
#![warn(missing_docs)]

pub mod runner;
pub mod safety;
