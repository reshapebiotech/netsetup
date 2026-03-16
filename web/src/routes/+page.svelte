<script lang="ts">
  import { NetworkState } from '$lib/network.svelte';
  import { onMount } from 'svelte';
  
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Badge } from '$lib/components/ui/badge/index.js';
  import * as Card from '$lib/components/ui/card/index.js';
  import * as Accordion from '$lib/components/ui/accordion/index.js';

  import { 
    Loader2, 
    Wifi, 
    WifiOff, 
    CheckCircle2, 
    XCircle, 
    Lock, 
    Unlock, 
    RefreshCw, 
    Trash2,
    AlertCircle
  } from 'lucide-svelte';

  const networkState = new NetworkState();

  onMount(() => {
    networkState.loadNetworks();
  });

  let selectedSsid = $state<string | undefined>();
  let password = $state('');

  function handleConnect() {
    if (selectedSsid) {
      networkState.connect(selectedSsid, password);
    }
  }

  // Reset password when selection changes
  $effect(() => {
    if (selectedSsid !== undefined) {
      password = '';
    }
  });
</script>

<div class="max-w-2xl mx-auto p-6 min-h-screen">
  <div class="mb-8">
    <h1 class="text-3xl font-bold tracking-tight mb-2">Network Settings</h1>
    <p class="text-muted-foreground">Connect to a Wi-Fi network.</p>
  </div>

  {#if networkState.error}
    <div class="bg-destructive/10 text-destructive p-4 rounded-lg mb-6 border border-destructive/20 flex items-center gap-3">
      <AlertCircle class="h-5 w-5" />
      <p>{networkState.error}</p>
    </div>
  {/if}

  <!-- Connection Status Chasm Handling -->
  {#if networkState.machine.type !== 'idle'}
    <Card.Root class="mb-8">
      <Card.Header>
        <Card.Title>Connection Status</Card.Title>
      </Card.Header>
      <Card.Content>
        {#if networkState.machine.type === 'connecting'}
          <div class="flex items-center text-blue-600">
            <Loader2 class="animate-spin -ml-1 mr-3 h-5 w-5" />
            Connecting to {networkState.machine.ssid}...
          </div>
        {:else if networkState.machine.type === 'testing'}
          <div class="flex items-center text-yellow-600">
            <Loader2 class="animate-spin -ml-1 mr-3 h-5 w-5" />
            Testing connection to {networkState.machine.ssid}...
          </div>
        {:else if networkState.machine.type === 'disconnected'}
          <div class="flex items-center text-orange-600">
            <WifiOff class="animate-pulse -ml-1 mr-3 h-5 w-5" />
            <span>Device disconnected. Waiting for {networkState.machine.ssid} to be available...</span>
          </div>
          <p class="text-sm text-muted-foreground mt-2 ml-8">This is expected during network switch.</p>
        {:else if networkState.machine.type === 'success'}
          <div class="flex items-center justify-between">
            <div class="flex items-center text-green-600">
              <CheckCircle2 class="w-6 h-6 mr-2" />
              Successfully connected to {networkState.machine.ssid}
            </div>
            <Button variant="secondary" size="sm" onclick={() => networkState.reset()}>Dismiss</Button>
          </div>
        {:else if networkState.machine.type === 'failed'}
          <div class="flex flex-col text-destructive">
            <div class="flex items-center">
              <XCircle class="w-6 h-6 mr-2" />
              Failed to connect to {networkState.machine.ssid}
            </div>
            <p class="text-sm mt-2 ml-8 text-muted-foreground">{networkState.machine.error}</p>
            <Button variant="destructive" size="sm" class="mt-4 self-start ml-8" onclick={() => networkState.reset()}>
              Dismiss
            </Button>
          </div>
        {/if}
      </Card.Content>
    </Card.Root>
  {/if}

  <div class="space-y-8">
    <!-- Saved Networks -->
    {#if networkState.saved.length > 0}
      <div>
        <h2 class="text-xl font-semibold mb-4">Saved Networks</h2>
        <Card.Root>
          <div class="divide-y divide-border">
            {#each networkState.saved as network (network.id)}
              <div class="p-4 flex items-center justify-between hover:bg-muted/50 transition-colors">
                <div class="flex items-center gap-3">
                  <Wifi class="h-4 w-4 text-muted-foreground" />
                  <span class="font-medium">{network.ssid}</span>
                </div>
                <Button 
                  variant="ghost" 
                  size="sm"
                  class="text-destructive hover:text-destructive hover:bg-destructive/10"
                  onclick={() => networkState.forget(network.id)}>
                  <Trash2 class="h-4 w-4 mr-2" />
                  Forget
                </Button>
              </div>
            {/each}
          </div>
        </Card.Root>
      </div>
    {/if}

    <!-- Available Networks -->
    <div>
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-xl font-semibold">Available Networks</h2>
        <Button 
          variant="outline"
          size="sm"
          onclick={() => networkState.loadNetworks()}
          disabled={networkState.loading}>
          {#if networkState.loading}
            <Loader2 class="h-4 w-4 mr-2 animate-spin" />
            Scanning...
          {:else}
            <RefreshCw class="h-4 w-4 mr-2" />
            Refresh
          {/if}
        </Button>
      </div>

      <Card.Root>
        {#if networkState.loading && networkState.available.length === 0}
          <div class="p-8 text-center text-muted-foreground flex flex-col items-center gap-3">
            <Loader2 class="h-8 w-8 animate-spin text-primary" />
            <p>Scanning for nearby networks...</p>
          </div>
        {:else if networkState.available.length === 0}
          <div class="p-8 text-center text-muted-foreground">No networks found</div>
        {:else}
          <Accordion.Root type="single" class="w-full" bind:value={selectedSsid}>
            {#each networkState.available as network (network.ssid)}
              {@const requiresPassword = Boolean(network.security && network.security !== 'NONE' && network.security !== 'open')}
              
              <Accordion.Item value={network.ssid} class="px-4 border-b last:border-0">
                <Accordion.Trigger class="hover:no-underline py-4">
                  <div class="flex items-center justify-between w-full pr-4">
                    <div class="flex items-center space-x-3">
                      <div class="text-muted-foreground">
                        {#if requiresPassword}
                          <Lock class="w-4 h-4" />
                        {:else}
                          <Unlock class="w-4 h-4" />
                        {/if}
                      </div>
                      <span class="font-medium">{network.ssid}</span>
                    </div>
                    <div class="flex items-center space-x-3">
                      {#if network.security !== 'NONE' && network.security !== 'open'}
                        <Badge variant="secondary" class="text-[10px] uppercase">{network.security}</Badge>
                      {/if}
                      <div class="flex items-end space-x-0.5 h-4 opacity-75">
                        <div class="w-1 rounded-t-sm h-1.5 {network.signal_strength > 20 ? 'bg-foreground' : 'bg-muted'}"></div>
                        <div class="w-1 rounded-t-sm h-2.5 {network.signal_strength > 40 ? 'bg-foreground' : 'bg-muted'}"></div>
                        <div class="w-1 rounded-t-sm h-3.5 {network.signal_strength > 60 ? 'bg-foreground' : 'bg-muted'}"></div>
                        <div class="w-1 rounded-t-sm h-4 {network.signal_strength > 80 ? 'bg-foreground' : 'bg-muted'}"></div>
                      </div>
                    </div>
                  </div>
                </Accordion.Trigger>
                <Accordion.Content>
                  <div class="pt-2 pb-4 pl-7 pr-4">
                    <form 
                      onsubmit={(e) => { e.preventDefault(); handleConnect(); }} 
                      class="flex items-center space-x-3 max-w-md"
                    >
                      {#if requiresPassword}
                        <Input 
                          type="password" 
                          bind:value={password}
                          placeholder="Network Password"
                          autocomplete="current-password"
                          class="flex-1"
                        />
                      {/if}
                      <Button 
                        type="submit"
                        disabled={requiresPassword && password.length < 1}>
                        Connect
                      </Button>
                    </form>
                  </div>
                </Accordion.Content>
              </Accordion.Item>
            {/each}
          </Accordion.Root>
        {/if}
      </Card.Root>
    </div>
  </div>
</div>
