<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import PageActions from '#lib/components/ui/page-actions.svelte';
  import Card from '#lib/components/ui/card.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import { networkApi } from '#lib/network/api.js';
  import { resultSummary } from '#lib/network/results.js';
  import { deviceHandoffUrl } from '#lib/network/handoff.js';

  const testId = $derived(page.params.testId ?? '');
  const transport = $derived(page.params.transport === 'ethernet' ? 'ethernet' : 'wifi');
  let deviceAddress = $state<string | null>(null);
  const deviceUrl = $derived(deviceHandoffUrl(deviceAddress, transport, testId));
  let status = $state<'saving' | 'saved' | 'unconfirmed' | 'error'>('saving');
  let message = $state('Applying and verifying your connection…');
  let cancelled = false;

  async function checkOutcome() {
    const deadline = Date.now() + 180_000;
    while (!cancelled && Date.now() < deadline) {
      try {
        const current = await networkApi.getStatus(testId);
        if (cancelled) return;
        if (current.status === 'Saved') {
          deviceAddress = current.detail.address;
          status = 'saved';
          message = `Connection saved. ${resultSummary(current.detail)}.`;
          return;
        }
        if (current.status === 'Failed') {
          status = 'error';
          message = current.detail;
          return;
        }
        if (current.status === 'Connected') break;
      } catch {
        // Poll again when the device becomes reachable after switching networks.
      }
      await new Promise((resolve) => setTimeout(resolve, 1500));
    }
    if (!cancelled) {
      status = 'unconfirmed';
      message = 'We could not confirm whether saving completed. Reconnect to the device and open netsetup to check its status.';
    }
  }

  async function saveNow() {
    status = 'saving';
    message = 'Applying and verifying your connection…';
    try {
      const current = await networkApi.getStatus(testId);
      if (current.status === 'Connected') {
        deviceAddress = current.detail.address;
        await networkApi.saveTest(testId);
      } else if (current.status === 'Failed') {
        status = 'error';
        message = current.detail;
        return;
      }
    } catch {
      // A lost response is inconclusive; retrieve the actual operation status.
    }
    await checkOutcome();
  }

  onMount(() => {
    deviceAddress = page.state.deviceAddress ?? null;
    void saveNow();
    return () => { cancelled = true; };
  });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Final handoff</p>
    <h1 class="text-3xl font-semibold">Applying your saved network</h1>
  </header>
  <Card className="space-y-4">
    <Alert tone={status === 'error' ? 'error' : 'info'}>{message}</Alert>
    {#if status === 'saving'}
      <Loading label="Waiting for confirmation…" />
      <p class="text-sm text-muted-foreground">
        Your connection to netsetup may drop as the device switches to your network.
        If verification fails, the previous connection will be restored.
        Reconnect to the device’s own network if it returns.
      </p>
    {/if}
    {#if deviceUrl && status !== 'error'}
      <div class="space-y-3 border-t border-border pt-4">
        <p class="text-sm text-muted-foreground">Join the same network as the device, then open netsetup.</p>
        <p class="break-all font-mono text-sm">http://{deviceAddress}:3000/</p>
        {#if status !== 'saved'}<p class="text-xs text-muted-foreground">This is the address from the connection test. Saving has not been confirmed.</p>{/if}
      </div>
    {/if}
  </Card>
  {#if deviceUrl || status !== 'saving'}
    <PageActions>
      {#snippet secondary()}
        {#if deviceUrl && status === 'saved'}<Button variant="secondary" onclick={() => goto('/')}>Return home</Button>
        {:else if deviceUrl && status === 'unconfirmed'}<Button variant="secondary" onclick={saveNow}>Check again</Button>{/if}
      {/snippet}
      {#snippet primary()}
        {#if status === 'error'}<Button onclick={() => goto(`/setup/new/${page.params.transport}`)}>Configure again</Button>
        {:else if deviceUrl}<a href={deviceUrl} class="inline-flex min-h-11 items-center rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground">Open netsetup</a>
        {:else if status === 'unconfirmed'}<Button onclick={saveNow}>Check again</Button>
        {:else if status === 'saved'}<Button onclick={() => goto('/')}>Return home</Button>{/if}
      {/snippet}
    </PageActions>
  {/if}
</div>
