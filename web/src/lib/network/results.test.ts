import { describe, expect, it } from 'vitest';
import { groupLevel, serviceLevel, servicesReady, resultSummary } from './results';
import type { ConnectivityReport, EndpointGroupResult } from './types';

function group(name: string, required: boolean, reachable: boolean[]): EndpointGroupResult {
  return { name, required, endpoints: reachable.map((value, index) => ({
    name: `Endpoint ${index}`, url: `https://${name.toLowerCase()}.example.com/${index}`,
    description: '', reachable: value
  })) };
}
const local: ConnectivityReport = {
  interface: 'wlan0', address: '192.168.1.42', gateway_reachable: true,
  internet_verified: false, endpoint_groups: []
};
describe('connection results', () => {
  it('does not claim internet access from a reachable gateway', () => {
    expect(resultSummary(local)).toBe('Local network connected; internet could not be verified');
  });
  it('works without product endpoints configured', () => {
    expect(resultSummary({ ...local, internet_verified: true })).toBe('Internet access verified');
    expect(servicesReady({ ...local, internet_verified: true })).toBe(true);
    expect(serviceLevel(local)).toBeNull();
  });
  it('classifies full, partial and no connectivity within each group', () => {
    expect(groupLevel(group('Core', true, [true, true]))).toBe('full');
    expect(groupLevel(group('Core', true, [true, false]))).toBe('partial');
    expect(groupLevel(group('Core', true, [false, false]))).toBe('none');
  });
  it('accepts all required endpoints even when public internet checks are blocked', () => {
    const report = { ...local, endpoint_groups: [group('Core', true, [true, true])] };
    expect(servicesReady(report)).toBe(true);
    expect(resultSummary(report)).toBe('Full service connectivity');
  });
  it('distinguishes optional outages from required outages', () => {
    const report = { ...local, endpoint_groups: [group('Core', true, [true, true]), group('Updates', false, [true, false])] };
    expect(serviceLevel(report)).toBe('partial');
    expect(servicesReady(report)).toBe(true);
    expect(resultSummary(report)).toBe('Required services available; other services limited');
    report.endpoint_groups[0].endpoints[0].reachable = false;
    expect(servicesReady(report)).toBe(false);
    expect(resultSummary(report)).toBe('Partial service connectivity');
  });
  it('does not confuse internet access with service access', () => {
    const report = { ...local, internet_verified: true, endpoint_groups: [group('Core', true, [false, false])] };
    expect(servicesReady(report)).toBe(false);
    expect(resultSummary(report)).toBe('No service connectivity');
  });
});
