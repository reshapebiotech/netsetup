<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Alert from '$lib/components/ui/alert.svelte';
  import Loading from '$lib/components/ui/loading.svelte';
  import { networkApi } from '$lib/network/api';
  import type { ConnectionState, Transport } from '$lib/network/types';

  let transport: Transport = 'wifi';
  let testId = '';
  let status: ConnectionState | null = null;
  let loading = true;
  let movingToSave = false;
  let error = '';

  $: transport = ($page.params.transport === 'ethernet' ? 'ethernet' : 'wifi') as Transport;
  $: testId = $page.params.testId ?? '';

  async function loadStatus() {
    loading = true;
    error = '';
    try {
      status = await networkApi.getStatus(testId);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load test status';
    } finally {
      loading = false;
    }
  }

  async function save() {
    movingToSave = true;
    await goto(`/setup/new/${transport}/save-handoff/${testId}`);
  }

  async function retry() {
    try {
      await networkApi.discardTest(testId);
    } catch {
      // best effort
    }
    await goto(`/setup/new/${transport}`);
  }

  onMount(loadStatus);
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Test result</p>
    <h1 class="text-3xl font-bold">{transport === 'wifi' ? 'Wi-Fi' : 'Ethernet'} test complete</h1>
  </header>

  <Card className="space-y-4">
    {#if loading}
      <Loading label="Loading test result…" />
    {:else if error}
      <Alert tone="error">{error}</Alert>
    {:else if status?.status === 'Connected'}
      <Alert tone="info">Connection test succeeded.</Alert>
      <p class="text-sm text-muted-foreground">Do you want to save this as the active managed network? This replaces the previous profile.</p>
      <div class="flex items-center gap-3">
        <Button onclick={save} loading={movingToSave}>Save connection</Button>
        <Button variant="secondary" onclick={retry}>Try a different network</Button>
      </div>
    {:else if status?.status === 'Failed'}
      <Alert tone="error">{status.detail}</Alert>
      <div class="flex items-center gap-3">
        <Button onclick={retry}>Try again</Button>
        <Button variant="secondary" onclick={() => goto('/')}>Back to home</Button>
      </div>
    {:else}
      <Alert tone="info">Test is still in progress. Return to testing view.</Alert>
      <Button onclick={() => goto(`/setup/new/${transport}/testing/${testId}`)}>Back to testing</Button>
    {/if}
  </Card>
</div>
