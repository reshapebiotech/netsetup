<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import Card from '#lib/components/ui/card.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import Icon from '#lib/components/ui/icon.svelte';
  import { networkApi } from '#lib/network/api.js';
  import { resultSummary } from '#lib/network/results.js';
  import type { ConnectionState } from '#lib/network/types.js';

  const transport = $derived(page.params.transport === 'ethernet' ? 'ethernet' : 'wifi');
  const testId = $derived(page.params.testId ?? '');
  let status = $state<ConnectionState | null>(null);
  let loading = $state(true);
  let movingToSave = $state(false);
  let error = $state('');

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
    error = '';
    try {
      await networkApi.discardTest(testId);
      await goto(`/setup/new/${transport}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not discard this test';
    }
  }

  onMount(() => { void loadStatus(); });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Test result</p>
    <h1 class="flex items-center gap-2 text-3xl font-bold">
      <Icon name={transport === 'wifi' ? 'wifi' : 'ethernet'} className="h-6 w-6 text-primary" />
      {transport === 'wifi' ? 'Wi-Fi' : 'Ethernet'} test complete
    </h1>
  </header>
  <Card className="space-y-4">
    {#if loading}
      <Loading label="Loading test result…" />
    {:else if error}
      <Alert tone="error">{error}</Alert>
      <Button onclick={loadStatus}>Reload result</Button>
    {:else if status?.status === 'Connected'}
      <Alert tone="info">{resultSummary(status.detail)}</Alert>
      <dl class="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
        <dt>Network connected</dt><dd>{status.detail.interface} · {status.detail.address}</dd>
        <dt>Gateway reachable</dt>
        <dd>{status.detail.gateway_reachable === null ? 'No gateway configured' : status.detail.gateway_reachable ? 'Verified' : 'Could not verify'}</dd>
        <dt>Internet access</dt><dd>{status.detail.internet_verified ? 'Verified' : 'Could not verify'}</dd>
        <dt>Reshape service</dt><dd>{status.detail.reshape_reachable ? 'Reachable' : 'Could not reach'}</dd>
      </dl>
      {#if !status.detail.reshape_reachable}
        <p class="text-sm text-muted-foreground">
          You can save this connection, but the device may be unable to communicate with Reshape.
          A firewall, DNS settings, or a service outage may prevent these checks from succeeding.
        </p>
      {/if}
      <p class="text-sm text-muted-foreground">Your previous connection has been restored. Saving will apply these settings and replace the previous {transport === 'wifi' ? 'Wi-Fi' : 'Ethernet'} profile.</p>
      <div class="flex flex-wrap items-center gap-3">
        <Button onclick={save} loading={movingToSave}>
          <span class="mr-2 inline-flex items-center"><Icon name="save" /></span>
          {status.detail.reshape_reachable ? 'Save connection' : 'Save anyway'}
        </Button>
        <Button variant="secondary" onclick={retry}>Try a different network</Button>
      </div>
    {:else if status?.status === 'Failed'}
      <Alert tone="error">{status.detail}</Alert>
      <Button onclick={retry}>Try again</Button>
    {:else if status?.status === 'Saved'}
      <Alert tone="info">This connection has already been saved.</Alert>
      <Button onclick={() => goto('/')}>Back to home</Button>
    {:else}
      <Alert tone="info">The network operation is still in progress.</Alert>
      <Button onclick={() => goto(status?.status === 'Saving'
        ? `/setup/new/${transport}/save-handoff/${testId}`
        : `/setup/new/${transport}/testing/${testId}`)}>Check progress</Button>
    {/if}
  </Card>
</div>
