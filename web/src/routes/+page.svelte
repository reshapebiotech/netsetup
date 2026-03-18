<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Alert from '$lib/components/ui/alert.svelte';
  import Loading from '$lib/components/ui/loading.svelte';
  import Icon from '$lib/components/ui/icon.svelte';
  import { networkApi } from '$lib/network/api';
  import { branding } from '$lib/branding';
  import type { SavedNetwork } from '$lib/network/types';

  let saved: SavedNetwork[] = [];
  let loading = true;
  let error = '';
  let deletingId = '';
  let deleteError = '';
  let hasInternet = false;
  let connectivityKnown = false;

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

  onMount(() => {
    refreshHome();
  });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">{$branding.product_name}</p>
    <h1 class="text-3xl font-bold">{$branding.headline}</h1>
    <p class="text-sm text-muted-foreground">{$branding.support_text}</p>
    <p class="text-sm text-muted-foreground">Pick one managed network profile. Saving a new one replaces the previous profile.</p>
  </header>

  <Card className="space-y-4">
    <h2 class="flex items-center gap-2 text-lg font-semibold">
      <Icon name="network" className="h-5 w-5 text-primary" />
      Configured network
    </h2>

    {#if loading}
      <Loading label="Loading configured network…" />
    {:else if error}
      <Alert tone="error">{error}</Alert>
    {:else if deleteError}
      <Alert tone="error">{deleteError}</Alert>
    {:else if saved.length === 0}
      {#if connectivityKnown && hasInternet}
        <Alert tone="info">
          Device is already online using automatic network settings (DHCP). You do not need to configure anything unless you want to override network behavior.
        </Alert>
      {:else}
        <Alert tone="info">No managed network is configured yet.</Alert>
      {/if}
    {:else}
      {#each saved as entry}
        <div class="rounded-md border border-border bg-white p-3">
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
              disabled={!!deletingId && deletingId !== entry.id}
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
    <Card className="space-y-3">
      <h2 class="flex items-center gap-2 text-lg font-semibold">
        <Icon name="help" className="h-5 w-5 text-primary" />
        {$branding.help_title}
      </h2>
      {#if $branding.help_text}
        <p class="text-sm text-muted-foreground">{$branding.help_text}</p>
      {/if}
      <div class="flex flex-wrap items-center gap-2">
        {#each $branding.help_links as link}
          <a class="rounded-md border border-border bg-white px-3 py-2 text-sm font-medium text-primary hover:bg-muted" href={link.url} target="_blank" rel="noreferrer">
            <span class="mr-1 inline-flex items-center"><Icon name="link" className="h-4 w-4" /></span>
            {link.label}
          </a>
        {/each}
      </div>
    </Card>
  {/if}

  <div class="flex items-center gap-3">
    <Button onclick={() => goto('/setup/new')}>
      <span class="mr-2 inline-flex items-center"><Icon name="plus" /></span>
      Add new network
    </Button>
    <Button variant="secondary" onclick={() => goto('/settings')}>
      <span class="mr-2 inline-flex items-center"><Icon name="settings" /></span>
      Settings
    </Button>
    <Button variant="secondary" onclick={refreshHome} loading={loading}>
      {#if !loading}
        <span class="mr-2 inline-flex items-center"><Icon name="refresh" /></span>
      {/if}
      Refresh
    </Button>
  </div>
</div>
