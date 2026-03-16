import { scanWifi, getSavedNetworks, connectWifi, checkConnectionStatus, forgetNetwork, type WifiNetwork, type SavedNetwork, type ConnectionState } from './api';

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
    loading = $state(false);
    error = $state<string | null>(null);
    machine = $state<MachineState>({ type: 'idle' });

    async loadNetworks() {
        this.loading = true;
        this.error = null;
        try {
            const [scanned, saved] = await Promise.all([
                scanWifi(),
                getSavedNetworks()
            ]);
            this.available = scanned;
            this.saved = saved;
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
                
                // If it's still 'Testing', we just wait and poll again.
                // But if we previously thought it was disconnected (AP dropped), 
                // and now we got a response, we should transition back to testing
                if (this.machine.type === 'disconnected') {
                    this.machine = { type: 'testing', ssid, testId };
                }

            } catch (e: any) {
                // Network error - this is EXPECTED! The AP just dropped.
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
