#!/usr/bin/env bash
set -euo pipefail

BIN_SRC="${1:-./target/aarch64-unknown-linux-gnu/release/reshape-netsetup}"
BIN_DEST="/opt/reshape-netsetup/reshape-netsetup"
SERVICE_DEST="/etc/systemd/system/reshape-netsetup.service"
RULES_DEST="/usr/local/bin/captive-portal-rules.sh"

if [[ ! -f "${BIN_SRC}" ]]; then
  echo "binary not found: ${BIN_SRC}" >&2
  exit 1
fi

sudo install -d /opt/reshape-netsetup
sudo install -m 0755 "${BIN_SRC}" "${BIN_DEST}"
sudo install -m 0755 deploy/scripts/captive-portal-rules.sh "${RULES_DEST}"
sudo install -m 0644 deploy/systemd/reshape-netsetup.service "${SERVICE_DEST}"

sudo systemctl daemon-reload
sudo systemctl enable --now reshape-netsetup.service
sudo systemctl status --no-pager reshape-netsetup.service
