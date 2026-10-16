<script lang="ts">
  import { getDevice } from '#lib/device.js';
  const device = getDevice();
  import Button from '#lib/components/ui/button.svelte';
  import Card from '#lib/components/ui/card.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import ConnectionReport from '#lib/components/network/connection-report.svelte';
  import { networkApi } from '#lib/network/api.js';
  import type { CurrentConnectionCheck } from '#lib/network/types.js';

  let results = $state<CurrentConnectionCheck[] | null>(null);
  let checking = $state(false);
  let error = $state('');
  let checkedAt = $state<Date | null>(null);

  async function check() {
    if (checking) return;
    checking = true;
    error = '';
    results = null;
    checkedAt = null;
    try {
      results = await networkApi.checkCurrentConnections();
      checkedAt = new Date();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not check connectivity. Try again.';
    } finally {
      checking = false;
    }
  }
</script>

<svelte:head><title>Connectivity · {device.hostname ?? 'Network setup'}</title></svelte:head>

<main class="mx-auto max-w-2xl space-y-6">
  <header class="space-y-2">
    <h1 class="text-3xl font-bold tracking-tight">Connectivity</h1>
    <p class="text-muted-foreground">Check whether this device can reach its router, the internet, and Reshape on its current Wi-Fi or Ethernet connection.</p>
  </header>
  <Card className="space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <p class="text-sm text-muted-foreground">Your network settings stay as they are.</p>
      <Button onclick={check} loading={checking}>{checking ? 'Checking…' : 'Check connectivity'}</Button>
    </div>
    {#if checking}
      <p role="status" class="text-sm text-muted-foreground">Checking active connections. This may take a minute.</p>
    {:else if error}
      <Alert tone="error">{error}</Alert>
    {:else if results !== null}
      <p class="text-xs text-muted-foreground">Checked at {checkedAt?.toLocaleTimeString()}</p>
      {#if results.length === 0}
        <Alert tone="info">No active Wi-Fi or Ethernet connection to check. The device’s setup Wi-Fi is only for configuration.</Alert>
        <a href="/setup/new" class="inline-flex min-h-11 items-center font-medium text-primary hover:underline">Add a network →</a>
      {/if}
    {/if}
  </Card>
  {#each results ?? [] as result (result.interface)}
    <Card className="space-y-4">
      <h2 class="break-words text-lg font-semibold">{result.name}</h2>
      {#if result.report}
        <ConnectionReport report={result.report} />
      {:else}
        <p class="text-sm text-muted-foreground">{result.interface}</p>
        <Alert tone="error">{result.error ?? 'Could not check this connection.'}</Alert>
      {/if}
    </Card>
  {/each}
  <a href="/help" class="inline-flex min-h-11 items-center text-sm font-medium text-primary hover:underline">Help understanding the results →</a>
</main>
