<script lang="ts">
  import { page } from '$app/state';
  import { goto, preloadCode } from '$app/navigation';
  import PageActions from '#lib/components/ui/page-actions.svelte';
  import Card from '#lib/components/ui/card.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Input from '#lib/components/ui/input.svelte';
  import Select from '#lib/components/ui/select.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import Loading from '#lib/components/ui/loading.svelte';
  import Icon from '#lib/components/ui/icon.svelte';
  import { networkApi } from '#lib/network/api.js';
  import { captiveFocus } from '#lib/network/captive-focus.js';
  import type { NetworkSettings, SavedNetworkDetails, Transport, WifiNetwork } from '#lib/network/types.js';
  import { buildSettingsFromForm } from '#lib/network/settings.js';
  import { validateSettings } from '#lib/network/validators.js';
  import { onMount } from 'svelte';

  const transport = $derived<Transport>(page.params.transport === 'ethernet' ? 'ethernet' : 'wifi');

  const draftId = $derived(page.url.searchParams.get('draft'));
  const editId = $derived(draftId ?? page.url.searchParams.get('edit'));
  let original = $state<SavedNetworkDetails | null>(null);
  let profileLoading = $state(true);
  let profileError = $state('');
  let keepPassword = $state(true);


  let wifiNetworks = $state<WifiNetwork[]>([]);
  let wifiLoading = $state(false);
  let wifiError = $state('');
  let setupWifiName: string | undefined;

  let manualSsid = $state('');
  let selectedNetwork = $state<WifiNetwork | null>(null);
  let hiddenSsid = $state(false);
  const ssid = $derived(hiddenSsid ? manualSsid : selectedNetwork?.ssid ?? '');
  const networkLabel = (network: WifiNetwork) => `${network.ssid} · ${network.security} · ${network.signal_strength}%`;
  const wifiOptions = $derived([
    { label: 'Choose a network…', value: '' },
    ...wifiNetworks.map(network => ({ label: networkLabel(network), value: network.id })),
    ...(selectedNetwork && !wifiNetworks.some(network => network.id === selectedNetwork?.id)
      ? [{ label: `${networkLabel(selectedNetwork)} (not seen in latest scan)`, value: selectedNetwork.id }] : [])
  ]);
  const openNetwork = $derived(hiddenSsid
    ? original?.security === 'open' && ssid === original.settings.ssid
    : selectedNetwork !== null && ['', '--', 'none', 'open'].includes(selectedNetwork.security.trim().toLowerCase()));

  function selectNetwork(id: string) {
    const network = wifiNetworks.find(network => network.id === id)
      ?? (selectedNetwork?.id === id ? selectedNetwork : null);
    selectedNetwork = network;
    if (network) enterprise = /802[.-]1x|eap/i.test(network.security);
  }

  function setManualEntry(enabled: boolean) {
    if (enabled && !hiddenSsid) manualSsid = ssid;
    hiddenSsid = enabled;
  }
  let password = $state('');
  let showPassword = $state(false);
  let enterprise = $state(false);
  let eapLogin = $state('');
  let eapPassword = $state('');
  let showEapPassword = $state(false);

  const canKeepPassword = $derived(original !== null && original.security !== 'open'
    && (transport !== 'wifi' || ssid === original.settings.ssid)
    && enterprise === (original.security === 'enterprise')
    && (!enterprise || eapLogin === original.settings.eap_login));
  const retainingPassword = $derived(canKeepPassword && keepPassword);

  let ipMethod = $state<'auto' | 'manual'>('auto');
  let ipv4Address = $state('');
  let ipv4Prefix = $state('24');
  let ipv4Gateway = $state('');
  let dnsRaw = $state('');

  let submitError = $state('');
  let submitting = $state(false);

  async function loadWifi() {
    if (transport !== 'wifi') return;
    wifiLoading = true;
    wifiError = '';
    try {
      wifiNetworks = await networkApi.scanWifi();
    } catch (e) {
      wifiError = e instanceof Error ? e.message : 'Failed to scan Wi-Fi';
    } finally {
      wifiLoading = false;
    }
  }

  onMount(() => {
    void networkApi.getApControllerStatus().then(status => {
      if (typeof status.ssid === 'string' && status.ssid) setupWifiName = status.ssid;
    }).catch(() => {});
    void initialize();
  });

  async function initialize() {
    if (editId) {
      try {
        original = draftId ? await networkApi.getDraft(draftId) : await networkApi.getSaved(editId);
        if (original.network.transport !== transport || original.edit_unavailable) {
          throw new Error(original.edit_unavailable ?? 'This network uses a different connection type.');
        }
        const saved = original.settings;
        manualSsid = saved.ssid ?? '';
        enterprise = original.security === 'enterprise';
        eapLogin = saved.eap_login ?? '';
        ipMethod = saved.ipv4?.method ?? 'auto';
        ipv4Address = saved.ipv4?.address_data?.[0]?.address ?? '';
        ipv4Prefix = String(saved.ipv4?.address_data?.[0]?.prefix ?? 24);
        ipv4Gateway = saved.ipv4?.gateway ?? '';
        dnsRaw = saved.ipv4?.dns?.join(', ') ?? '';
        hiddenSsid = transport === 'wifi';
      } catch (e) {
        profileError = e instanceof Error ? e.message : 'Could not load this network.';
      } finally { profileLoading = false; }
    }
    profileLoading = false;
    await loadWifi();
  }

  function buildSettings(): NetworkSettings {
    const settings = buildSettingsFromForm({
      transport,
      ssid,
      password,
      enterprise,
      openNetwork,
      eapLogin,
      eapPassword,
      ipMethod,
      ipv4Address,
      ipv4Prefix,
      ipv4Gateway,
      dnsRaw
    });
    if (retainingPassword && editId) {
      settings.id = editId;
      delete settings.password;
      delete settings.eap_password;
    }
    return settings;
  }

  async function startTest() {
    if (submitting || profileLoading || profileError || wifiLoading) return;
    submitError = '';
    const settings = buildSettings();
    const errors = validateSettings(transport, settings, enterprise, !openNetwork, retainingPassword);
    if (errors.length) {
      submitError = errors[0];
      return;
    }

    submitting = true;
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    try {
      await Promise.all([
        preloadCode('/setup/new/[transport]/testing/[testId]'),
        preloadCode('/setup/new/[transport]/result/[testId]'),
        preloadCode('/setup/new/[transport]/save-handoff/[testId]')
      ]);
      const testId = await networkApi.startTest(transport, settings);
      await goto(`/setup/new/${transport}/testing/${testId}`, { state: { setupWifiName } });
    } catch (e) {
      submitError = e instanceof Error ? e.message : 'Failed to start test';
    } finally {
      submitting = false;
    }
  }
