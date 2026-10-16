<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { getDevice } from '#lib/device.js';
  import { networkApi } from '#lib/network/api.js';
  import type { SavedNetworkDetails } from '#lib/network/types.js';
  import Card from '#lib/components/ui/card.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import Button from '#lib/components/ui/button.svelte';

  const device = getDevice();

  let details = $state<SavedNetworkDetails | null>(null);
  let error = $state('');
  let loading = $state(true);
  async function load() {
    loading = true;
    error = '';
    try { details = await networkApi.getSaved(page.params.id ?? ''); }
    catch (e) { error = e instanceof Error ? e.message : 'Could not load this network.'; }
    finally { loading = false; }
  }
  onMount(() => { void load(); });
</script>

<svelte:head><title>Network configuration · {device.hostname ?? 'Network setup'}</title></svelte:head>

<main class="mx-auto max-w-2xl space-y-6">
  <header class="space-y-3">
    <a href="/" class="inline-flex min-h-11 items-center text-sm font-medium text-primary hover:underline">← Back to networks</a>
    <h1 class="break-words text-3xl font-semibold">{details?.settings.ssid ?? details?.network.display_name ?? 'Network configuration'}</h1>
    <p class="text-muted-foreground">Saved settings for this connection.</p>
  </header>
  {#if loading}
    <Loading label="Loading network configuration…" />
  {:else if error}
    <Alert tone="error">{error}</Alert>
    <Button onclick={load}>Try again</Button>
  {:else if details}
    <Card className="space-y-4">
      <h2 class="text-lg font-semibold">Connection</h2>
      <dl class="grid grid-cols-2 gap-3 text-sm [&>dd]:break-words">
        <dt>Type</dt><dd>{details.network.transport === 'wifi' ? 'Wi-Fi' : 'Ethernet'}</dd>
        {#if details.settings.ssid}<dt>Network name</dt><dd>{details.settings.ssid}</dd>{/if}
        <dt>Authentication</dt><dd>{details.security === 'enterprise' ? 'Enterprise (802.1X)' : details.security === 'personal' ? 'Wi-Fi password' : 'None'}</dd>
        {#if details.settings.eap_login}<dt>Identity</dt><dd>{details.settings.eap_login}</dd>{/if}
        {#if details.security !== 'open'}<dt>Password</dt><dd>Hidden · kept when editing</dd>{/if}
      </dl>
    </Card>
    <Card className="space-y-4">
      <h2 class="text-lg font-semibold">IPv4 and DNS</h2>
      <dl class="grid grid-cols-2 gap-3 text-sm [&>dd]:break-words">
        <dt>IP assignment</dt><dd>{details.settings.ipv4?.method === 'manual' ? 'Manual' : 'Automatic (DHCP)'}</dd>
        {#each details.settings.ipv4?.address_data ?? [] as address (`${address.address}/${address.prefix}`)}
          <dt>Address</dt><dd>{address.address}/{address.prefix}</dd>
        {/each}
        <dt>Gateway</dt><dd>{details.settings.ipv4?.gateway || (details.settings.ipv4?.method === 'auto' ? 'Provided by DHCP' : 'None')}</dd>
        <dt>DNS servers</dt><dd>{details.settings.ipv4?.dns?.join(', ') || (details.settings.ipv4?.method === 'auto' ? 'Provided by DHCP' : 'None')}</dd>
      </dl>
    </Card>
    {#if details.edit_unavailable}
      <Alert tone="info">{details.edit_unavailable}</Alert>
    {:else}
      <div class="flex justify-end">
        <a href={`/setup/new/${details.network.transport}?edit=${encodeURIComponent(details.network.id)}`} class="inline-flex min-h-11 items-center rounded-md bg-primary px-4 text-sm font-semibold text-primary-foreground hover:opacity-90">Edit connection</a>
      </div>
    {/if}
  {/if}
</main>
