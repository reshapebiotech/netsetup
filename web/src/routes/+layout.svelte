<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { page } from '$app/state';
  import { setDevice } from '#lib/device.js';
  import '../app.css';
  import { branding } from '#lib/branding.js';
  import { networkApi } from '#lib/network/api.js';
  import { draftUrl } from '#lib/network/drafts.js';
  import { goto } from '$app/navigation';
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
  const editingNetwork = $derived(/^\/setup\/new\/(wifi|ethernet)\/?$/.test(page.url.pathname));

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
    checking: 'text-muted-foreground',
    online: 'text-success',
    no_internet: 'text-warning',
    disconnected: 'text-destructive'
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
    let cancelled = false;
    if (page.url.pathname === '/') {
      void networkApi.listDrafts().then(drafts => {
        if (cancelled || page.url.pathname !== '/') return;
        const draft = drafts.find(draft => ['Testing', 'Connected', 'Saving'].includes(draft.state.status));
        if (draft) void goto(draftUrl(draft), { replaceState: true });
      }).catch(() => {});
    }
    void networkApi.getDeviceInfo().then(info => {
      if (typeof info.hostname === 'string' && info.hostname) device.hostname = info.hostname;
    }).catch(() => {});
    void refreshDeviceStatus();
    const timer = window.setInterval(() => {
      if (!editingNetwork) void refreshDeviceStatus();
    }, 5000);
    return () => { cancelled = true; window.clearInterval(timer); };
  });
</script>

<svelte:head>
  <title>netsetup{device.hostname ? ` • ${device.hostname}` : ''}</title>
  <link rel="icon" href={$branding.favicon_path} />
</svelte:head>

<a href="#page-content" class="skip-link">Skip to content</a>
<div class="app-shell">
  <header class="app-header">
    <div class="header-top">
      <div class="device-brand">
        <a href="/" aria-label="Back to home" class="brand-link">
          <img src={$branding.logo_path} alt="Reshape" class="brand-logo" />
        </a>
        <div class="device-identity">
          <span class="app-name">netsetup</span>
          <span class="identity-separator" aria-hidden="true">•</span>
          <span class="device-name"><bdi>{device.hostname ?? 'Connecting…'}</bdi></span>
        </div>
      </div>
      <div class="device-status" title={statusTooltip[deviceStatus]}>
        <p class={`status-label ${statusClass[deviceStatus]}`}>
          <Icon name={statusIcon[deviceStatus]} className={deviceStatus === 'checking' ? 'h-4 w-4 animate-spin' : 'h-4 w-4'} />
          {statusText[deviceStatus]}
        </p>
        {#if lastUpdatedAt}
          <p class="status-time">Updated {lastUpdatedAt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</p>
        {/if}
      </div>
    </div>
    <nav aria-label="Main navigation" class="main-nav">
      {#each navigation as item (item.href)}
        {@const active = item.href === '/' ? page.url.pathname === '/' || page.url.pathname.startsWith('/setup/') || page.url.pathname.startsWith('/networks/') : page.url.pathname.startsWith(item.href)}
        <a href={item.href} aria-current={active ? 'page' : undefined}>{item.label}</a>
      {/each}
    </nav>
  </header>
  <div id="page-content" class="page-content" tabindex="-1">
    {@render children()}
  </div>
</div>

<style>
  .app-shell { width: min(100% - 3rem, 56rem); margin-inline: auto; }
  .app-header { padding-top: 2rem; border-bottom: 1px solid hsl(var(--border)); }
  .header-top { display: flex; align-items: center; justify-content: space-between; gap: 1.5rem; padding: 0.5rem 0 1.75rem; }
  .device-brand { display: flex; align-items: center; min-width: 0; gap: 1.5rem; }
  .brand-link { flex-shrink: 0; padding-block: 0.5rem; }
  .brand-logo { display: block; width: 8rem; height: auto; }
  .device-identity { display: flex; align-items: baseline; flex-wrap: wrap; gap: 0.5rem; min-width: 0; padding-left: 1.5rem; border-left: 1px solid hsl(var(--border)); }
  .app-name { font-size: 0.875rem; font-weight: 600; }
  .identity-separator { color: hsl(var(--muted-foreground)); }
  .device-name { font-size: 0.875rem; font-weight: 500; overflow-wrap: anywhere; }
  .device-status { flex-shrink: 0; text-align: right; }
  .status-label { display: flex; justify-content: flex-end; align-items: center; gap: 0.5rem; font-size: 0.8125rem; font-weight: 500; }
  .status-time { margin-top: 0.25rem; font-size: 0.6875rem; color: hsl(var(--muted-foreground)); }
  .main-nav { display: flex; gap: 1.75rem; }
  .main-nav a { display: flex; align-items: center; min-height: 3rem; padding: 0.5rem 0; border-bottom: 2px solid transparent; margin-bottom: -1px; color: hsl(var(--muted-foreground)); font-size: 0.875rem; font-weight: 500; transition: color 150ms; }
  .main-nav a:hover { color: hsl(var(--foreground)); }
  .main-nav a[aria-current="page"] { border-color: hsl(var(--primary)); color: hsl(var(--foreground)); }
  .page-content { padding-block: 2.75rem calc(4rem + var(--netsetup-actions-height, 0px)); }
  .page-content:focus { outline: none; }
  .skip-link { position: fixed; top: 0.5rem; left: 1rem; z-index: 20; padding: 0.75rem 1rem; background: hsl(var(--primary)); color: hsl(var(--primary-foreground)); border-radius: 0.5rem; transform: translateY(-150%); }
  .skip-link:focus { transform: translateY(0); }
  @media (max-width: 540px) {
    .app-shell { width: calc(100% - 2rem); }
    .app-header { padding-top: 1.25rem; }
    .header-top { align-items: flex-start; gap: 1rem; padding-bottom: 1.25rem; }
    .device-brand { flex-direction: column; align-items: flex-start; gap: 0.75rem; }
    .brand-logo { width: 7rem; }
    .device-identity { padding-left: 0; border: 0; }
    .device-status { padding-top: 0.625rem; }
    .main-nav { justify-content: space-between; gap: 0.75rem; }
    .main-nav a { font-size: 0.8125rem; }
    .page-content { padding-block: 2rem calc(3rem + var(--netsetup-actions-height, 0px)); }
  }
</style>
