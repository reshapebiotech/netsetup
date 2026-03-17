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

  async function refresh() {
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

  onMount(refresh);
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

  <div class="flex items-center gap-3">
    <Button onclick={() => goto('/setup/new')}>Add new network</Button>
    <Button variant="secondary" onclick={refresh} loading={loading}>Refresh</Button>
  </div>
</div>
