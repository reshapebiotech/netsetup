#!/usr/bin/env bash
set -euo pipefail

BIN_SRC="${1:-./target/aarch64-unknown-linux-gnu/release/reshape-netsetup}"
BIN_DEST="/opt/reshape-netsetup/reshape-netsetup"
SERVICE_DEST="/etc/systemd/system/reshape-netsetup.service"
RULES_DEST="/usr/local/bin/captive-portal-rules.sh"

for cmd in nmcli busctl curl ping iptables; do
  if ! command -v "$cmd" >/dev/null 2>&1 && [[ ! -x "/usr/sbin/$cmd" ]]; then
    echo "required command not found: $cmd" >&2
    exit 1
  fi
done

for src in "${BIN_SRC}" deploy/scripts/captive-portal-rules.sh deploy/systemd/reshape-netsetup.service; do
  if [[ ! -f "${src}" ]]; then
    echo "required file not found: ${src}" >&2
    exit 1
  fi
done

install_atomic() {
  local src="$1" dest="$2" mode="$3" staged
  staged="$(sudo mktemp "${dest}.XXXXXX")"
  if ! sudo install -m "${mode}" "${src}" "${staged}" || ! sudo mv -f -- "${staged}" "${dest}"; then
    sudo rm -f -- "${staged}"
    return 1
  fi
}

sudo install -d /opt/reshape-netsetup
install_atomic "${BIN_SRC}" "${BIN_DEST}" 0755
install_atomic deploy/scripts/captive-portal-rules.sh "${RULES_DEST}" 0755
install_atomic deploy/systemd/reshape-netsetup.service "${SERVICE_DEST}" 0644

sudo systemctl daemon-reload
sudo systemctl enable reshape-netsetup.service
sudo systemctl restart reshape-netsetup.service
sudo systemctl status --no-pager reshape-netsetup.service
