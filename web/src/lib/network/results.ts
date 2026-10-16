import type { ConnectivityReport } from './types';

export function resultSummary(report: ConnectivityReport): string {
  if (report.reshape_reachable) return 'Connected and ready';
  if (report.internet_verified) return 'Internet works; Reshape could not be reached';
  return 'Local network connected; internet could not be verified';
}
