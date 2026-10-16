import { describe, expect, it } from 'vitest';
import { validateSettings } from '#lib/network/validators.js';

describe('network validators', () => {
  it('requires wifi ssid', () => {
    const errors = validateSettings('wifi', {}, false);
    expect(errors).toContain('SSID is required for Wi-Fi');
  });

  it('requires password for non-enterprise wifi', () => {
    const errors = validateSettings('wifi', { ssid: 'LabWiFi' }, false);
    expect(errors).toContain('Password is required for secured Wi-Fi');
  });

  it('requires enterprise credentials in enterprise mode', () => {
    const errors = validateSettings('wifi', { ssid: 'Corp' }, true);
    expect(errors).toContain('802.1X identity is required');
    expect(errors).toContain('802.1X password is required');
  });

  it('accepts valid enterprise wifi settings', () => {
    const errors = validateSettings(
      'wifi',
      { ssid: 'Corp', eap_login: 'user@example.com', eap_password: 'secret' },
      true
    );
    expect(errors).toEqual([]);
  });

  it('accepts valid enterprise ethernet settings', () => {
    const errors = validateSettings(
      'ethernet',
      { eap_login: 'user@example.com', eap_password: 'secret' },
      true
    );
    expect(errors).toEqual([]);
  });

  it('requires manual ipv4 fields', () => {
    const errors = validateSettings(
      'ethernet',
      { ipv4: { method: 'manual', address_data: [] } },
      false
    );
    expect(errors).toContain('Manual IPv4 address is required');
    expect(errors).toContain('Gateway is required for manual IPv4');
  });

  it('rejects invalid ipv4 and dns values', () => {
    const errors = validateSettings(
      'ethernet',
      {
        ipv4: {
          method: 'manual',
          address_data: [{ address: '999.1.1.1', prefix: 64 }],
          gateway: 'abcd',
          dns: ['1.1.1.1', 'aaaa']
        }
      },
      false
    );
    expect(errors).toContain('Manual IPv4 address must be a valid IPv4 value');
    expect(errors).toContain('IPv4 prefix must be an integer between 0 and 32');
    expect(errors).toContain('Gateway must be a valid IPv4 value');
    expect(errors).toContain("DNS server 'aaaa' must be a valid IPv4 value");
  });

  it('rejects invalid DNS in auto mode', () => {
    const errors = validateSettings(
      'ethernet',
      {
        ipv4: {
          method: 'auto',
          dns: ['8.8.8.8', '999.1.1.1']
        }
      },
      false
    );
    expect(errors).toContain("DNS server '999.1.1.1' must be a valid IPv4 value");
  });

  it('accepts valid manual ipv4 + dns values', () => {
    const errors = validateSettings(
      'ethernet',
      {
        ipv4: {
          method: 'manual',
          address_data: [{ address: '192.168.1.20', prefix: 24 }],
          gateway: '192.168.1.1',
          dns: ['1.1.1.1', '8.8.8.8']
        }
      },
      false
    );
    expect(errors).toEqual([]);
  });

  it('accepts empty dns', () => {
    const errors = validateSettings(
      'ethernet',
      {
        ipv4: {
          method: 'auto',
          dns: []
        }
      },
      false
    );
    expect(errors).toEqual([]);
  });
});
