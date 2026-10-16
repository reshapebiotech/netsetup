<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Card from '#lib/components/ui/card.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import Icon from '#lib/components/ui/icon.svelte';
  import { networkApi } from '#lib/network/api.js';
  import { branding } from '#lib/branding.js';
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

<div class="space-y-8">
  <header class="space-y-4">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">{$branding.product_name}</p>
    <h1 class="text-4xl font-bold leading-tight">{$branding.headline}</h1>
    <div class="max-w-2xl space-y-2">
      <p class="text-base text-muted-foreground">Use this tool to bring the device online and manage the network profile used by the device.</p>
      <p class="text-base text-muted-foreground">{$branding.support_text}</p>
    </div>
  </header>

  <Card className="space-y-5 p-6">
    <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <h2 class="flex items-center gap-2 text-lg font-semibold">
        <Icon name="network" className="h-5 w-5 text-primary" />
        Configured network
      </h2>

      <div class="space-y-2 sm:text-right">
        {#if resetConfirming}
          <p class="text-xs text-muted-foreground">This removes saved networks and may disconnect this device.</p>
          <div class="flex flex-wrap items-center gap-2 sm:justify-end">
            <Button variant="secondary" disabled={resetLoading} onclick={cancelResetAll}>Cancel</Button>
            <Button variant="destructive" loading={resetLoading} disabled={resetLoading} onclick={confirmResetAll}>Reset all networks</Button>
          </div>
        {:else}
          <div class="flex flex-wrap items-center gap-2">
            <Button variant="secondary" onclick={refreshHome} loading={loading} disabled={resetLoading}>
              {#if !loading}
                <span class="mr-2 inline-flex items-center"><Icon name="refresh" /></span>
              {/if}
              Refresh
            </Button>
            <Button variant="destructive" disabled={resetLoading} onclick={beginResetAll}>Reset all networks</Button>
          </div>
        {/if}
      </div>
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
        <div class="rounded-lg border border-border bg-white p-4">
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="flex items-center gap-2 text-sm font-semibold">
                <Icon name={entry.transport === 'wifi' ? 'wifi' : 'ethernet'} className="h-4 w-4 text-muted-foreground" />
                {entry.display_name}
              </p>
              <p class="text-xs uppercase tracking-wide text-muted-foreground">{entry.transport}</p>
            </div>
            <Button
              variant="destructive"
              loading={deletingId === entry.id}
              disabled={resetLoading || (!!deletingId && deletingId !== entry.id)}
              onclick={() => deleteSaved(entry.id)}
            >
              Delete
            </Button>
          </div>
        </div>
      {/each}
    {/if}
  </Card>

  {#if $branding.help_links.length > 0}
    <Card className="space-y-4 p-6">
      <h2 class="flex items-center gap-2 text-lg font-semibold">
        <Icon name="help" className="h-5 w-5 text-primary" />
        {$branding.help_title}
      </h2>
      {#if $branding.help_text}
        <p class="text-sm text-muted-foreground">{$branding.help_text}</p>
      {/if}
      <div class="flex flex-wrap items-center gap-2">
        {#each $branding.help_links as link (`${link.label}-${link.url}`)}
          <a class="rounded-md border border-border bg-white px-3 py-2 text-sm font-medium text-primary hover:bg-muted" href={link.url} target="_blank" rel="noreferrer">
            <span class="mr-1 inline-flex items-center"><Icon name="link" className="h-4 w-4" /></span>
            {link.label}
          </a>
        {/each}
      </div>
    </Card>
  {/if}

  <div class="flex items-center justify-between gap-3">
    <Button variant="secondary" onclick={() => goto('/settings')} disabled={resetLoading}>
      <span class="mr-2 inline-flex items-center"><Icon name="settings" /></span>
      Settings
    </Button>
    <Button onclick={() => goto('/setup/new')} disabled={resetLoading}>
      <span class="mr-2 inline-flex items-center"><Icon name="plus" /></span>
      Add new network
    </Button>
  </div>
</div>
