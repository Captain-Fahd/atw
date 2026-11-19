//! `atw`: the AccessibleTech Wipe binary that runs on the boot USB.
#![warn(missing_docs)]

fn main() {
    println!("atw {}", env!("CARGO_PKG_VERSION"));
}
