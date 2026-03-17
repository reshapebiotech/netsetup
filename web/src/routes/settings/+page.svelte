<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Alert from '$lib/components/ui/alert.svelte';
  import Loading from '$lib/components/ui/loading.svelte';
  import { networkApi } from '$lib/network/api';
  import type { NetworkMacAddresses } from '$lib/network/types';

  let apPersistent = false;
  let apLoading = true;
  let apSaving = false;
  let apError = '';
  let apSavedMessage = '';
  let macsLoading = true;
  let macsError = '';
  let macs: NetworkMacAddresses = { wifi: [], ethernet: [] };

  async function loadApPreferences() {
    apLoading = true;
    apError = '';
    apSavedMessage = '';
    try {
      const prefs = await networkApi.getApPreferences();
      apPersistent = prefs.persistent_enabled;
    } catch (e) {
      apError = e instanceof Error ? e.message : 'Failed to load settings';
    } finally {
      apLoading = false;
    }
  }

  async function saveApPreferences() {
    apSaving = true;
    apError = '';
    apSavedMessage = '';
    try {
      await networkApi.setApPreferences({ persistent_enabled: apPersistent });
      apSavedMessage = apPersistent ? 'Persistent setup Wi-Fi is now on.' : 'Persistent setup Wi-Fi is now off.';
    } catch (e) {
      apError = e instanceof Error ? e.message : 'Failed to save settings';
    } finally {
      apSaving = false;
    }
  }

  async function loadMacs() {
    macsLoading = true;
    macsError = '';
    try {
      macs = await networkApi.getMacAddresses();
    } catch (e) {
      macsError = e instanceof Error ? e.message : 'Failed to load MAC addresses';
    } finally {
      macsLoading = false;
    }
  }

  onMount(() => {
    void loadApPreferences();
    void loadMacs();
  });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Machine Setup</p>
    <h1 class="text-3xl font-bold">Settings</h1>
  </header>

  <Card className="space-y-4">
    <h2 class="text-lg font-semibold">Device behavior</h2>
    <p class="text-sm text-muted-foreground">
      Keep setup Wi-Fi available while Ethernet is connected. This setting is temporary and resets after restart.
    </p>

    {#if apLoading}
      <Loading label="Loading settings…" />
    {/if}

    {#if apError}
      <Alert tone="error">{apError}</Alert>
    {/if}

    {#if apSavedMessage}
      <Alert tone="info">{apSavedMessage}</Alert>
    {/if}

    <label class="flex items-center gap-2 text-sm">
      <input type="checkbox" bind:checked={apPersistent} class="rounded border-input" disabled={apLoading || apSaving} />
      Keep setup Wi-Fi available in Ethernet mode
    </label>

    <div class="flex items-center gap-3">
      <Button variant="secondary" onclick={saveApPreferences} loading={apSaving} disabled={apLoading}>
        Save settings
      </Button>
      <Button variant="secondary" onclick={loadApPreferences} loading={apLoading} disabled={apSaving}>
        Reload
      </Button>
    </div>
  </Card>

  <Card className="space-y-4">
    <h2 class="text-lg font-semibold">Network MAC addresses</h2>

    {#if macsLoading}
      <Loading label="Loading MAC addresses…" />
    {/if}

    {#if macsError}
      <Alert tone="error">{macsError}</Alert>
    {/if}

    <div class="space-y-3">
      <div>
        <p class="text-sm font-semibold">Wi-Fi</p>
        {#if macs.wifi.length}
          {#each macs.wifi as mac}
            <p class="font-mono text-sm text-muted-foreground">{mac}</p>
          {/each}
        {:else}
          <p class="text-sm text-muted-foreground">No Wi-Fi MAC found.</p>
        {/if}
      </div>
      <div>
        <p class="text-sm font-semibold">Ethernet</p>
        {#if macs.ethernet.length}
          {#each macs.ethernet as mac}
            <p class="font-mono text-sm text-muted-foreground">{mac}</p>
          {/each}
        {:else}
          <p class="text-sm text-muted-foreground">No Ethernet MAC found.</p>
        {/if}
      </div>
    </div>

    <div class="flex items-center gap-3">
      <Button variant="secondary" onclick={loadMacs} loading={macsLoading}>Refresh MAC addresses</Button>
    </div>
  </Card>

  <div class="flex items-center gap-3">
    <Button variant="secondary" onclick={() => goto('/')}>Back</Button>
  </div>
</div>
