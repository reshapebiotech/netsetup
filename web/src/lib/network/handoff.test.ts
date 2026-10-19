import { describe, expect, it } from 'vitest';
import { deviceHandoffUrl } from './handoff';

describe('device handoff address', () => {
  it('retains the operation ID and uses the LAN web port', () => {
    expect(deviceHandoffUrl('192.168.11.0', 'wifi', 'test/id'))
      .toBe('http://192.168.11.0:3000/setup/new/wifi/save-handoff/test%2Fid');
  });
  it.each([null, '', 'example.com', 'javascript:alert(1)', '999.1.1.1', '1.2.3.4:80', '01.2.3.4'])('rejects invalid address %s', address => {
    expect(deviceHandoffUrl(address, 'ethernet', 'test')).toBeNull();
  });
});
