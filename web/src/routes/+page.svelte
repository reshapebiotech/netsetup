<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Card from '#lib/components/ui/card.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import Icon from '#lib/components/ui/icon.svelte';
  import { networkApi } from '#lib/network/api.js';
  import type { SavedNetwork } from '#lib/network/types.js';

  let saved = $state<SavedNetwork[]>([]);
  let loading = $state(true);
  let error = $state('');
  let deletingId = $state('');
  let deleteError = $state('');
  let hasInternet = $state(false);
  let connectivityKnown = $state(false);
  let resetLoading = $state(false);
  let resetConfirming = $state(false);
  let resetMessage = $state('');
  let resetMessageTone = $state<'' | 'info' | 'error'>('');

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

  async function loadConnectivityHint() {
    try {
      const connectivity = await networkApi.getConnectivity();
      hasInternet = connectivity.is_connected && connectivity.can_reach_host;
      connectivityKnown = true;
    } catch {
      hasInternet = false;
      connectivityKnown = false;
    }
  }

  function refreshHome() {
    void loadSaved();
    void loadConnectivityHint();
  }

  async function deleteSaved(id: string) {
    deletingId = id;
    deleteError = '';
    try {
      await networkApi.forget(id);
      await loadSaved();
    } catch (e) {
      deleteError = e instanceof Error ? e.message : 'Failed to delete saved network';
    } finally {
      deletingId = '';
    }
  }

  function beginResetAll() {
    resetConfirming = true;
    resetMessage = '';
    resetMessageTone = '';
  }

  function cancelResetAll() {
    resetConfirming = false;
  }

  async function confirmResetAll() {
    resetLoading = true;
    resetMessage = '';
    resetMessageTone = '';
    deleteError = '';
    try {
      await networkApi.resetAllSaved();
      resetConfirming = false;
      resetMessage = 'All saved networks were reset.';
      resetMessageTone = 'info';
      await loadSaved();
      await loadConnectivityHint();
    } catch (e) {
      resetMessage = e instanceof Error ? e.message : 'Failed to reset all saved networks';
      resetMessageTone = 'error';
    } finally {
      resetLoading = false;
    }
  }

  onMount(() => {
    refreshHome();
  });
</script>

<main class="space-y-7">
  <header class="space-y-3">
    <h1 class="text-3xl font-semibold sm:text-4xl">Networks</h1>
    <p class="max-w-xl text-base leading-relaxed text-muted-foreground">Connect your device to Wi-Fi or Ethernet. Test new settings before saving them.</p>
  </header>

  <Card className="space-y-5">
    <div class="flex items-center justify-between gap-3">
      <h2 class="text-base font-semibold">Saved connections</h2>
      <Button variant="secondary" onclick={refreshHome} loading={loading} disabled={resetLoading}>
        {#if !loading}<Icon name="refresh" className="h-4 w-4" />{/if}
        Refresh
      </Button>
    </div>

    {#if loading}
      <Loading label="Loading configured network…" />
    {:else if error}
      <Alert tone="error">{error}</Alert>
    {:else if deleteError}
      <Alert tone="error">{deleteError}</Alert>
    {:else if resetMessageTone === 'error'}
      <Alert tone="error">{resetMessage}</Alert>
    {:else if resetMessageTone === 'info'}
      <Alert tone="info">{resetMessage}</Alert>
    {:else if saved.length === 0}
      {#if connectivityKnown && hasInternet}
        <Alert tone="info">
          Device is already online using automatic network settings (DHCP). You do not need to configure anything unless you want to override network behavior.
        </Alert>
      {:else}
        <Alert tone="info">No managed network is configured yet.</Alert>
      {/if}
    {:else}
      {#each saved as entry (entry.id)}
        <div class="flex items-center gap-3 rounded-lg border border-border bg-background/30 p-3 sm:gap-4 sm:p-4">
            <a href={`/networks/${encodeURIComponent(entry.id)}`} class="group flex min-w-0 flex-1 items-center gap-3 rounded-md py-2 sm:gap-4">
              <span class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-border bg-muted/40 text-muted-foreground transition-colors group-hover:text-primary">
                <Icon name={entry.transport === 'wifi' ? 'wifi' : 'ethernet'} className="h-5 w-5" />
              </span>
              <span class="min-w-0">
                <span class="block break-words text-sm font-medium transition-colors group-hover:text-primary">{entry.display_name}</span>
                <span class="mt-1 block text-xs text-muted-foreground">{entry.transport === 'wifi' ? 'Wi-Fi' : 'Ethernet'} · View settings →</span>
              </span>
            </a>
            <Button
              variant="destructive"
              loading={deletingId === entry.id}
              disabled={resetLoading || (!!deletingId && deletingId !== entry.id)}
              onclick={() => deleteSaved(entry.id)}
            >
              Delete
            </Button>
        </div>
      {/each}
    {/if}
  </Card>

  <div class="flex flex-wrap items-start justify-between gap-4">
    <div>
      {#if resetConfirming}
        <div class="max-w-sm space-y-3">
          <p class="text-sm text-muted-foreground">This removes saved networks and may disconnect this device.</p>
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" disabled={resetLoading} onclick={cancelResetAll}>Cancel</Button>
            <Button variant="destructive" loading={resetLoading} onclick={confirmResetAll}>Reset all networks</Button>
          </div>
        </div>
      {:else}
        <button class="min-h-11 rounded-md text-sm text-muted-foreground transition-colors hover:text-destructive disabled:opacity-50" disabled={resetLoading} onclick={beginResetAll}>Reset all networks</button>
      {/if}
    </div>
    <Button onclick={() => goto('/setup/new')} disabled={resetLoading}>
      <span class="mr-2 inline-flex items-center"><Icon name="plus" /></span>
      Add new network
    </Button>
  </div>
</main>
