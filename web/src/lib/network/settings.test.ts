import { describe, expect, it } from 'vitest';
import { buildSettingsFromForm, parseDnsInput } from '#lib/network/settings.js';
import { validateSettings } from '#lib/network/validators.js';

describe('network settings payload', () => {
  it.each(['0', '24', '32', 0, 24, 32])('preserves a valid /%s prefix', (prefix) => {
    const settings = manualSettings(prefix);
    expect(settings.ipv4?.address_data?.[0].prefix).toBe(Number(prefix));
    expect(validateSettings('ethernet', settings, false))
      .not.toContain('IPv4 prefix must be an integer between 0 and 32');
  });

  it.each(['', ' ', 'abc', '24junk', '24.5', '-1', '33', 'Infinity'])('rejects invalid prefix %j instead of substituting /24', (prefix) => {
    expect(validateSettings('ethernet', manualSettings(prefix), false))
      .toContain('IPv4 prefix must be an integer between 0 and 32');
  });

  it('submits DNS in auto mode', () => {
    const settings = buildSettingsFromForm({
      transport: 'ethernet',
      ssid: '',
      password: '',
      enterprise: false,
      eapLogin: '',
      eapPassword: '',
      ipMethod: 'auto',
      ipv4Address: '',
      ipv4Prefix: '24',
      ipv4Gateway: '',
      dnsRaw: 'aaaa, 1.1.1.1'
    });

    expect(settings).toEqual({
      ipv4: {
        method: 'auto',
        dns: ['aaaa', '1.1.1.1']
      }
    });
  });

  it('submits DNS in manual mode', () => {
    const settings = buildSettingsFromForm({
      transport: 'ethernet',
      ssid: '',
      password: '',
      enterprise: false,
      eapLogin: '',
      eapPassword: '',
      ipMethod: 'manual',
      ipv4Address: '192.168.1.20',
      ipv4Prefix: '24',
      ipv4Gateway: '192.168.1.1',
      dnsRaw: '8.8.8.8, 1.1.1.1'
    });

    expect(settings.ipv4?.dns).toEqual(['8.8.8.8', '1.1.1.1']);
  });

  it('builds enterprise wifi payload', () => {
    const settings = buildSettingsFromForm({
      transport: 'wifi',
      ssid: 'CorpWiFi',
      password: 'ignored',
      enterprise: true,
      eapLogin: ' user@example.com ',
      eapPassword: 'secret',
      ipMethod: 'auto',
      ipv4Address: '',
      ipv4Prefix: '24',
      ipv4Gateway: '',
      dnsRaw: ''
    });

    expect(settings.ssid).toBe('CorpWiFi');
    expect(settings.eap_login).toBe('user@example.com');
    expect(settings.eap_password).toBe('secret');
    expect(settings.password).toBeUndefined();
  });

  it('builds enterprise ethernet payload', () => {
    const settings = buildSettingsFromForm({
      transport: 'ethernet',
      ssid: '',
      password: 'ignored',
      enterprise: true,
      eapLogin: ' user@example.com ',
      eapPassword: 'secret',
      ipMethod: 'auto',
      ipv4Address: '',
      ipv4Prefix: '24',
      ipv4Gateway: '',
      dnsRaw: ''
    });

    expect(settings.ssid).toBeUndefined();
    expect(settings.eap_login).toBe('user@example.com');
    expect(settings.eap_password).toBe('secret');
    expect(settings.password).toBeUndefined();
  });

  it('normalizes DNS input tokens', () => {
    expect(parseDnsInput('1.1.1.1, , 8.8.8.8,')).toEqual(['1.1.1.1', '8.8.8.8']);
  });
});

function manualSettings(ipv4Prefix: string | number) {
  return buildSettingsFromForm({
    transport: 'ethernet', ssid: '', password: '', enterprise: false,
    eapLogin: '', eapPassword: '', ipMethod: 'manual',
    ipv4Address: '192.168.1.20', ipv4Gateway: '192.168.1.1', ipv4Prefix, dnsRaw: ''
  });
}
