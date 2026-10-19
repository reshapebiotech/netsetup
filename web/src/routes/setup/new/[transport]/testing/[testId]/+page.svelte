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
  const setupWifiName = $derived(page.state.setupWifiName ?? null);
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
    <h1 class="text-3xl font-semibold">
      {transport === 'wifi' ? 'Stay on setup Wi-Fi' : 'Testing your Ethernet connection'}
    </h1>
  </header>
  <Card className="space-y-5">
    {#if transport === 'wifi'}
      <div class="flex items-start gap-3 rounded-lg border border-warning/30 bg-warning/8 p-4">
        <Icon name="wifi" className="mt-1 h-6 w-6 text-warning" />
        <div class="min-w-0 space-y-1">
          <p class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Wi-Fi network to stay on</p>
          <p class="break-words text-xl font-semibold"><bdi>{setupWifiName ?? 'The device’s setup Wi-Fi'}</bdi></p>
        </div>
      </div>
      <ol class="space-y-4">
        <li class="flex gap-3">
          <span aria-hidden="true" class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold">1</span>
          <div class="space-y-1">
            <p class="font-semibold">Keep this page open.</p>
            <p class="text-sm text-muted-foreground">Setup Wi-Fi may disappear for up to two minutes during the test.</p>
          </div>
        </li>
        <li class="flex gap-3">
          <span aria-hidden="true" class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold">2</span>
          <div class="space-y-1">
            <p class="font-semibold">If disconnected, rejoin {#if setupWifiName}<bdi class="break-words">{setupWifiName}</bdi>{:else}setup Wi-Fi{/if} in Wi-Fi settings.</p>
            <p class="text-sm text-muted-foreground">If asked, choose to stay connected without internet.</p>
          </div>
        </li>
      </ol>
      <p class="border-t border-border pt-4 text-sm font-medium">Wait for the test result before switching to your usual Wi-Fi.</p>
    {:else}
      <p class="text-sm text-muted-foreground">Keep this page open. We’ll test the connection, restore your previous settings, and show the result.</p>
    {/if}
    {#if timedOut}
      <Alert tone="error">We could not retrieve the result. Reconnect to setup Wi-Fi and check again. If it does not return, reboot the device.</Alert>
      <Button onclick={() => goto(`/setup/new/${transport}/result/${testId}`)}>Check result</Button>
    {:else}
      <Loading label={waitingForApi ? 'Waiting to reconnect to your device…' : 'Testing and restoring your connection…'} />
    {/if}
  </Card>
</div>
