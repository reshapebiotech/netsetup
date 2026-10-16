import { describe, expect, it } from 'vitest';
import { resultSummary } from './results';
import type { ConnectivityReport } from './types';

const local: ConnectivityReport = {
  interface: 'wlan0', address: '192.168.1.42', gateway_reachable: true,
  internet_verified: false, reshape_reachable: false
};

describe('connection results', () => {
  it('does not claim internet access from a reachable gateway', () => {
    expect(resultSummary(local)).toBe('Local network connected; internet could not be verified');
  });
  it('distinguishes internet access from Reshape access', () => {
    expect(resultSummary({ ...local, internet_verified: true })).toBe('Internet works; Reshape could not be reached');
  });
  it('accepts Reshape access even if a public probe is blocked', () => {
    expect(resultSummary({ ...local, gateway_reachable: false, reshape_reachable: true })).toBe('Connected and ready');
  });
});
