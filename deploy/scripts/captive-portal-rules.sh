#!/usr/bin/env bash
set -euo pipefail

ACTION="${1:-}"
AP_IFACE="${AP_IFACE:-wlan0}"
DNS_PORT="${DNS_PORT:-10053}"
CAPTIVE_IP="${CAPTIVE_IP:-10.42.0.1}"
HTTP_PORT="${HTTP_PORT:-80}"
APP_PORT="${APP_PORT:-3000}"

if [[ "${ACTION}" != "up" && "${ACTION}" != "down" ]]; then
  echo "usage: $0 up|down" >&2
  exit 2
fi

ipt() {
  iptables "$@"
}

add_rule() {
  local table="$1"
  shift
  if ! ipt -t "${table}" -C "$@" 2>/dev/null; then
    ipt -t "${table}" -A "$@"
  fi
}

del_rule() {
  local table="$1"
  shift
  if ipt -t "${table}" -C "$@" 2>/dev/null; then
    ipt -t "${table}" -D "$@"
  fi
}

if [[ "${ACTION}" == "up" ]]; then
  add_rule nat PREROUTING -i "${AP_IFACE}" -p udp --dport 53 -j REDIRECT --to-ports "${DNS_PORT}"
  add_rule nat PREROUTING -i "${AP_IFACE}" -p tcp --dport 53 -j REDIRECT --to-ports "${DNS_PORT}"
  add_rule nat PREROUTING -i "${AP_IFACE}" -p tcp --dport "${HTTP_PORT}" -j REDIRECT --to-ports "${APP_PORT}"

  # Allow forwarding/NAT from AP clients to upstream interfaces.
  add_rule filter FORWARD -i "${AP_IFACE}" -j ACCEPT
  add_rule filter FORWARD -o "${AP_IFACE}" -m state --state RELATED,ESTABLISHED -j ACCEPT
  add_rule nat POSTROUTING ! -o "${AP_IFACE}" -s "${CAPTIVE_IP}/24" -j MASQUERADE
else
  del_rule nat PREROUTING -i "${AP_IFACE}" -p udp --dport 53 -j REDIRECT --to-ports "${DNS_PORT}"
  del_rule nat PREROUTING -i "${AP_IFACE}" -p tcp --dport 53 -j REDIRECT --to-ports "${DNS_PORT}"
  del_rule nat PREROUTING -i "${AP_IFACE}" -p tcp --dport "${HTTP_PORT}" -j REDIRECT --to-ports "${APP_PORT}"

  del_rule filter FORWARD -i "${AP_IFACE}" -j ACCEPT
  del_rule filter FORWARD -o "${AP_IFACE}" -m state --state RELATED,ESTABLISHED -j ACCEPT
  del_rule nat POSTROUTING ! -o "${AP_IFACE}" -s "${CAPTIVE_IP}/24" -j MASQUERADE
fi
