#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: package-release.sh VERSION [BINARY]}"
[[ "$VERSION" =~ ^[A-Za-z0-9][A-Za-z0-9.+_-]*$ ]] || { echo 'Invalid version' >&2; exit 1; }
REPO_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY="${2:-$REPO_DIR/target/aarch64-unknown-linux-gnu/release/reshape-netsetup}"
OUTPUT_DIR="$REPO_DIR/dist"
NAME="reshape-netsetup-$VERSION-linux-arm64"
WORK="$(mktemp -d)"
trap 'rm -rf -- "$WORK"' EXIT
mkdir -p "$WORK/$NAME" "$OUTPUT_DIR"
install -m 0755 "$BINARY" "$WORK/$NAME/reshape-netsetup"
install -m 0755 "$REPO_DIR/deploy/scripts/install-rpi.sh" "$WORK/$NAME/install.sh"
install -m 0755 "$REPO_DIR/deploy/scripts/captive-portal-rules.sh" "$WORK/$NAME/captive-portal-rules.sh"
install -m 0644 "$REPO_DIR/deploy/systemd/reshape-netsetup.service" "$WORK/$NAME/reshape-netsetup.service"
install -m 0644 "$REPO_DIR/deploy/config.example.toml" "$WORK/$NAME/config.example.toml"
install -m 0644 "$REPO_DIR/README.md" "$WORK/$NAME/README.md"
install -m 0644 "$REPO_DIR/INSTALL.md" "$WORK/$NAME/INSTALL.md"
printf '%s\n' "$VERSION" > "$WORK/$NAME/VERSION"
(cd "$WORK/$NAME" && shasum -a 256 reshape-netsetup install.sh captive-portal-rules.sh reshape-netsetup.service config.example.toml README.md INSTALL.md VERSION > SHA256SUMS)
tar -czf "$OUTPUT_DIR/$NAME.tar.gz" -C "$WORK" "$NAME"
(cd "$OUTPUT_DIR" && shasum -a 256 "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
printf '%s\n' "$OUTPUT_DIR/$NAME.tar.gz"
