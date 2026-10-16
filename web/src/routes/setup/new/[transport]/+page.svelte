<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import Alert from '$lib/components/ui/alert.svelte';
  import Loading from '$lib/components/ui/loading.svelte';
  import Icon from '$lib/components/ui/icon.svelte';
  import { networkApi } from '$lib/network/api';
  import type { NetworkSettings, Transport, WifiNetwork } from '$lib/network/types';
  import { buildSettingsFromForm } from '$lib/network/settings';
  import { validateSettings } from '$lib/network/validators';
  import { onMount } from 'svelte';

  let transport: Transport = 'wifi';
  $: transport = ($page.params.transport === 'ethernet' ? 'ethernet' : 'wifi') as Transport;

  let wifiNetworks: WifiNetwork[] = [];
  let wifiLoading = false;
  let wifiError = '';

  let ssid = '';
  let hiddenSsid = false;
  let password = '';
  let showPassword = false;
  let enterprise = false;
  let eapLogin = '';
  let eapPassword = '';
  let showEapPassword = false;

  let ipMethod: 'auto' | 'manual' = 'auto';
  let ipv4Address = '';
  let ipv4Prefix = '24';
  let ipv4Gateway = '';
  let dnsRaw = '';

  let submitError = '';
  let submitting = false;

  async function loadWifi() {
    if (transport !== 'wifi') return;
    wifiLoading = true;
    wifiError = '';
    try {
      wifiNetworks = await networkApi.scanWifi();
      if (!ssid && wifiNetworks.length) ssid = wifiNetworks[0].ssid;
    } catch (e) {
      wifiError = e instanceof Error ? e.message : 'Failed to scan Wi-Fi';
    } finally {
      wifiLoading = false;
    }
  }

  onMount(loadWifi);

  function buildSettings(): NetworkSettings {
    return buildSettingsFromForm({
      transport,
      ssid,
      password,
      enterprise,
      eapLogin,
      eapPassword,
      ipMethod,
      ipv4Address,
      ipv4Prefix,
      ipv4Gateway,
      dnsRaw
    });
  }

  async function startTest() {
    submitError = '';
    const settings = buildSettings();
    const errors = validateSettings(transport, settings, enterprise);
    if (errors.length) {
      submitError = errors[0];
      return;
    }

    submitting = true;
    try {
      const testId = await networkApi.startTest(transport, settings);
      await goto(`/setup/new/${transport}/testing/${testId}`);
    } catch (e) {
      submitError = e instanceof Error ? e.message : 'Failed to start test';
    } finally {
      submitting = false;
    }
  }
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">New {transport} profile</p>
    <h1 class="flex items-center gap-2 text-3xl font-bold">
      <Icon name={transport === 'wifi' ? 'wifi' : 'ethernet'} className="h-6 w-6 text-primary" />
      Configure and test
    </h1>
  </header>

  {#if submitError}
    <Alert tone="error">{submitError}</Alert>
  {/if}

  {#if transport === 'wifi'}
    <Card className="space-y-3">
      <div class="flex items-center justify-between gap-3">
        <h2 class="flex items-center gap-2 text-lg font-semibold">
          <Icon name="wifi" className="h-5 w-5 text-primary" />
          Wi-Fi network
        </h2>
        <Button variant="secondary" onclick={loadWifi} loading={wifiLoading}>
          {#if !wifiLoading}
            <span class="mr-2 inline-flex items-center"><Icon name="refresh" /></span>
          {/if}
          Rescan
        </Button>
      </div>

      {#if wifiError}
        <Alert tone="error">{wifiError}</Alert>
      {/if}

      {#if wifiLoading}
        <Loading label="Scanning nearby Wi-Fi networks…" />
      {/if}

      <label class="flex items-center gap-2 text-sm">
        <input type="checkbox" bind:checked={hiddenSsid} class="rounded border-input" />
        Hidden SSID
      </label>

      {#if hiddenSsid}
        <label class="space-y-2 text-sm font-medium">
          SSID
          <Input bind:value={ssid} placeholder="Enter hidden network name" />
        </label>
      {:else}
        <label class="space-y-2 text-sm font-medium">
          SSID
          <Select
            bind:value={ssid}
            disabled={wifiLoading || wifiNetworks.length === 0}
            options={wifiNetworks.map((n) => ({ label: `${n.ssid} (${n.signal_strength}%)`, value: n.ssid }))}
          />
        </label>
      {/if}

      <label class="flex items-center gap-2 text-sm">
        <input type="checkbox" bind:checked={enterprise} class="rounded border-input" />
        Enterprise Wi-Fi (PEAP/MSCHAPv2)
      </label>

      {#if !enterprise}
        <label class="space-y-2 text-sm font-medium">
          Password
          <div class="flex items-center gap-2">
            <Input bind:value={password} type={showPassword ? 'text' : 'password'} disabled={submitting} />
            <Button
              variant="secondary"
              onclick={() => (showPassword = !showPassword)}
              disabled={submitting}
            >
              <span class="mr-2 inline-flex items-center">
                <Icon name={showPassword ? 'eye-off' : 'eye'} />
              </span>
              {showPassword ? 'Hide' : 'Show'}
            </Button>
          </div>
        </label>
      {/if}
    </Card>
  {:else}
    <Card className="space-y-3">
      <h2 class="flex items-center gap-2 text-lg font-semibold">
        <Icon name="ethernet" className="h-5 w-5 text-primary" />
        Ethernet authentication
      </h2>

      <label class="flex items-center gap-2 text-sm">
        <input type="checkbox" bind:checked={enterprise} class="rounded border-input" />
        Wired 802.1X (PEAP/MSCHAPv2)
      </label>
    </Card>
  {/if}

  {#if enterprise}
    <Card className="space-y-3">
      <h2 class="flex items-center gap-2 text-lg font-semibold">
        <Icon name="network" className="h-5 w-5 text-primary" />
        802.1X credentials
      </h2>
      <label class="space-y-2 text-sm font-medium">
        Identity
        <Input bind:value={eapLogin} placeholder="user@example.com" disabled={submitting} />
      </label>
      <label class="space-y-2 text-sm font-medium">
        Password
        <div class="flex items-center gap-2">
          <Input bind:value={eapPassword} type={showEapPassword ? 'text' : 'password'} disabled={submitting} />
          <Button
            variant="secondary"
            onclick={() => (showEapPassword = !showEapPassword)}
            disabled={submitting}
          >
            <span class="mr-2 inline-flex items-center">
              <Icon name={showEapPassword ? 'eye-off' : 'eye'} />
            </span>
            {showEapPassword ? 'Hide' : 'Show'}
          </Button>
        </div>
      </label>
    </Card>
  {/if}

  <Card className="space-y-3">
    <h2 class="flex items-center gap-2 text-lg font-semibold">
      <Icon name="network" className="h-5 w-5 text-primary" />
      IPv4 and DNS
    </h2>

    <label class="space-y-2 text-sm font-medium">
      IPv4 method
      <Select bind:value={ipMethod} options={[{ label: 'Automatic (DHCP)', value: 'auto' }, { label: 'Manual', value: 'manual' }]} />
    </label>

    {#if ipMethod === 'manual'}
      <div class="grid gap-3 sm:grid-cols-2">
        <label class="space-y-2 text-sm font-medium">
          Address
          <Input bind:value={ipv4Address} placeholder="192.168.1.20" disabled={submitting} />
        </label>
        <label class="space-y-2 text-sm font-medium">
          Prefix
          <Input bind:value={ipv4Prefix} type="number" disabled={submitting} />
        </label>
      </div>
      <label class="space-y-2 text-sm font-medium">
        Gateway
        <Input bind:value={ipv4Gateway} placeholder="192.168.1.1" disabled={submitting} />
      </label>
    {/if}

    <label class="space-y-2 text-sm font-medium">
      Custom DNS servers (comma separated)
      <Input bind:value={dnsRaw} placeholder="1.1.1.1,8.8.8.8" disabled={submitting} />
      <p class="text-xs font-normal text-muted-foreground">
        Leave empty to use DNS servers provided by DHCP. IPv4 addresses only.
      </p>
    </label>
  </Card>

  {#if submitting}
    <Alert tone="info">Submitting connection test… keep this tab open.</Alert>
  {/if}

  <div class="flex items-center gap-3">
    <Button onclick={startTest} loading={submitting}>
      {#if !submitting}
        <span class="mr-2 inline-flex items-center"><Icon name="check" /></span>
      {/if}
      {submitting ? 'Starting test…' : 'Test connection'}
    </Button>
    <Button variant="secondary" onclick={() => goto('/')}>
      <span class="mr-2 inline-flex items-center"><Icon name="back" /></span>
      Cancel
    </Button>
  </div>
</div>
