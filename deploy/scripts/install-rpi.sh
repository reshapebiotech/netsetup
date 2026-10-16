#!/usr/bin/env bash
set -euo pipefail

fail() { echo "$*" >&2; exit 1; }
usage() {
  echo 'Usage: install.sh [BINARY] [--config CONFIG.toml] [--legacy-dir DIRECTORY]'
  echo 'Existing configuration is preserved unless --config is supplied.'
}

if [[ "${1:-}" == --help || "${1:-}" == -h ]]; then usage; exit 0; fi
if [[ "$EUID" != 0 ]]; then exec sudo -- "$0" "$@"; fi

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_SOURCE=''
LEGACY_DIR=''
BIN_SOURCE=''
while (( $# )); do
  case "$1" in
    --config) (( $# >= 2 )) || fail 'Missing --config path'; CONFIG_SOURCE="$2"; shift 2 ;;
    --legacy-dir) (( $# >= 2 )) || fail 'Missing --legacy-dir path'; LEGACY_DIR="$2"; shift 2 ;;
    --*) fail "Unknown option: $1" ;;
    *) [[ -z "$BIN_SOURCE" ]] || fail 'Only one binary may be supplied'; BIN_SOURCE="$1"; shift ;;
  esac
done

[[ -f /etc/debian_version ]] || fail 'This installer supports Debian/Raspberry Pi OS 12 or newer.'
# shellcheck source=/dev/null
source /etc/os-release
if [[ ! "${VERSION_ID:-}" =~ ^[0-9]+$ ]] || (( VERSION_ID < 12 )); then
  fail 'Debian/Raspberry Pi OS 12 or newer is required.'
fi
[[ "$(uname -m)" == aarch64 ]] || fail 'A 64-bit ARM operating system is required.'
command -v systemctl >/dev/null || fail 'systemd is required.'

if [[ -f "$SCRIPT_DIR/reshape-netsetup.service" ]]; then
  UNIT_SOURCE="$SCRIPT_DIR/reshape-netsetup.service"
  RULES_SOURCE="$SCRIPT_DIR/captive-portal-rules.sh"
  BIN_SOURCE="${BIN_SOURCE:-$SCRIPT_DIR/reshape-netsetup}"
  (cd "$SCRIPT_DIR" && sha256sum --check --quiet SHA256SUMS)
else
  UNIT_SOURCE="$SCRIPT_DIR/../systemd/reshape-netsetup.service"
  RULES_SOURCE="$SCRIPT_DIR/captive-portal-rules.sh"
  BIN_SOURCE="${BIN_SOURCE:-$SCRIPT_DIR/../../target/aarch64-unknown-linux-gnu/release/reshape-netsetup}"
fi
for source in "$BIN_SOURCE" "$UNIT_SOURCE" "$RULES_SOURCE"; do
  [[ -f "$source" ]] || fail "Required file not found: $source"
done

umask 077
WORK="$(mktemp -d)"
trap 'rm -rf -- "$WORK"' EXIT
CONFIG_DEST=/etc/reshape-netsetup/config.toml
if [[ -n "$CONFIG_SOURCE" ]]; then
  cp -- "$CONFIG_SOURCE" "$WORK/config.toml"
elif [[ -f "$CONFIG_DEST" ]]; then
  cp -- "$CONFIG_DEST" "$WORK/config.toml"
elif [[ -n "$LEGACY_DIR" && -f "$LEGACY_DIR/.env" ]]; then
  "$BIN_SOURCE" --import-legacy-config "$LEGACY_DIR/.env" > "$WORK/config.toml"
else
  fail 'First installation requires --config CONFIG.toml with an AP password.'
fi
"$BIN_SOURCE" --check-config "$WORK/config.toml"
if [[ -f "$SCRIPT_DIR/VERSION" ]]; then
  cp "$SCRIPT_DIR/VERSION" "$WORK/VERSION"
else
  "$BIN_SOURCE" --version > "$WORK/VERSION"
fi

missing=0
for command in nmcli busctl curl ping iptables dnsmasq; do
  if ! command -v "$command" >/dev/null && [[ ! -x "/usr/sbin/$command" ]]; then missing=1; fi
done
if [[ "$missing" == 1 || ! -s /etc/ssl/certs/ca-certificates.crt ]]; then
  apt-get update
  env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    network-manager systemd curl iputils-ping iptables dnsmasq-base ca-certificates
fi

changed=0
if [[ -n "$LEGACY_DIR" && -d "$LEGACY_DIR" ]]; then
  if command -v docker >/dev/null; then
    docker info >/dev/null 2>&1 || fail 'Start Docker so the legacy containers can be retired.'
    for container in setup-api setup-ui; do
      if metadata="$(docker inspect --format '{{.Config.Image}}|{{.HostConfig.RestartPolicy.Name}}|{{.State.Running}}' "$container" 2>/dev/null)"; then
        case "$metadata" in
          ghcr.io/reshapebiotech/machine-setup-api:*\|*|ghcr.io/reshapebiotech/machine-setup-ui:*\|*) ;;
          *) fail "Refusing to stop an unrelated container named $container" ;;
        esac
        if [[ "$metadata" != *'|no|false' ]]; then
          docker update --restart=no "$container" >/dev/null
          docker stop "$container" >/dev/null
          changed=1
        fi
      fi
    done
  fi
  if command -v crontab >/dev/null; then
    for owner in root "$(stat -c '%U' "$LEGACY_DIR")"; do
      if crontab -u "$owner" -l > "$WORK/cron" 2>/dev/null; then
        awk '/^#Ansible: machine-setup-self-update$/ { getline; next } { print }' "$WORK/cron" > "$WORK/cron.cleaned"
        if ! cmp -s "$WORK/cron" "$WORK/cron.cleaned"; then
          crontab -u "$owner" "$WORK/cron.cleaned"
          changed=1
        fi
      fi
    done
  fi
fi

install_atomic() {
  local source="$1" destination="$2" mode="$3" staged
  if [[ -f "$destination" ]] && cmp -s "$source" "$destination"; then
    chmod "$mode" "$destination"
    return
  fi
  staged="$(mktemp "$destination.XXXXXX")"
  if ! install -m "$mode" "$source" "$staged" || ! mv -f -- "$staged" "$destination"; then
    rm -f -- "$staged"
    return 1
  fi
  changed=1
}

install -d -m 0755 /opt/reshape-netsetup
install -d -m 0700 /etc/reshape-netsetup
install_atomic "$BIN_SOURCE" /opt/reshape-netsetup/reshape-netsetup 0755
install_atomic "$RULES_SOURCE" /usr/local/bin/captive-portal-rules.sh 0755
install_atomic "$UNIT_SOURCE" /etc/systemd/system/reshape-netsetup.service 0644
install_atomic "$WORK/config.toml" "$CONFIG_DEST" 0600
install_atomic "$WORK/VERSION" /opt/reshape-netsetup/VERSION 0644

if [[ "$changed" == 0 ]] && systemctl is-active --quiet reshape-netsetup.service && systemctl is-enabled --quiet reshape-netsetup.service; then
  echo 'Installation unchanged'
  exit 0
fi
systemctl daemon-reload
systemctl enable reshape-netsetup.service
systemctl restart reshape-netsetup.service
systemctl is-active --quiet reshape-netsetup.service
printf 'Installed reshape-netsetup %s\n' "$(cat /opt/reshape-netsetup/VERSION)"
