<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { page } from '$app/state';
  import { setDevice } from '#lib/device.js';
  import '../app.css';
  import { branding } from '#lib/branding.js';
  import { networkApi } from '#lib/network/api.js';
  import Icon from '#lib/components/ui/icon.svelte';

  const device = $state<{ hostname: string | null }>({ hostname: null });
  setDevice(device);

  const navigation = [
    { href: '/', label: 'Network' },
    { href: '/connectivity', label: 'Connectivity' },
    { href: '/settings', label: 'Settings' },
    { href: '/help', label: 'Help' }
  ];

  type DeviceStatus = 'checking' | 'online' | 'no_internet' | 'disconnected';

  let { children }: { children: Snippet } = $props();
  let deviceStatus = $state<DeviceStatus>('checking');
  let lastUpdatedAt = $state<Date | null>(null);

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
    void networkApi.getDeviceInfo().then(info => {
      if (typeof info.hostname === 'string' && info.hostname) device.hostname = info.hostname;
    }).catch(() => {});
    void refreshDeviceStatus();
    const timer = window.setInterval(() => {
      void refreshDeviceStatus();
    }, 5000);
    return () => window.clearInterval(timer);
  });
</script>

<svelte:head>
  <title>{device.hostname ?? 'Network setup'}</title>
  <link rel="icon" href={$branding.favicon_path} />
</svelte:head>

<div
  class="mx-auto min-h-screen w-full max-w-4xl px-4 py-8 sm:px-6 lg:px-10"
>
  <header class="mb-8 overflow-hidden rounded-xl bg-slate-900 text-white shadow-sm">
    <div class="flex flex-wrap items-center justify-between gap-4 p-4 sm:px-5">
    {#if $branding.logo_path}
      <a href="/" aria-label="Back to home" class="shrink-0">
        <img src={$branding.logo_path} alt="Reshape" class="h-auto max-h-12 w-auto object-contain" />
      </a>
    {/if}
    {#if device.hostname}
      <p class="min-w-0 flex-1 break-words text-sm font-semibold"><bdi>{device.hostname}</bdi></p>
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
    <nav aria-label="Main navigation" class="grid grid-cols-4 gap-1 border-t border-white/15 p-2">
      {#each navigation as item (item.href)}
        {@const active = item.href === '/' ? page.url.pathname === '/' || page.url.pathname.startsWith('/setup/') || page.url.pathname.startsWith('/networks/') : page.url.pathname.startsWith(item.href)}
        <a href={item.href} aria-current={active ? 'page' : undefined}
          class={['flex min-h-11 items-center justify-center rounded-md px-1 text-sm font-semibold transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white', active ? 'bg-white/15 text-white' : 'text-slate-300 hover:bg-white/10 hover:text-white']}>
          {item.label}
        </a>
      {/each}
    </nav>
  </header>
  {@render children()}
</div>
