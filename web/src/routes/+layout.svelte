<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { branding, loadBranding } from '$lib/branding';
  import { networkApi } from '$lib/network/api';
  import Icon from '$lib/components/ui/icon.svelte';

  type DeviceStatus = 'checking' | 'online' | 'no_internet' | 'disconnected';

  let deviceStatus: DeviceStatus = 'checking';
  let lastUpdatedAt: Date | null = null;

  const statusText: Record<DeviceStatus, string> = {
    checking: 'Checking status…',
    online: 'Online',
    no_internet: 'No internet',
    disconnected: 'Disconnected'
  };

  const statusTooltip: Record<DeviceStatus, string> = {
    checking: 'Checking device status',
    online: 'Connected to device and device has internet',
    no_internet: 'Connected to device and device does not have internet',
    disconnected: 'Disconnected from device. Status unknown'
  };

  const statusClass: Record<DeviceStatus, string> = {
    checking: 'border-border bg-white text-foreground',
    online: 'border-primary/30 bg-primary/10 text-foreground',
    no_internet: 'border-amber-300 bg-amber-50 text-foreground',
    disconnected: 'border-destructive/30 bg-destructive/10 text-foreground'
  };

  const statusIcon: Record<DeviceStatus, 'refresh' | 'check' | 'alert'> = {
    checking: 'refresh',
    online: 'check',
    no_internet: 'alert',
    disconnected: 'alert'
  };

  async function refreshDeviceStatus() {
    try {
      const connectivity = await networkApi.getConnectivity();
      deviceStatus =
        connectivity.is_connected && connectivity.can_reach_host ? 'online' : 'no_internet';
    } catch {
      deviceStatus = 'disconnected';
    } finally {
      lastUpdatedAt = new Date();
    }
  }

  onMount(() => {
    void loadBranding();
    void refreshDeviceStatus();
    const timer = window.setInterval(() => {
      void refreshDeviceStatus();
    }, 5000);
    return () => window.clearInterval(timer);
  });
</script>

<svelte:head>
  <title>{$branding.product_name}</title>
  <link rel="icon" href={$branding.favicon_path} />
</svelte:head>

<div
  class="mx-auto min-h-screen w-full max-w-4xl px-4 py-8 sm:px-6 lg:px-10"
  style={$branding.background_path ? `background-image: url(${$branding.background_path}); background-size: cover; background-position: center;` : undefined}
>
  <div class="mb-8 flex flex-col gap-3 rounded-xl border border-border/80 bg-white/80 p-4 shadow-sm backdrop-blur sm:flex-row sm:items-center sm:justify-between">
    {#if $branding.logo_path}
      <a href="/" aria-label="Back to home" class="shrink-0">
        <img src={$branding.logo_path} alt="{$branding.product_name} logo" class="h-auto max-h-14 w-auto object-contain" />
      </a>
    {/if}
    <div class={`rounded-lg border px-3 py-2 text-sm sm:max-w-sm ${statusClass[deviceStatus]}`} title={statusTooltip[deviceStatus]}>
      <p class="flex items-center gap-2 font-medium" title={statusTooltip[deviceStatus]}>
        <Icon name={statusIcon[deviceStatus]} className={deviceStatus === 'checking' ? 'h-4 w-4 animate-spin' : 'h-4 w-4'} />
        {statusText[deviceStatus]}
      </p>
      <p class="mt-0.5 text-[11px] text-muted-foreground">
        Last updated:
        {#if lastUpdatedAt}
          {lastUpdatedAt.toLocaleTimeString()}
        {:else}
          --
        {/if}
      </p>
    </div>
  </div>
  <slot />
</div>
