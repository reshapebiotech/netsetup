<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import type { LayoutData } from './$types';
  import { goto } from '$app/navigation';
  import { House, WifiSlash } from 'phosphor-svelte';
  import Button from '$lib/components/button/Button.svelte';
  import API from '$lib/api';
  import { isConnectedToRobot, isLoading } from '$lib/stores';

  export let data: LayoutData;
  $: origin = data.url.origin;
  $: thisPage = data.url;

  const checkConnectivity = async () => {
    const controller = new AbortController();
    try {
      const timeout = setTimeout(() => controller.abort(), 10000);
      const { response } = await API().GET('/', { signal: controller.signal });
      clearTimeout(timeout);
      $isConnectedToRobot = true;
      if (!response.ok) {
        $isConnectedToRobot = false;
      }
    } catch (_: unknown) {
      $isConnectedToRobot = false;
    }
  };

  onMount(async () => {
    const interval = setInterval(checkConnectivity, 8000);
    return (() => clearInterval(interval)) as unknown as void;
  });
</script>

<div class="relative">
  <div
    class="flex flex-row justify-between px-4 py-4 border-b-1 top-0 left-0 right-0 fixed rounded-none bg-background-500 z-50"
  >
    <img alt="reshape logo" src="/reshape-logo.svg" width="24px" height="24px" />
    {#if thisPage?.pathname !== '/'}
      <Button iconButton size="sm" on:click={() => goto('/')}><House weight="bold" /></Button>
    {:else}
      <div class="h-8"></div>
    {/if}
  </div>
  <slot />
</div>
<div>
  {#if $isConnectedToRobot === false && !$isLoading}
    <div class="toast">
      <div class="alert alert-warning max-w-l">
        <div class="flex flex-row gap-2">
          <div class="flex flex-col self-center pr-1"><WifiSlash size="20" weight="bold" /></div>
          <div class="flex flex-col text-left text-wrap">
            <div class="text-sm font-medium max-[600px]:text-xs">
              It looks like you have lost connection to your device.
            </div>
            <div class="text-sm max-[600px]:text-xs">
              Your robot may have connected to your network or you lost connection. Verify the
              status from the robot logo.
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
