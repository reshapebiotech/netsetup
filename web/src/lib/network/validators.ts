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

function ipv4Number(value: string): number {
  return value.split('.').reduce((address, octet) => address * 256 + Number(octet), 0);
}

function ipv4Mask(prefix: number): number {
  return 2 ** 32 - 2 ** (32 - prefix);
}

function isSubnetEndpoint(address: number, prefix: number): boolean {
  if (prefix >= 31) return false;
  const host = (address & ~ipv4Mask(prefix)) >>> 0;
  return host === 0 || host === 2 ** (32 - prefix) - 1;
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
    const parsedAddresses: { address: number; prefix: number }[] = [];
    for (const manual of input.ipv4.address_data ?? []) {
      const validAddress = isValidIpv4(manual.address);
      const validPrefix = Number.isInteger(manual.prefix) && manual.prefix >= 0 && manual.prefix <= 32;
      if (!validAddress) errors.push('Manual IPv4 address must be a valid IPv4 value');
      if (!validPrefix) errors.push('IPv4 prefix must be an integer between 0 and 32');
      if (validAddress && validPrefix) {
        const address = ipv4Number(manual.address);
        if (isSubnetEndpoint(address, manual.prefix)) {
          errors.push("Manual IPv4 address must not be the subnet's network or broadcast address");
        }
        parsedAddresses.push({ address, prefix: manual.prefix });
      }
    }
    const gateway = input.ipv4.gateway ?? '';
    if (gateway.trim() && !isValidIpv4(gateway)) {
      errors.push('Gateway must be a valid IPv4 value');
    } else if (isValidIpv4(gateway) && parsedAddresses.length > 0) {
      const address = ipv4Number(gateway);
      if (parsedAddresses.some((manual) => manual.address === address)) {
        errors.push('Gateway must differ from the device IP address');
      } else {
        const subnets = parsedAddresses.filter((manual) =>
          (manual.address & ipv4Mask(manual.prefix)) === (address & ipv4Mask(manual.prefix))
        );
        if (subnets.length === 0) {
          errors.push('Gateway must be in the same subnet as a manual IPv4 address');
        } else if (subnets.every((manual) => isSubnetEndpoint(address, manual.prefix))) {
          errors.push("Gateway must not be the subnet's network or broadcast address");
        }
      }
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
