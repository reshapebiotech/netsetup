import { describe, expect, it } from 'vitest';
import { validateSettings } from '$lib/network/validators';

describe('network validators', () => {
  it('requires wifi ssid', () => {
    const errors = validateSettings('wifi', {}, false);
    expect(errors).toContain('SSID is required for Wi-Fi');
  });

  it('requires enterprise credentials in enterprise mode', () => {
    const errors = validateSettings('wifi', { ssid: 'Corp' }, true);
    expect(errors).toContain('Enterprise identity is required');
    expect(errors).toContain('Enterprise password is required');
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
});
