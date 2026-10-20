import type {
  NetworkDraft,
  CurrentConnectionCheck,
  EndpointGroupConfig,
  AccessPointControllerStatus,
  AccessPointPreferences,
  NetworkMacAddresses,
  ConnectionState,
  Connectivity,
  EthernetNetwork,
  NetworkSettings,
  SavedNetwork,
  SavedNetworkDetails,
  Transport,
  WifiNetwork
} from '#lib/network/types.js';

interface ConnectResponse {
  test_id: string;
}

const TIMEOUT_FAST_MS = 4_000;
const TIMEOUT_DEFAULT_MS = 12_000;
const TIMEOUT_SLOW_MS = 20_000;

export class NetworkApiError extends Error {
  constructor(message: string, readonly status: number) { super(message); }
}

async function parseOrThrow(res: Response) {
  if (!res.ok) {
    let message = `Request failed (${res.status})`;
    try {
      const payload = await res.json();
      if (payload?.error) message = payload.error;
    } catch {
      // noop
    }
    throw new NetworkApiError(message, res.status);
  }
  return res;
}

async function withTimeout(
  input: RequestInfo | URL,
  init?: RequestInit,
  timeoutMs = TIMEOUT_DEFAULT_MS
) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    return await fetch(input, { ...init, signal: ctrl.signal });
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error('Request timed out while waiting for the device');
    }
    throw e;
  } finally {
    clearTimeout(timer);
  }
}

function api(path: string) {
  return `/api/network${path}`;
}

export const networkApi = {
  async getDeviceInfo(): Promise<{ hostname: string }> {
    const res = await withTimeout('/api/device');
    await parseOrThrow(res);
    return res.json();
  },
  async listSaved(): Promise<SavedNetwork[]> {
    const res = await withTimeout(api('/saved'));
    await parseOrThrow(res);
    return res.json();
  },

  async listDrafts(): Promise<NetworkDraft[]> {
    const res = await withTimeout(api('/tests'), { cache: 'no-store' }, TIMEOUT_FAST_MS);
    await parseOrThrow(res);
    const drafts = await res.json();
    if (!Array.isArray(drafts)) throw new Error('Could not load unfinished setup');
    return drafts;
  },

  async getDraft(id: string): Promise<SavedNetworkDetails> {
    const res = await withTimeout(api(`/tests/${encodeURIComponent(id)}`), { cache: 'no-store' });
    await parseOrThrow(res);
    return res.json();
  },

  async getSaved(id: string): Promise<SavedNetworkDetails> {
    const res = await withTimeout(api(`/saved/${encodeURIComponent(id)}`));
    await parseOrThrow(res);
    return res.json();
  },

  async scanWifi(): Promise<WifiNetwork[]> {
    const res = await withTimeout(api('/wifi/scan'));
    await parseOrThrow(res);
    return res.json();
  },

  async scanEthernet(): Promise<EthernetNetwork[]> {
    const res = await withTimeout(api('/ethernet/scan'));
    await parseOrThrow(res);
    return res.json();
  },

  async getConnectivityGroups(): Promise<EndpointGroupConfig[]> {
    const res = await withTimeout(api('/connectivity/groups'));
    await parseOrThrow(res);
    return res.json();
  },

  async checkCurrentConnections(): Promise<CurrentConnectionCheck[]> {
    const res = await withTimeout(api('/connectivity/check'), { cache: 'no-store' }, 90_000);
    await parseOrThrow(res);
    return res.json();
  },

  async getConnectivity(): Promise<Connectivity> {
    const res = await withTimeout(api('/connectivity'));
    await parseOrThrow(res);
    return res.json();
  },

  async getMacAddresses(): Promise<NetworkMacAddresses> {
    const res = await withTimeout(api('/mac_addresses'));
    await parseOrThrow(res);
    return res.json();
  },

  async getApControllerStatus(): Promise<AccessPointControllerStatus> {
    const res = await withTimeout(api('/access_point/controller'));
    await parseOrThrow(res);
    return res.json();
  },

  async getApPreferences(): Promise<AccessPointPreferences> {
    const res = await withTimeout(api('/access_point/preferences'), undefined, TIMEOUT_SLOW_MS);
    await parseOrThrow(res);
    return res.json();
  },

  async setApPreferences(prefs: AccessPointPreferences): Promise<void> {
    const res = await withTimeout(
      api('/access_point/preferences'),
      {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(prefs)
      },
      TIMEOUT_SLOW_MS
    );
    await parseOrThrow(res);
  },

  async startTest(transport: Transport, settings: NetworkSettings): Promise<string> {
    const endpoint = transport === 'wifi' ? '/wifi/test' : '/ethernet/test';
    const res = await withTimeout(
      api(endpoint),
      {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(settings)
      },
      TIMEOUT_SLOW_MS
    );
    await parseOrThrow(res);
    const payload = (await res.json()) as ConnectResponse;
    return payload.test_id;
  },

  async getStatus(testId: string): Promise<ConnectionState> {
    const res = await withTimeout(api(`/status/${testId}`), { cache: 'no-store' }, TIMEOUT_FAST_MS);
    await parseOrThrow(res);
    return res.json();
  },

  async saveTest(testId: string): Promise<void> {
    const res = await withTimeout(api(`/tests/${testId}/save`), { method: 'POST' }, TIMEOUT_SLOW_MS);
    await parseOrThrow(res);
  },

  async discardTest(testId: string): Promise<void> {
    const res = await withTimeout(api(`/tests/${testId}/discard`), { method: 'POST' });
    await parseOrThrow(res);
  },

  async forget(id: string): Promise<void> {
    const res = await withTimeout(api(`/saved/${id}`), { method: 'DELETE' });
    await parseOrThrow(res);
  },

  async resetAllSaved(): Promise<void> {
    const res = await withTimeout(api('/saved/reset'), { method: 'POST' }, TIMEOUT_SLOW_MS);
    await parseOrThrow(res);
  }
};
