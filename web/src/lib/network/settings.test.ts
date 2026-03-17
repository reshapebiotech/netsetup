import { describe, expect, it } from 'vitest';
import { buildSettingsFromForm, parseDnsInput } from '$lib/network/settings';

describe('network settings payload', () => {
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

  it('normalizes DNS input tokens', () => {
    expect(parseDnsInput('1.1.1.1, , 8.8.8.8,')).toEqual(['1.1.1.1', '8.8.8.8']);
  });
});
