<script lang="ts">
  import { getDevice } from '#lib/device.js';
  import PageActions from '#lib/components/ui/page-actions.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Card from '#lib/components/ui/card.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import ConnectionReport from '#lib/components/network/connection-report.svelte';
  import { networkApi } from '#lib/network/api.js';
  import type { CurrentConnectionCheck } from '#lib/network/types.js';

  const device = getDevice();

  let results = $state<CurrentConnectionCheck[] | null>(null);
  let checking = $state(false);
  let error = $state('');
  let checkedAt = $state<Date | null>(null);
  const checkedTime = $derived(checkedAt
    ? new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'long' }).format(checkedAt)
    : null);

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

<svelte:head><title>Connectivity • netsetup{device.hostname ? ` • ${device.hostname}` : ''}</title></svelte:head>

<main class="mx-auto max-w-2xl space-y-4">
  <header class="space-y-2">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-3xl font-semibold tracking-tight">Connectivity</h1>
    </div>
    <p class="text-sm text-muted-foreground">Check your current connections without changing network settings.</p>
    {#if checkedAt}<p class="text-xs text-muted-foreground">{device.hostname ?? 'Device'} · <time datetime={checkedAt.toISOString()}>{checkedTime}</time></p>{/if}
    {#if checking}
      <p role="status" class="text-sm text-muted-foreground">Checking active connections. This may take a minute.</p>
    {:else if error}
      <Alert tone="error">{error}</Alert>
    {:else if results?.length === 0}
      <Alert tone="info">No active Wi-Fi or Ethernet connection to check. The device’s own network is used to open netsetup.</Alert>
      <a href="/setup/new" class="inline-flex min-h-11 items-center font-medium text-primary hover:underline">Add a network →</a>
    {/if}
  </header>
  {#each results ?? [] as result (result.interface)}
    <Card className="space-y-4">
      <div class="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
        <h2 class="break-words text-lg font-semibold">{result.name}</h2>
        <p class="text-xs text-muted-foreground">{#if result.configuration}{result.configuration.transport === 'wifi' ? 'Wi-Fi' : 'Ethernet'} · {/if}<span class="font-mono">{result.interface}</span></p>
      </div>
      {#if result.configuration}
        {@const config = result.configuration}
        <section aria-label="Current network configuration" class="space-y-2">
          <dl class="network-details">
            {#if config.ssid !== null && config.ssid !== result.name}<div><dt>SSID</dt><dd>{config.ssid}</dd></div>{/if}
            <div><dt>IP assignment</dt><dd>{config.ipv4_method === 'auto' ? 'Automatic (DHCP)' : config.ipv4_method === 'manual' ? 'Manual' : config.ipv4_method || 'Unknown'}</dd></div>
            <div>
              <dt>IPv4</dt>
              <dd>{#each config.addresses as address (`${address.address}/${address.prefix}`)}<span class="block font-mono">{address.address}/{address.prefix}</span>{:else}Not assigned{/each}</dd>
            </div>
            <div><dt>Gateway</dt><dd class="font-mono">{config.gateway || 'Not configured'}</dd></div>
            <div><dt>DNS servers</dt><dd class="font-mono">{config.dns.join(', ') || 'Not configured'}</dd></div>
            <div><dt>MAC address</dt><dd class="font-mono">{config.mac_address || 'Unavailable'}</dd></div>
          </dl>
        </section>
      {:else if result.configuration_error}
        <Alert tone="warning">Network details unavailable: {result.configuration_error}</Alert>
      {/if}
      {#if result.report}
        <ConnectionReport report={result.report} compact />
      {:else}
        <Alert tone="error">{result.error ?? 'Could not check this connection.'}</Alert>
      {/if}
    </Card>
  {/each}
  <PageActions>
    {#snippet secondary()}<a href="/help" class="inline-flex min-h-11 items-center text-sm font-medium text-primary hover:underline">Help with results</a>{/snippet}
    {#snippet primary()}<Button onclick={check} loading={checking}>{checking ? 'Checking…' : 'Check connectivity'}</Button>{/snippet}
  </PageActions>
</main>


<style>
  .network-details { font-size: 0.875rem; }
  .network-details > div { display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 3fr); gap: 1rem; padding-block: 0.375rem; }
  .network-details > div + div { border-top: 1px solid hsl(var(--border) / 0.6); }
  .network-details dt { color: hsl(var(--muted-foreground)); }
  .network-details dd { min-width: 0; overflow-wrap: anywhere; text-align: right; }
</style>
