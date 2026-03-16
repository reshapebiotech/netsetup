<script lang="ts">
  import { goto } from '$app/navigation';
  import type { types } from '$lib/api';
  import Button from '$lib/components/button/Button.svelte';
  import Card from '$lib/components/card/Card.svelte';
  import InternetSettings from '$lib/components/internet-settings/InternetSettings.svelte';
  import Page from '$lib/components/page/Page.svelte';
  import StatusLabel from '$lib/components/status-label/StatusLabel.svelte';
  import { hasEapEthernet, hasNoEthernet, hasInternet, networkConfig, networks } from '$lib/stores';
  import { connectWired, resetNetworkConfig } from '$lib/utils';
  import { CheckCircle } from 'phosphor-svelte';

  const handleSubmit = async (e: Event) => {
    e.preventDefault();

    const settings: types['NetworkSettings-Input'] = {
      ssid: $networkConfig.ssid,
      password: $networkConfig.password,
      eap_login: $networkConfig.eap_login,
      eap_password: $networkConfig.eap_password,
      ipv4: {
        method: $networkConfig.ipv4_method,
        address_data: $networkConfig.ipv4_address
          ? [
              {
                address: $networkConfig.ipv4_address,
                prefix: Number($networkConfig.ipv4_prefix),
              },
            ]
          : undefined,
        gateway: $networkConfig.ipv4_gateway ? $networkConfig.ipv4_gateway : undefined,
        dns: $networkConfig.dns ? $networkConfig.dns : undefined,
      },
    };
    try {
      await connectWired(settings);
      goto('/setup/ethernet/one');
    } catch (e) {
      console.error(e);
    }
  };
</script>

<Page let:C>
  <C.Stepper numberOfSteps={5} currentStep={4} />

  <C.Title>Network settings</C.Title>
  <div class="flex flex-col gap-6">
    {#if !$hasInternet || $hasNoEthernet || $networks === undefined}
      <InternetSettings {handleSubmit} goBack={() => goto('/setup/three')} />
    {:else}
      <div role="alert" class="alert alert-success rounded-lg text-dark-70">
        <CheckCircle size="24" />
        <span>Everything looks good</span>
      </div>
      <Card>
        <div class="flex justify-between items-center">
          <h5>Network</h5>
          <StatusLabel status="success" />
        </div>
      </Card>
    {/if}
  </div>
  <div class="flex gap-2 justify-end">
    {#if $hasEapEthernet || $hasNoEthernet || !$hasInternet}
      <Button
        variant="btn-outline btn-primary"
        on:click={() => {
          resetNetworkConfig();
          goto('/setup/wifi/one');
        }}
      >
        Switch to Wi-Fi
      </Button>
    {/if}
  </div>
</Page>
