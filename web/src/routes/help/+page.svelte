<script lang="ts">
  import { getDevice } from '#lib/device.js';
  import { onMount } from 'svelte';
  import { networkApi } from '#lib/network/api.js';
  import { branding } from '#lib/branding.js';

  const device = getDevice();

  let setupWifiName = $state<string | null>(null);
  let bootGraceSeconds = $state<number | null>(null);
  let bootGrace = $derived(
    bootGraceSeconds === null ? null : bootGraceSeconds > 0 && bootGraceSeconds % 60 === 0
      ? `${bootGraceSeconds / 60} ${bootGraceSeconds === 60 ? 'minute' : 'minutes'}`
      : `${bootGraceSeconds} ${bootGraceSeconds === 1 ? 'second' : 'seconds'}`
  );

  onMount(() => {
    void networkApi.getApControllerStatus().then(status => {
      setupWifiName = typeof status.ssid === 'string' && status.ssid.length > 0 ? status.ssid : null;
      bootGraceSeconds = Number.isSafeInteger(status.boot_grace_seconds) && status.boot_grace_seconds >= 0
        ? status.boot_grace_seconds : null;
    }).catch(() => {
      bootGraceSeconds = null;
    });
  });
</script>

<svelte:head>
  <title>Help • netsetup{device.hostname ? ` • ${device.hostname}` : ''}</title>
</svelte:head>

<main class="mx-auto max-w-2xl">
  <header class="mb-6 space-y-3">
    <a href="/" class="inline-flex min-h-11 items-center text-sm font-medium text-primary hover:underline">← Back to netsetup</a>
    <h1 class="text-3xl font-semibold tracking-tight">netsetup help</h1>
    <p class="rounded-lg border border-border bg-card p-4 text-sm leading-relaxed text-muted-foreground">
      To open <strong class="text-foreground">netsetup</strong>, connect to <strong class="text-foreground"><bdi>{setupWifiName ?? 'the device’s network'}</bdi></strong> in your phone’s Wi-Fi settings.
    </p>
  </header>

  <div class="overflow-hidden rounded-xl border border-border bg-card ">
    <details class="help-topic" name="help-topic">
      <summary>
        <span><span class="topic-title">Connection lost during a test</span><span class="topic-hint">During a connection test or after saving</span></span>
        <span class="chevron" aria-hidden="true">⌄</span>
      </summary>
      <div class="answer">
        <p>The device briefly disconnects to try your network. <strong>Keep netsetup open.</strong></p>
        <ol class="steps">
          <li><span aria-hidden="true">1</span><div>Wait up to two minutes for the device’s network to return.</div></li>
          <li><span aria-hidden="true">2</span><div>Rejoin {setupWifiName ?? 'the device’s network'} in your phone’s Wi-Fi settings.</div></li>
          <li><span aria-hidden="true">3</span><div>Return to netsetup to review and save the result.</div></li>
        </ol>
        <p class="text-muted-foreground">Popup closed? While connected to the device’s network, open <a class="font-medium text-primary underline underline-offset-4" href="http://10.42.0.1/">10.42.0.1</a> in your browser.</p>
        <aside class="border-l-2 border-primary/40 pl-4">
          <p class="font-semibold">Already saved successfully?</p>
          <p class="mt-1 text-muted-foreground">The direct connection may close when the device joins your network. Reconnect your phone to your usual Wi-Fi.</p>
        </aside>
      </div>
    </details>

    <details class="help-topic" name="help-topic">
      <summary>
        <span><span class="topic-title">Some services couldn’t be reached</span><span class="topic-hint">What to do about partial connectivity</span></span>
        <span class="chevron" aria-hidden="true">⌄</span>
      </summary>
      <div class="answer">
        <p>Take a screenshot of the results and share it with your IT department or Reshape support. It shows which services need attention and can help identify firewall or network restrictions.</p>
      </div>
    </details>

    <details class="help-topic" name="help-topic">
      <summary>
        <span><span class="topic-title">I can’t open netsetup</span><span class="topic-hint">When it opens and how to get back in</span></span>
        <span class="chevron" aria-hidden="true">⌄</span>
      </summary>
      <div class="answer">
        <p><strong>The device opens its own network when it cannot get online at startup.</strong> If you need to change networks, restart the device to trigger that check again.</p>
        {#if bootGrace !== null}
          <p>After the device boots, saved networks have <strong>{bootGrace}</strong> to provide internet access. If none does, the device’s network opens.</p>
        {:else}
          <p>After the device boots, it gives saved networks a startup window to provide internet access. If none does, the device’s network opens.</p>
        {/if}
        <p>After startup, the device’s network also opens if all saved Wi-Fi and Ethernet profiles have been removed. The device’s own access point does not count. A saved Ethernet profile still counts while disconnected or waiting for DHCP.</p>
        <p class="text-muted-foreground">Losing internet later does not automatically open the device’s network while saved networks remain. Connection tests and saves pause this check.</p>
        <p class="text-muted-foreground">If the device is already online, you can open netsetup over its existing network. Direct access in Settings keeps the device’s own network available until you turn it off. It can run alongside Ethernet; testing another Wi-Fi network can still interrupt it.</p>
      </div>
    </details>
  </div>

  <footer class="mt-6 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
    {#each $branding.help_links as link (link.url)}
      <a class="inline-flex min-h-11 items-center font-medium text-primary hover:underline" href={link.url} target="_blank" rel="noreferrer">Online guide ↗</a>
    {/each}
    <span class="text-muted-foreground">Requires internet</span>
  </footer>
</main>

<style>
  .help-topic + .help-topic { border-top: 1px solid hsl(var(--border)); }
  summary { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 1.25rem; cursor: pointer; list-style: none; }
  summary::-webkit-details-marker { display: none; }
  summary:hover { background: hsl(var(--muted) / 0.25); }
  summary:focus-visible { outline: 2px solid hsl(var(--primary)); outline-offset: -4px; border-radius: 0.5rem; }
  .topic-title { display: block; font-size: 1rem; font-weight: 600; }
  .topic-hint { display: block; margin-top: 0.25rem; font-size: 0.875rem; color: hsl(var(--muted-foreground)); }
  .chevron { color: hsl(var(--primary)); font-size: 1.5rem; line-height: 1; }
  details[open] .chevron { transform: rotate(180deg); }
  .answer { display: grid; gap: 1.25rem; padding: 0.25rem 1.25rem 1.5rem; font-size: 0.875rem; line-height: 1.6; }
  .steps { display: grid; gap: 1rem; }
  .steps li { display: flex; align-items: baseline; gap: 0.75rem; }
  .steps li > span { display: inline-flex; flex-shrink: 0; align-items: center; justify-content: center; width: 1.5rem; height: 1.5rem; border-radius: 50%; background: hsl(var(--muted)); color: hsl(var(--primary)); font-size: 0.75rem; font-weight: 600; }
</style>
