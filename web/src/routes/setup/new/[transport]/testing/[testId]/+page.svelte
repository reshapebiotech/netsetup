<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import Card from '#lib/components/ui/card.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Icon from '#lib/components/ui/icon.svelte';
  import { networkApi } from '#lib/network/api.js';

  const transport = $derived(page.params.transport === 'ethernet' ? 'ethernet' : 'wifi');
  const testId = $derived(page.params.testId ?? '');
  let waitingForApi = $state(false);
  let timedOut = $state(false);
  let cancelled = false;

  async function poll() {
    const deadline = Date.now() + 180_000;
    while (!cancelled && Date.now() < deadline) {
      try {
        const status = await networkApi.getStatus(testId);
        if (cancelled) return;
        waitingForApi = false;
        if (status.status !== 'Testing') {
          await goto(`/setup/new/${transport}/result/${testId}`);
          return;
        }
      } catch {
        waitingForApi = true;
      }
      await new Promise((resolve) => setTimeout(resolve, 1500));
    }
    if (!cancelled) timedOut = true;
  }

  onMount(() => {
    void poll();
    return () => { cancelled = true; };
  });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Testing connection</p>
    <h1 class="flex items-center gap-2 text-3xl font-bold">
      <Icon name={transport === 'wifi' ? 'wifi' : 'ethernet'} className="h-6 w-6 text-primary" />
      Hold tight while we switch networks
    </h1>
  </header>
  <Card className="space-y-4">
    <p class="text-sm text-muted-foreground">
      We will check the network, gateway, internet, and Reshape service, then restore your previous connection so you can review the results.
    </p>
    {#if transport === 'wifi'}
      <p class="text-sm text-muted-foreground">
        Setup Wi-Fi may disappear for up to two minutes. Keep this tab open.
        When it returns, reconnect to the same setup Wi-Fi if your phone or computer does not reconnect automatically.
      </p>
    {/if}
    {#if timedOut}
      <Alert tone="error">We could not retrieve the result. Reconnect to setup Wi-Fi and check again. If it does not return, reboot the device.</Alert>
      <Button onclick={() => goto(`/setup/new/${transport}/result/${testId}`)}>Check result</Button>
    {:else}
      <Loading label={waitingForApi ? 'Reconnecting to your device…' : 'Testing and restoring your connection…'} />
    {/if}
  </Card>
</div>
