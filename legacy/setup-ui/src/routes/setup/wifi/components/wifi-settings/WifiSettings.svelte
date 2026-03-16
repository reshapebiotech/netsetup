<script lang="ts">
  import Button from '$lib/components/button/Button.svelte';
  import Card from '$lib/components/card/Card.svelte';
  import InternetSettings from '$lib/components/internet-settings/InternetSettings.svelte';
  import { networkConfig } from '$lib/stores';

  export let configChoice = 'standard';
  export let goBack: () => void;
  export let goForward: () => void;

  $: if (configChoice !== 'manual') {
    $networkConfig.ipv4_address = '';
    $networkConfig.ipv4_gateway = '';
    $networkConfig.dns = [];
  }

  const handleSubmit = async (e: Event) => {
    e.preventDefault();

    goForward();
  };
</script>

<div class="flex flex-col gap-4">
  <label for="standard" class="cursor-pointer">
    <Card let:C>
      <div class="flex justify-between items-center">
        <div class="flex flex-col gap-1">
          <h5>Standard</h5>
          <p class="text-sm text-dark-70">No additional requirements</p>
        </div>
        <C.Radio bind:group={configChoice} value="standard" id="standard" />
      </div>
    </Card>
  </label>
  <label for="custom" class="cursor-pointer">
    <Card let:C>
      <div class="flex justify-between items-center">
        <div class="flex flex-col gap-1 pr-2">
          <h5>Custom</h5>
          <p class="text-sm text-dark-70">
            Networks with a firewall, SHCP, static IP address or other specific requirements
          </p>
        </div>
        <C.Radio bind:group={configChoice} value="custom" id="custom" />
      </div>
    </Card>
  </label>
</div>
{#if configChoice === 'custom'}
  <InternetSettings {handleSubmit} {goBack} />
{:else}
  <div class="flex flex-col-reverse sm:flex-row sm:justify-end gap-2">
    <Button variant="btn-outline btn-primary" on:click={goBack}>Previous</Button>
    <Button variant="btn-primary" on:click={goForward}>Continue</Button>
  </div>
{/if}
