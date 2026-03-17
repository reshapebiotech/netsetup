<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Alert from '$lib/components/ui/alert.svelte';
  import { networkApi } from '$lib/network/api';

  let testId = '';
  let status: 'saving' | 'saved' | 'error' = 'saving';
  let message = 'Saving connection…';

  $: testId = $page.params.testId ?? '';

  async function saveNow() {
    status = 'saving';
    message = 'Saving connection…';
    try {
      await networkApi.saveTest(testId);
      status = 'saved';
      message = 'Saved. This page may disconnect briefly while network services restart.';
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to save connection';
      if (msg.toLowerCase().includes('timed out') || msg.toLowerCase().includes('abort')) {
        status = 'saved';
        message = 'Connection is switching. This page may disconnect briefly, which is expected.';
      } else {
        status = 'error';
        message = msg;
      }
    }
  }

  onMount(() => {
    void saveNow();
  });
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">Final handoff</p>
    <h1 class="text-3xl font-bold">Applying your saved network</h1>
  </header>

  <Card className="space-y-4">
    {#if status === 'error'}
      <Alert tone="error">{message}</Alert>
      <div class="flex items-center gap-3">
        <Button onclick={saveNow}>Retry save</Button>
        <Button variant="secondary" onclick={() => goto('/')}>Back to home</Button>
      </div>
    {:else}
      <Alert tone="info">{message}</Alert>
      <p class="text-sm text-muted-foreground">
        This is expected behavior: during save, this page can briefly lose connection.
        If the page freezes or disconnects, you can close it safely. Setup is complete.
      </p>
      <p class="text-sm text-muted-foreground">
        Next steps: join the same LAN as the device and continue from your normal machine management entrypoint.
      </p>
      {#if status === 'saved'}
        <Button onclick={() => goto('/')}>Return home</Button>
      {/if}
    {/if}
  </Card>
</div>
