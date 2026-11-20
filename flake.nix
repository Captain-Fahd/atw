{
  description = "atw: AccessibleTech Wipe dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "clippy" "rustfmt" ];
        targets = [ "x86_64-unknown-linux-musl" ];
      };
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = [
          rust
          pkgs.podman
          pkgs.qemu
          pkgs.OVMFFull
          pkgs.swtpm
          pkgs.ripgrep
          pkgs.jq
          pkgs.e2fsprogs
          pkgs.util-linux
          pkgs.shellcheck
          pkgs.socat
        ];
        # OVMF firmware with Secure Boot and Microsoft keys enrolled.
        OVMF_CODE = "${pkgs.OVMFFull.fd}/FV/OVMF_CODE.ms.fd";
        OVMF_VARS = "${pkgs.OVMFFull.fd}/FV/OVMF_VARS.ms.fd";
      };
    };
}
