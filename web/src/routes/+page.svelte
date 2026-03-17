<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Alert from '$lib/components/ui/alert.svelte';
  import Loading from '$lib/components/ui/loading.svelte';
  import { networkApi } from '$lib/network/api';
  import type { SavedNetwork } from '$lib/network/types';

  let saved: SavedNetwork[] = [];
  let loading = true;
  let error = '';
  let apPersistent = false;
  let apLoading = true;
  let apSaving = false;
  let apError = '';
  let apSavedMessage = '';

  async function loadSaved() {
    loading = true;
    error = '';
    try {
      saved = await networkApi.listSaved();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load saved network';
    } finally {
      loading = false;
    }
  }

  async function loadApPreferences() {
    apLoading = true;
    apError = '';
    apSavedMessage = '';
    try {
      const prefs = await networkApi.getApPreferences();
      apPersistent = prefs.persistent_enabled;
    } catch (e) {
      apError = e instanceof Error ? e.message : 'Failed to load device behavior';
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
      apError = e instanceof Error ? e.message : 'Failed to save device behavior';
    } finally {
      apSaving = false;
    }
  }

  onMount(() => {
    void loadSaved();
    void loadApPreferences();
  });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Machine Setup</p>
    <h1 class="text-3xl font-bold">Network configuration</h1>
    <p class="text-sm text-muted-foreground">Pick one managed network profile. Saving a new one replaces the previous profile.</p>
  </header>

  <Card className="space-y-4">
    <h2 class="text-lg font-semibold">Configured network</h2>

    {#if loading}
      <Loading label="Loading configured network…" />
    {:else if error}
      <Alert tone="error">{error}</Alert>
    {:else if saved.length === 0}
      <Alert tone="info">No managed network is configured yet.</Alert>
    {:else}
      {#each saved as entry}
        <div class="rounded-md border border-border bg-white p-3">
          <p class="text-sm font-semibold">{entry.display_name}</p>
          <p class="text-xs uppercase tracking-wide text-muted-foreground">{entry.transport}</p>
        </div>
      {/each}
    {/if}
  </Card>

  <Card className="space-y-4">
    <h2 class="text-lg font-semibold">Device behavior</h2>
    <p class="text-sm text-muted-foreground">
      Keep setup Wi-Fi available while Ethernet is connected. This setting is temporary and resets after restart.
    </p>

    {#if apLoading}
      <Loading label="Loading device behavior…" />
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
        Save behavior
      </Button>
      <Button variant="secondary" onclick={loadApPreferences} loading={apLoading} disabled={apSaving}>
        Reload
      </Button>
    </div>
  </Card>

  <div class="flex items-center gap-3">
    <Button onclick={() => goto('/setup/new')}>Add new network</Button>
    <Button variant="secondary" onclick={loadSaved} loading={loading}>Refresh</Button>
  </div>
</div>
