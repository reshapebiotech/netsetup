<script lang="ts">
  import Page from '$lib/components/page/Page.svelte';
  import Button from '$lib/components/button/Button.svelte';
  import Highlight, { LineNumbers } from 'svelte-highlight';

  import { json } from 'svelte-highlight/languages';
  import type { PageData } from './$types';
  import { goto } from '$app/navigation';
  export let data: PageData;

  // A naive way to make linebreaks. Works for now but not optimal with wifi names including ","
  let code = JSON.stringify(data?.settings?.[0])
    ?.split(',')
    .join(',\n')
    .split('{')
    .join('{\n')
    .split('}')
    .join('}\n');
</script>

<Page let:C>
  <C.Section>
    <C.Title>Network configuration</C.Title>
  </C.Section>
  <div class="flex flex-col">
    <h3 class="pb-4">Hardware addresses</h3>
    {#if !data.mac_addresses || data.mac_addresses.length === 0}
      <p>No hardware devices found</p>
    {:else}
      <ul>
        {#each data.mac_addresses as address}
          <li>
            <code class="rounded-md bg-white-500 px-2 py-1"
              >{address.interface_name}: {address.mac_address}</code
            >
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  <div class="flex flex-col gap-2">
    <h3 class="pb-4">Settings</h3>
    <div class="flex flex-col gap-4">
      {#if !data.settings || data.settings.length === 0}
        <p>No settings applied</p>
      {:else}
        <Highlight language={json} class="rounded-lg" {code} let:highlighted>
          <LineNumbers {highlighted} wrapLines />
        </Highlight>
      {/if}
      <div class="flex gap-4 justify-end pt-6">
        <Button variant="btn-outline btn-primary" on:click={() => goto('/')}>Previous</Button>
      </div>
    </div>
  </div>
</Page>
