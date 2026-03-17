import type { NetworkSettings, Transport } from '$lib/network/types';

export function validateSettings(transport: Transport, input: NetworkSettings, enterprise: boolean): string[] {
  const errors: string[] = [];

  if (transport === 'wifi' && !input.ssid?.trim()) {
    errors.push('SSID is required for Wi-Fi');
  }

  if (transport === 'wifi' && enterprise) {
    if (!input.eap_login?.trim()) errors.push('Enterprise identity is required');
    if (!input.eap_password?.trim()) errors.push('Enterprise password is required');
  }

  if (transport === 'wifi' && !enterprise && !input.password?.trim()) {
    errors.push('Password is required for secured Wi-Fi');
  }

  if (input.ipv4?.method === 'manual') {
    if (!input.ipv4.address_data?.length) errors.push('Manual IPv4 address is required');
    if (!input.ipv4.gateway?.trim()) errors.push('Gateway is required for manual IPv4');
  }

  return errors;
}
