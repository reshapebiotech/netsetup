<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Card from '#lib/components/ui/card.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import Icon from '#lib/components/ui/icon.svelte';
  import { networkApi } from '#lib/network/api.js';
  import type { NetworkMacAddresses } from '#lib/network/types.js';

  let apPersistent = $state(false);
  let apLoading = $state(true);
  let apSaving = $state(false);
  let apError = $state('');
  let apSavedMessage = $state('');
  let macsLoading = $state(true);
  let macsError = $state('');
  let macs = $state<NetworkMacAddresses>({ wifi: [], ethernet: [] });

  async function loadApPreferences() {
    apLoading = true;
    apError = '';
    apSavedMessage = '';
    try {
      const prefs = await networkApi.getApPreferences();
      apPersistent = prefs.persistent_enabled;
    } catch (e) {
      apError = e instanceof Error ? e.message : 'Failed to load setup Wi-Fi settings';
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
      apSavedMessage = apPersistent ? 'Setup Wi-Fi test mode is on.' : 'Setup Wi-Fi test mode is off.';
    } catch (e) {
      apError = e instanceof Error ? e.message : 'Failed to save setup Wi-Fi settings';
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
    <h1 class="flex items-center gap-2 text-3xl font-semibold">
      <Icon name="settings" className="h-6 w-6 text-primary" />
      Settings
    </h1>
    <p class="text-base text-muted-foreground">Device information and setup Wi-Fi preferences.</p>
  </header>

  <Card className="space-y-4">
    <h2 class="flex items-center gap-2 text-lg font-semibold">
      <Icon name="wifi" className="h-5 w-5 text-primary" />
      Setup Wi-Fi test mode
    </h2>
    <p class="text-sm text-muted-foreground">
      Keep setup Wi-Fi available until this mode is turned off. It can run alongside an Ethernet connection, so you can configure the device while it stays online.
    </p>

    {#if apLoading}
      <Loading label="Loading setup Wi-Fi settings…" />
    {/if}

    {#if apError}
      <Alert tone="error">{apError}</Alert>
    {/if}

    {#if apSavedMessage}
      <Alert tone="info">{apSavedMessage}</Alert>
    {/if}

    <p class="text-sm text-muted-foreground">Testing another Wi-Fi network can still temporarily interrupt the setup access point.</p>

    <div class="flex items-center justify-between gap-3 rounded-md border border-border bg-card p-3">
      <div class="space-y-1">
        <p class="text-sm font-medium">Keep setup access point available</p>
        <p class="text-xs text-muted-foreground">
          Status: {apPersistent ? 'On' : 'Off'}
        </p>
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={apPersistent}
        aria-label="Toggle setup Wi-Fi test mode"
        disabled={apLoading || apSaving}
        onclick={() => (apPersistent = !apPersistent)}
        class="inline-flex h-11 w-14 shrink-0 items-center justify-center rounded-lg disabled:cursor-not-allowed disabled:opacity-60"
      >
        <span class={`relative inline-flex h-7 w-12 items-center rounded-full transition-colors ${apPersistent ? 'bg-primary' : 'bg-muted'}`}>
          <span class={`inline-block h-5 w-5 rounded-full transition-transform ${apPersistent ? 'translate-x-6 bg-primary-foreground' : 'translate-x-1 bg-foreground'}`}></span>
        </span>
      </button>
    </div>

    <div class="flex items-center gap-3">
      <Button variant="secondary" onclick={saveApPreferences} loading={apSaving} disabled={apLoading}>
        {#if !apSaving}
          <span class="mr-2 inline-flex items-center"><Icon name="save" /></span>
        {/if}
        Save settings
      </Button>
      <Button variant="secondary" onclick={loadApPreferences} loading={apLoading} disabled={apSaving}>
        {#if !apLoading}
          <span class="mr-2 inline-flex items-center"><Icon name="refresh" /></span>
        {/if}
        Reload
      </Button>
    </div>
  </Card>

  <Card className="space-y-4">
    <h2 class="flex items-center gap-2 text-lg font-semibold">
      <Icon name="network" className="h-5 w-5 text-primary" />
      Network MAC addresses
    </h2>

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
          {#each macs.wifi as mac (mac)}
            <p class="font-mono text-sm text-muted-foreground">{mac}</p>
          {/each}
        {:else}
          <p class="text-sm text-muted-foreground">No Wi-Fi MAC found.</p>
        {/if}
      </div>
      <div>
        <p class="text-sm font-semibold">Ethernet</p>
        {#if macs.ethernet.length}
          {#each macs.ethernet as mac (mac)}
            <p class="font-mono text-sm text-muted-foreground">{mac}</p>
          {/each}
        {:else}
          <p class="text-sm text-muted-foreground">No Ethernet MAC found.</p>
        {/if}
      </div>
    </div>

    <div class="flex items-center gap-3">
      <Button variant="secondary" onclick={loadMacs} loading={macsLoading}>
        {#if !macsLoading}
          <span class="mr-2 inline-flex items-center"><Icon name="refresh" /></span>
        {/if}
        Refresh MAC addresses
      </Button>
    </div>
  </Card>

  <div class="flex items-center gap-3">
    <Button variant="secondary" onclick={() => goto('/')}>
      <span class="mr-2 inline-flex items-center"><Icon name="back" /></span>
      Back
    </Button>
  </div>
</div>
