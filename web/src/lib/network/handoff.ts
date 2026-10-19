import type { Transport } from './types';
import { isValidIpv4 } from './validators';

export function deviceHandoffUrl(address: string | null, transport: Transport, testId: string): string | null {
  if (!address || !isValidIpv4(address) || !testId) return null;
  return `http://${address}:3000/setup/new/${transport}/save-handoff/${encodeURIComponent(testId)}`;
}
