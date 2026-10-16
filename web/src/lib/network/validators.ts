import type { NetworkSettings, Transport } from '#lib/network/types.js';

function isValidIpv4(value: string): boolean {
  const parts = value.split('.');
  if (parts.length !== 4) return false;
  return parts.every((part) => {
    if (!/^\d+$/.test(part)) return false;
    if (part.length > 1 && part.startsWith('0')) return false;
    const num = Number.parseInt(part, 10);
    return num >= 0 && num <= 255;
  });
}

export function validateSettings(transport: Transport, input: NetworkSettings, enterprise: boolean, requiresPassword = true, retainingPassword = false): string[] {
  const errors: string[] = [];

  if (transport === 'wifi' && !input.ssid?.trim()) {
    errors.push('SSID is required for Wi-Fi');
  }

  if (enterprise) {
    if (!input.eap_login?.trim()) errors.push('802.1X identity is required');
    if (!retainingPassword && !input.eap_password?.trim()) errors.push('802.1X password is required');
  }

  if (transport === 'wifi' && !enterprise && requiresPassword && !retainingPassword && !input.password?.trim()) {
    errors.push('Password is required for secured Wi-Fi');
  }

  if (input.ipv4?.method === 'manual') {
    if (!input.ipv4.address_data?.length) errors.push('Manual IPv4 address is required');
    if (!input.ipv4.gateway?.trim()) errors.push('Gateway is required for manual IPv4');
    const manual = input.ipv4.address_data?.[0];
    if (manual) {
      if (!isValidIpv4(manual.address)) errors.push('Manual IPv4 address must be a valid IPv4 value');
      if (!Number.isInteger(manual.prefix) || manual.prefix < 0 || manual.prefix > 32) {
        errors.push('IPv4 prefix must be an integer between 0 and 32');
      }
    }
    if (input.ipv4.gateway?.trim() && !isValidIpv4(input.ipv4.gateway.trim())) {
      errors.push('Gateway must be a valid IPv4 value');
    }
  }

  const dns = input.ipv4?.dns ?? [];
  for (const server of dns) {
    if (!isValidIpv4(server.trim())) {
      errors.push(`DNS server '${server}' must be a valid IPv4 value`);
    }
  }

  return errors;
}
