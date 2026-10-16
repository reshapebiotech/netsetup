<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import Card from '#lib/components/ui/card.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import { networkApi } from '#lib/network/api.js';
  import { resultSummary } from '#lib/network/results.js';

  const testId = $derived(page.params.testId ?? '');
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
      message = 'We could not confirm whether saving completed. Check that the device is online, or reconnect to setup Wi-Fi to check its status.';
    }
  }

  async function saveNow() {
    status = 'saving';
    message = 'Applying and verifying your connection…';
    try {
      const current = await networkApi.getStatus(testId);
      if (current.status === 'Connected') {
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
    void saveNow();
    return () => { cancelled = true; };
  });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Final handoff</p>
    <h1 class="text-3xl font-bold">Applying your saved network</h1>
  </header>
  <Card className="space-y-4">
    <Alert tone={status === 'error' ? 'error' : 'info'}>{message}</Alert>
    {#if status === 'saving'}
      <Loading label="Waiting for confirmation…" />
      <p class="text-sm text-muted-foreground">
        Setup Wi-Fi may disappear as the device switches to your network.
        If verification fails, the previous connection will be restored.
        Reconnect to setup Wi-Fi if it returns.
      </p>
    {:else if status === 'unconfirmed'}
      <Button onclick={saveNow}>Check again</Button>
    {:else if status === 'error'}
      <Button onclick={() => goto(`/setup/new/${page.params.transport}`)}>Configure again</Button>
    {:else}
      <Button onclick={() => goto('/')}>Return home</Button>
    {/if}
  </Card>
</div>
