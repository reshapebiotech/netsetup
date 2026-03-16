export interface WifiNetwork {
    ssid: string;
    signal_strength: number;
    security: string;
}

export interface SavedNetwork {
    id: string;
    ssid: string;
}

export type ConnectionState = 
    | { status: 'Testing' }
    | { status: 'Connected' }
    | { status: 'Failed'; detail: string };

export async function scanWifi(): Promise<WifiNetwork[]> {
    const res = await fetch('/api/network/wifi/scan');
    if (!res.ok) throw new Error('Failed to scan wifi');
    return res.json();
}

export async function getSavedNetworks(): Promise<SavedNetwork[]> {
    const res = await fetch('/api/network/saved');
    if (!res.ok) throw new Error('Failed to fetch saved networks');
    return res.json();
}

export async function connectWifi(ssid: string, password?: string): Promise<{ test_id: string }> {
    const res = await fetch('/api/network/wifi/connect', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ssid, password: password || undefined }),
    });
    if (!res.ok) throw new Error('Failed to connect to wifi');
    return res.json();
}

export async function checkConnectionStatus(testId: string): Promise<ConnectionState> {
    const res = await fetch(`/api/network/status/${testId}`);
    if (!res.ok) throw new Error('Failed to check status');
    return res.json();
}

export async function forgetNetwork(id: string): Promise<void> {
    const res = await fetch(`/api/network/saved/${id}`, { method: 'DELETE' });
    if (!res.ok) throw new Error('Failed to forget network');
}

export interface Connectivity {
    is_connected: boolean;
    can_reach_host: boolean;
}

export async function checkConnectivity(): Promise<Connectivity> {
    const res = await fetch('/api/network/wifi/connectivity');
    if (!res.ok) throw new Error('Failed to check connectivity');
    return res.json();
}
