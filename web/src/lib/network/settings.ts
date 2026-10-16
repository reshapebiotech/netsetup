import type { NetworkSettings, Transport } from '$lib/network/types';

export interface NetworkFormState {
  transport: Transport;
  ssid: string;
  password: string;
  enterprise: boolean;
  openNetwork?: boolean;
  eapLogin: string;
  eapPassword: string;
  ipMethod: 'auto' | 'manual';
  ipv4Address: string;
  ipv4Prefix: string;
  ipv4Gateway: string;
  dnsRaw: string;
}

export function parseDnsInput(dnsRaw: string): string[] {
  return dnsRaw
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean);
}

export function buildSettingsFromForm(state: NetworkFormState): NetworkSettings {
  const settings: NetworkSettings = {};
  const dns = parseDnsInput(state.dnsRaw);

  if (state.transport === 'wifi') {
    settings.ssid = state.ssid.trim();
    if (!state.enterprise && !state.openNetwork && state.password.length) {
      settings.password = state.password;
    }
  }

  if (state.enterprise) {
    settings.eap_login = state.eapLogin.trim();
    settings.eap_password = state.eapPassword;
  }

  if (state.ipMethod === 'manual') {
    settings.ipv4 = {
      method: 'manual',
      address_data: [{ address: state.ipv4Address.trim(), prefix: Number.parseInt(state.ipv4Prefix, 10) || 24 }],
      gateway: state.ipv4Gateway.trim(),
      dns
    };
  } else {
    settings.ipv4 = { method: 'auto', dns };
  }

  return settings;
}
