import { scanWifi, getSavedNetworks, connectWifi, checkConnectionStatus, forgetNetwork, checkConnectivity, type WifiNetwork, type SavedNetwork, type Connectivity } from './api';

export type MachineState = 
    | { type: 'idle' }
    | { type: 'connecting', ssid: string }
    | { type: 'testing', ssid: string, testId: string }
    | { type: 'disconnected', ssid: string, testId: string } // AP dropped, polling failed
    | { type: 'success', ssid: string }
    | { type: 'failed', ssid: string, error: string };

export class NetworkState {
    available = $state<WifiNetwork[]>([]);
    saved = $state<SavedNetwork[]>([]);
    connectivity = $state<Connectivity | null>(null);
    loading = $state(false);
    error = $state<string | null>(null);
    machine = $state<MachineState>({ type: 'idle' });

    private connectivityTimer: ReturnType<typeof setInterval> | null = null;

    startConnectivityPolling() {
        this.pollConnectivity();
        this.connectivityTimer = setInterval(() => this.pollConnectivity(), 5000);
    }

    stopConnectivityPolling() {
        if (this.connectivityTimer) clearInterval(this.connectivityTimer);
    }

    private async pollConnectivity() {
        try {
            this.connectivity = await checkConnectivity();
        } catch (e) {
            // Ignore temporary failures
        }
    }

    async loadNetworks() {
        this.loading = true;
        this.error = null;
        try {
            const [scanned, saved, conn] = await Promise.all([
                scanWifi(),
                getSavedNetworks(),
                checkConnectivity().catch(() => null)
            ]);
            this.available = scanned;
            this.saved = saved;
            if (conn) this.connectivity = conn;
        } catch (e: any) {
            this.error = e.message || 'Failed to load networks';
        } finally {
            this.loading = false;
        }
    }

    async forget(id: string) {
        try {
            await forgetNetwork(id);
            await this.loadNetworks();
        } catch (e: any) {
            this.error = e.message || 'Failed to forget network';
        }
    }

    async connect(ssid: string, password?: string) {
        this.machine = { type: 'connecting', ssid };
        this.error = null;

        try {
            const res = await connectWifi(ssid, password);
            this.machine = { type: 'testing', ssid, testId: res.test_id };
            this.pollStatus(res.test_id, ssid);
        } catch (e: any) {
            this.machine = { type: 'failed', ssid, error: e.message || 'Connection failed to start' };
        }
    }

    private async pollStatus(testId: string, ssid: string) {
        let attempts = 0;
        const maxAttempts = 60; // 60 seconds if polling every 1s

        while (attempts < maxAttempts) {
            try {
                const status = await checkConnectionStatus(testId);
                
                if (status.status === 'Connected') {
                    this.machine = { type: 'success', ssid };
                    await this.loadNetworks();
                    return;
                } else if (status.status === 'Failed') {
                    this.machine = { type: 'failed', ssid, error: status.detail };
                    return;
                }
                
                if (this.machine.type === 'disconnected') {
                    this.machine = { type: 'testing', ssid, testId };
                }

            } catch (e: any) {
                this.machine = { type: 'disconnected', ssid, testId };
            }

            await new Promise(r => setTimeout(r, 1000));
            attempts++;
        }

        this.machine = { type: 'failed', ssid, error: 'Connection timed out' };
    }

    reset() {
        this.machine = { type: 'idle' };
        this.error = null;
        this.loadNetworks();
    }
}
