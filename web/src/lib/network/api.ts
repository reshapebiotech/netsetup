import type {
  AccessPointControllerStatus,
  BrandingConfig,
  NetworkMacAddresses,
  AccessPointPreferences,
  ConnectionState,
  Connectivity,
  EthernetNetwork,
  NetworkSettings,
  SavedNetwork,
  Transport,
  WifiNetwork
} from '$lib/network/types';

interface ConnectResponse {
  test_id: string;
}

const TIMEOUT_FAST_MS = 4_000;
const TIMEOUT_DEFAULT_MS = 12_000;
const TIMEOUT_SLOW_MS = 20_000;

async function parseOrThrow(res: Response) {
  if (!res.ok) {
    let message = `Request failed (${res.status})`;
    try {
      const payload = await res.json();
      if (payload?.error) message = payload.error;
    } catch {
      // noop
    }
    throw new Error(message);
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
  async listSaved(): Promise<SavedNetwork[]> {
    const res = await withTimeout(api('/saved'));
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

  async getBranding(): Promise<BrandingConfig> {
    const res = await withTimeout('/api/branding');
    await parseOrThrow(res);
    return res.json();
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
    const res = await withTimeout(api(`/status/${testId}`), undefined, TIMEOUT_FAST_MS);
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
  }
};
