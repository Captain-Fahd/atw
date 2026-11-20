#!/usr/bin/env bash
# Builds build/atw-live.iso and build/atw-usb.img. Run from the dev shell
# (nix develop). Uses rootless Podman; never needs sudo.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p build/logs

cargo build --release --target x86_64-unknown-linux-musl

podman build --signature-policy image/policy.json \
	-t localhost/atw-live-builder -f image/Containerfile image

# CAP_SYS_ADMIN (inside the rootless user namespace only) lets live-build
# mount /proc and /sys in its chroot. No host devices are passed in.
podman run --rm \
	--cap-add SYS_ADMIN --cap-add MKNOD \
	--security-opt label=disable \
	-v "$PWD:/work" \
	-v atw-lb-work:/build \
	localhost/atw-live-builder /work/image/lb-build.sh

ls -lh build/atw-live.iso build/atw-usb.img
