import type { ConnectivityReport, EndpointGroupResult } from './types';

export type ConnectivityLevel = 'full' | 'partial' | 'none';
export function groupLevel(group: EndpointGroupResult): ConnectivityLevel {
  const reachable = group.endpoints.filter(endpoint => endpoint.reachable).length;
  if (reachable === 0) return 'none';
  return reachable === group.endpoints.length ? 'full' : 'partial';
}
export function serviceLevel(report: ConnectivityReport): ConnectivityLevel | null {
  const endpoints = report.endpoint_groups.flatMap(group => group.endpoints);
  if (!endpoints.length) return null;
  const reachable = endpoints.filter(endpoint => endpoint.reachable).length;
  return reachable === 0 ? 'none' : reachable === endpoints.length ? 'full' : 'partial';
}
export function servicesReady(report: ConnectivityReport): boolean {
  const required = report.endpoint_groups.filter(group => group.required);
  if (required.length) return required.every(group => groupLevel(group) === 'full');
  return serviceLevel(report) === 'full' || report.internet_verified;
}
export function resultSummary(report: ConnectivityReport): string {
  const level = serviceLevel(report);
  if (level === 'full') return 'Full service connectivity';
  if (level === 'none') return 'No service connectivity';
  if (level === 'partial') return report.endpoint_groups.some(group => group.required) && servicesReady(report)
    ? 'Required services available; other services limited' : 'Partial service connectivity';
  return report.internet_verified ? 'Internet access verified'
    : 'Local network connected; internet could not be verified';
}
