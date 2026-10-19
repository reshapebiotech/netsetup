<script lang="ts">
  import { getDevice } from '#lib/device.js';
  import { onMount } from 'svelte';
  import ConnectionReport from '#lib/components/network/connection-report.svelte';
  import type { ConnectivityReport, EndpointGroupConfig } from '#lib/network/types.js';
  import { networkApi } from '#lib/network/api.js';
  import { branding } from '#lib/branding.js';

  const device = getDevice();

  let groups = $state<EndpointGroupConfig[]>([]);
  const examples = $derived([
    { explanation: 'Every configured endpoint responded. Choose Save connection to keep these settings.', report: exampleReport('full') },
    { explanation: 'The device has an address and its router responds, but services could not be reached. Choose Save anyway if a local connection is what you need, or ask IT to check access.', report: exampleReport('none') },
    { explanation: 'Some endpoints responded and others did not. Use the group names and individual results to see what is affected. Required groups are needed for normal operation.', report: exampleReport('partial') }
  ]);
  function exampleReport(level: 'full' | 'partial' | 'none'): ConnectivityReport {
    return {
      interface: 'wlan0', address: '192.168.1.42', gateway_reachable: true,
      internet_verified: level !== 'none',
      endpoint_groups: groups.map(group => ({ ...group, endpoints: group.endpoints.map((endpoint, index) => ({
        ...endpoint, reachable: level === 'full' || (level === 'partial' && (group.required || index < group.endpoints.length - 1))
      })) }))
    };
  }

  let selectedExample = $state(1);
  const example = $derived(examples[selectedExample]);

  let setupWifiName = $state<string | null>(null);
  let bootGraceSeconds = $state<number | null>(null);
  let bootGrace = $derived(
    bootGraceSeconds === null ? null : bootGraceSeconds > 0 && bootGraceSeconds % 60 === 0
      ? `${bootGraceSeconds / 60} ${bootGraceSeconds === 60 ? 'minute' : 'minutes'}`
      : `${bootGraceSeconds} ${bootGraceSeconds === 1 ? 'second' : 'seconds'}`
  );

  onMount(() => {
    void networkApi.getConnectivityGroups().then(value => { groups = value; }).catch(() => {});
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
  <title>Network setup help · {device.hostname ?? 'Network setup'}</title>
</svelte:head>

<main class="mx-auto max-w-2xl">
  <header class="mb-6 space-y-3">
    <a href="/" class="inline-flex min-h-11 items-center text-sm font-medium text-primary hover:underline">← Back to network setup</a>
    <h1 class="text-3xl font-semibold tracking-tight">Network setup help</h1>
    <div class="space-y-2 rounded-lg border border-border bg-card p-4">
      {#if setupWifiName}
        <p class="text-sm text-muted-foreground">This device’s Wi-Fi network</p>
        <p class="text-xl font-semibold break-words"><bdi>{setupWifiName}</bdi></p>
      {/if}
      <p class="text-sm leading-relaxed text-muted-foreground">The device creates its own Wi-Fi network so you can connect your phone and configure it. We call this <strong class="text-foreground">setup Wi-Fi</strong> below.</p>
    </div>
  </header>

  <div class="overflow-hidden rounded-xl border border-border bg-card ">
    <details class="help-topic" name="help-topic">
      <summary>
        <span><span class="topic-title">Setup Wi-Fi disappeared</span><span class="topic-hint">During a connection test or after saving</span></span>
        <span class="chevron" aria-hidden="true">⌄</span>
      </summary>
      <div class="answer">
        <p>The device briefly leaves setup Wi-Fi to try your network. <strong>Keep the test tab open.</strong></p>
        <ol class="steps">
          <li><span aria-hidden="true">1</span><div>Wait up to two minutes for setup Wi-Fi to return.</div></li>
          <li><span aria-hidden="true">2</span><div>Rejoin the same setup Wi-Fi in your phone’s Wi-Fi settings.</div></li>
          <li><span aria-hidden="true">3</span><div>Return to the test tab to review and save the result.</div></li>
        </ol>
        <p class="text-muted-foreground">Popup closed? While connected to setup Wi-Fi, open <a class="font-medium text-primary underline underline-offset-4" href="http://10.42.0.1/">10.42.0.1</a> in your browser.</p>
        <aside class="border-l-2 border-primary/40 pl-4">
          <p class="font-semibold">Already saved successfully?</p>
          <p class="mt-1 text-muted-foreground">Setup Wi-Fi may close when the device joins your network. Reconnect your phone to your usual Wi-Fi.</p>
        </aside>
      </div>
    </details>

    <details class="help-topic" name="help-topic">
      <summary>
        <span><span class="topic-title">What does my test result mean?</span><span class="topic-hint">Understand the checks and whether to save</span></span>
        <span class="chevron" aria-hidden="true">⌄</span>
      </summary>
      <div class="answer">
        <div class="space-y-2">
          <label for="example-result" class="block font-semibold">Choose the result you saw</label>
          <select id="example-result" bind:value={selectedExample} class="w-full rounded-md border-border bg-card py-3 text-base focus:border-primary focus:ring-primary">
            <option value={0}>Full connectivity</option>
            <option value={1}>Local connection only</option>
            <option value={2}>Partial connectivity</option>
          </select>
        </div>
        <p>{example.explanation}</p>
        <figure class="overflow-hidden rounded-lg border border-border">
          <figcaption class="border-b border-border bg-muted/40 px-4 py-2 text-xs font-medium text-muted-foreground">Example result · illustration only</figcaption>
          <div class="p-4"><ConnectionReport report={example.report} /></div>
        </figure>
        <p class="text-muted-foreground">“Could not verify” can also mean a firewall or DNS setting blocked the check. It does not always mean the network is disconnected.</p>
      </div>
    </details>

    <details class="help-topic" name="help-topic">
      <summary>
        <span><span class="topic-title">I can’t find setup Wi-Fi</span><span class="topic-hint">When it opens and how to get back in</span></span>
        <span class="chevron" aria-hidden="true">⌄</span>
      </summary>
      <div class="answer">
        <p><strong>Setup Wi-Fi opens when the device cannot get online at startup.</strong> If you need to change networks, restart the device to trigger that check again.</p>
        {#if bootGrace !== null}
          <p>After the device boots, saved networks have <strong>{bootGrace}</strong> to provide internet access. If none does, setup Wi-Fi opens.</p>
        {:else}
          <p>After the device boots, it gives saved networks a startup window to provide internet access. If none does, setup Wi-Fi opens.</p>
        {/if}
        <p>After startup, setup Wi-Fi also opens if all saved Wi-Fi and Ethernet profiles have been removed. The device’s own access point does not count. A saved Ethernet profile still counts while disconnected or waiting for DHCP.</p>
        <p class="text-muted-foreground">Losing internet later does not automatically open setup Wi-Fi while saved networks remain. Connection tests and saves pause this check.</p>
        <p class="text-muted-foreground">If the device is already online, you can open this tool over its existing network. Setup Wi-Fi test mode in Settings keeps the access point available until you turn it off. It can run alongside Ethernet; testing another Wi-Fi network can still interrupt it.</p>
      </div>
    </details>
  </div>

  <footer class="mt-6 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
    {#each $branding.help_links as link (link.url)}
      <a class="inline-flex min-h-11 items-center font-medium text-primary hover:underline" href={link.url} target="_blank" rel="noreferrer">Full setup guide ↗</a>
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