</script>

<div class="space-y-6" {@attach captiveFocus}>
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">{editId ? 'Edit' : 'New'} {transport} profile</p>
    <h1 class="flex items-center gap-2 text-3xl font-semibold">
      <Icon name={transport === 'wifi' ? 'wifi' : 'ethernet'} className="h-6 w-6 text-primary" />
      Configure and test
    </h1>
  </header>

  {#if profileLoading}
    <Loading label="Loading saved settings…" />
  {:else if profileError}
    <Alert tone="error">{profileError}</Alert>
    <a href="/" class="inline-flex min-h-11 items-center font-medium text-primary hover:underline">Back to networks →</a>
  {/if}

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

      <label class="flex min-h-11 items-center gap-2 text-sm">
        <input type="checkbox" bind:checked={() => hiddenSsid, setManualEntry} class="rounded border-input" />
        Enter network name manually
      </label>

      {#if hiddenSsid}
        <label class="block space-y-2 text-sm font-medium">
          SSID
          <Input bind:value={manualSsid} placeholder="Enter network name" />
        </label>
      {:else}
        <label class="block space-y-2 text-sm font-medium">
          SSID
          <Select
            bind:value={() => selectedNetwork?.id ?? '', selectNetwork}
            disabled={wifiLoading || (wifiNetworks.length === 0 && !selectedNetwork)}
            options={wifiOptions}
          />
        </label>
      {/if}

      <label class="flex min-h-11 items-center gap-2 text-sm">
        <input type="checkbox" bind:checked={enterprise} disabled={!hiddenSsid && selectedNetwork !== null} class="rounded border-input" />
        Enterprise Wi-Fi (PEAP/MSCHAPv2)
      </label>

      {#if !enterprise && canKeepPassword}
        <label class="flex items-center gap-3 rounded-lg border border-border bg-background/30 px-3 py-3 text-sm">
          <input type="checkbox" bind:checked={keepPassword} class="rounded border-input" />
          {draftId ? 'Keep draft password' : 'Keep saved password'}
        </label>
      {/if}

      {#if !enterprise && !openNetwork && !retainingPassword}
        <div class="space-y-2">
          <label for="wifi-password" class="block text-sm font-medium">Password</label>
          <div class="flex items-center gap-2">
            <Input id="wifi-password" bind:value={password} type={showPassword ? 'text' : 'password'} autocomplete="off" autocapitalize="none" spellcheck={false} disabled={submitting} />
            <Button
              variant="secondary"
              aria-label={showPassword ? 'Hide password' : 'Show password'}
              aria-controls="wifi-password"
              aria-pressed={showPassword}
              onclick={() => (showPassword = !showPassword)}
              disabled={submitting}
            >
              <span class="mr-2 inline-flex items-center">
                <Icon name={showPassword ? 'eye-off' : 'eye'} />
              </span>
              {showPassword ? 'Hide' : 'Show'}
            </Button>
          </div>
        </div>
      {/if}
    </Card>
  {:else}
    <Card className="space-y-3">
      <h2 class="flex items-center gap-2 text-lg font-semibold">
        <Icon name="ethernet" className="h-5 w-5 text-primary" />
        Ethernet authentication
      </h2>

      <label class="flex min-h-11 items-center gap-2 text-sm">
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
      <label class="block space-y-2 text-sm font-medium">
        Identity
        <Input bind:value={eapLogin} placeholder="user@example.com" disabled={submitting} />
      </label>
      {#if canKeepPassword}
        <label class="flex items-center gap-3 rounded-lg border border-border bg-background/30 px-3 py-3 text-sm">
          <input type="checkbox" bind:checked={keepPassword} class="rounded border-input" />
          {draftId ? 'Keep draft password' : 'Keep saved password'}
        </label>
      {/if}

      {#if !retainingPassword}
      <div class="space-y-2">
        <label for="enterprise-password" class="block text-sm font-medium">Password</label>
        <div class="flex items-center gap-2">
          <Input id="enterprise-password" bind:value={eapPassword} type={showEapPassword ? 'text' : 'password'} autocomplete="off" autocapitalize="none" spellcheck={false} disabled={submitting} />
          <Button
            variant="secondary"
            aria-label={showEapPassword ? 'Hide password' : 'Show password'}
            aria-controls="enterprise-password"
            aria-pressed={showEapPassword}
            onclick={() => (showEapPassword = !showEapPassword)}
            disabled={submitting}
          >
            <span class="mr-2 inline-flex items-center">
              <Icon name={showEapPassword ? 'eye-off' : 'eye'} />
            </span>
            {showEapPassword ? 'Hide' : 'Show'}
          </Button>
        </div>
      </div>
      {/if}
    </Card>
  {/if}

  <Card className="space-y-3">
    <h2 class="flex items-center gap-2 text-lg font-semibold">
      <Icon name="network" className="h-5 w-5 text-primary" />
      IPv4 and DNS
    </h2>

    <label class="block space-y-2 text-sm font-medium">
      IPv4 method
      <Select bind:value={ipMethod} options={[{ label: 'Automatic (DHCP)', value: 'auto' }, { label: 'Manual', value: 'manual' }]} />
    </label>

    {#if ipMethod === 'manual'}
      <div class="grid gap-3 sm:grid-cols-2">
        <label class="block space-y-2 text-sm font-medium">
          Address
          <Input bind:value={ipv4Address} placeholder="192.168.1.20" disabled={submitting} />
        </label>
        <label class="block space-y-2 text-sm font-medium">
          Prefix
          <Input bind:value={ipv4Prefix} type="number" disabled={submitting} />
        </label>
      </div>
      <label class="block space-y-2 text-sm font-medium">
        Gateway
        <Input bind:value={ipv4Gateway} placeholder="192.168.1.1" disabled={submitting} />
      </label>
    {/if}

    <div class="space-y-2">
      <label for="network-dns" class="block text-sm font-medium">Custom DNS servers (comma separated)</label>
      <Input id="network-dns" aria-describedby="network-dns-help" bind:value={dnsRaw} placeholder="1.1.1.1,8.8.8.8" disabled={submitting} />
      <p id="network-dns-help" class="text-xs text-muted-foreground">
        Leave empty to use DNS servers provided by DHCP. IPv4 addresses only.
      </p>
    </div>
  </Card>

  {#if submitting}
    <Alert tone="info">Submitting connection test… keep this tab open.</Alert>
  {/if}

  <PageActions>
    {#snippet secondary()}
      <Button variant="secondary" disabled={submitting} onclick={() => goto('/')}>Cancel</Button>
    {/snippet}
    {#snippet primary()}
      <Button onclick={startTest} loading={submitting} disabled={profileLoading || !!profileError || wifiLoading}>
        {#if !submitting}<Icon name="check" />{/if}
        {submitting ? 'Starting test…' : 'Test connection'}
      </Button>
    {/snippet}
  </PageActions>
</div>
