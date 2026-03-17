<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Card from '$lib/components/ui/card.svelte';
  import Alert from '$lib/components/ui/alert.svelte';
  import Loading from '$lib/components/ui/loading.svelte';
  import { networkApi } from '$lib/network/api';
  import type { ConnectionState, Transport } from '$lib/network/types';

  let transport: Transport = 'wifi';
  let testId = '';
  let waitingForApi = false;
  let lastError = '';
  let pollCount = 0;
  let cancelled = false;

  $: transport = ($page.params.transport === 'ethernet' ? 'ethernet' : 'wifi') as Transport;
  $: testId = $page.params.testId ?? '';

  async function poll() {
    while (!cancelled) {
      pollCount += 1;
      try {
        const status = (await networkApi.getStatus(testId)) as ConnectionState;
        waitingForApi = false;
        lastError = '';

        if (status.status === 'Testing') {
          await new Promise((resolve) => setTimeout(resolve, 1400));
          continue;
        }

        await goto(`/setup/new/${transport}/result/${testId}`);
        return;
      } catch (e) {
        waitingForApi = true;
        lastError = e instanceof Error ? e.message : 'Unable to reach setup service';
        await new Promise((resolve) => setTimeout(resolve, 1800));
      }
    }
  }

  onMount(() => {
    void poll();
    return () => {
      cancelled = true;
    };
  });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Testing connection</p>
    <h1 class="text-3xl font-bold">Hold tight while we switch networks</h1>
  </header>

  <Card className="space-y-3">
    {#if transport === 'wifi'}
      <p class="text-sm text-muted-foreground">
        The Raspberry Pi has one radio, so this page can briefly disconnect while we test Wi-Fi.
        Keep this tab open. We will continue automatically as soon as the device is reachable again.
      </p>
    {:else}
      <p class="text-sm text-muted-foreground">
        We are testing Ethernet. This page should usually stay connected during the test.
        After you save, setup Wi-Fi may turn off and this page can disconnect.
      </p>
    {/if}

    {#if waitingForApi}
      <Loading label="Reconnecting to your device…" />
    {:else}
      <Loading label="Running connection test…" />
    {/if}

    {#if waitingForApi}
      <Alert tone="info">
        Reconnecting to your device… {lastError}
      </Alert>
    {/if}

    <p class="text-xs text-muted-foreground">Status checks: {pollCount}</p>

    <div class="h-2 w-full overflow-hidden rounded bg-muted">
      <div class="h-full w-1/2 animate-pulse bg-primary"></div>
    </div>
  </Card>
</div>
