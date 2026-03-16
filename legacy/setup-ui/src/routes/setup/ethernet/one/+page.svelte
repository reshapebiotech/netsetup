<script lang="ts">
  import Loading from '$lib/components/loading/Loading.svelte';
  import Page from '$lib/components/page/Page.svelte';
  import { CheckCircle, WarningCircle } from 'phosphor-svelte';
  import API from '$lib/api';
  import { onMount } from 'svelte';
  import { hasEapEthernet, hasNoEthernet, networks, hasInternet } from '$lib/stores';
  import { goto } from '$app/navigation';

  const fetchNetworks = async () => {
    const { data: networkData, error: networkError } = await API().GET('/wired-networks');
    if (!networkError) {
      networks.set(networkData ?? undefined);
    }

    hasNoEthernet.set(networkData?.networks.length === 0);
    hasEapEthernet.set(
      networkData?.networks.some((network) => network.security === 'EAP') ?? false
    );

    const { data: networkStatus, error: networkStatusError } = await API().GET('/connectivity');

    if (networkStatusError) {
      hasInternet.set(false);
    }

    hasInternet.set(networkStatus?.can_reach_host ?? false);
  };

  const forward = () => {
    goto('/setup/ethernet/two');
  };

  onMount(() => {
    fetchNetworks().then(() => {
      setTimeout(() => {
        if ($hasEapEthernet || $hasNoEthernet || !$hasInternet || $networks === undefined) {
          forward();
        } else {
          setTimeout(() => {
            goto('/');
          }, 3000);
        }
      }, 3000);
    });
  });
</script>

<Page let:C>
  <C.Stepper numberOfSteps={5} currentStep={3} />
  <C.Section>
    <C.Title>Checking network...</C.Title>
    <C.Description>This may take a few minutes.</C.Description>
  </C.Section>

  <div class="flex flex-col p-4 rounded items-center gap-4">
    {#if networks === undefined}
      <Loading size="lg" />
    {:else if $hasNoEthernet}
      <WarningCircle size="lg" class="w-14 text-yellow-300" />
    {:else if $hasEapEthernet || !$hasInternet}
      <WarningCircle size="lg" class="w-14 text-yellow-500" />
    {:else}
      <CheckCircle size="lg" class="w-14 text-green-500" />
    {/if}
  </div>
</Page>
