<script lang="ts">
  import API, { type types } from '$lib/api';
  import { form, field } from 'svelte-forms';
  import { pattern } from 'svelte-forms/validators';
  import Button from '$lib/components/button/Button.svelte';
  import Input from '$lib/components/input/Input.svelte';
  import Loading from '$lib/components/loading/Loading.svelte';
  import Page from '$lib/components/page/Page.svelte';
  import { ArrowRight, Eye, WifiHigh, WifiLow, WifiMedium } from 'phosphor-svelte';
  import { onMount } from 'svelte';
  import ConnectModal from '../components/connect-modal/ConnectModal.svelte';
  import WifiSettings from '../components/wifi-settings/WifiSettings.svelte';
  import type { PageData } from './$types';
  import { networkConfig } from '$lib/stores';
  import { goto } from '$app/navigation';

  export let data: PageData;

  let selectedNetwork: types['Network'] | undefined = undefined;

  let configStandard: string = 'standard';

  let step: 1 | 2 | 3 | 4 = 1;

  let networks: types['Networks'] = data;

  const fetchNetworks = async () => {
    const { data, error } = await API().GET('/wireless-networks');
    if (!error) {
      networks = data ?? [];
    }
    networks = data;
  };

  const backward = () => {
    const newStep = step - 1;
    if (newStep === 0) {
      goto('/setup/three');
    } else {
      step = step - 1;
    }
    selectedNetwork = undefined;
  };

  const createInterval = () => setInterval(fetchNetworks, 3000);

  let interval: number | undefined = undefined;

  const password = field('password', $networkConfig.password, [pattern(/^.{8,}$/)], {
    checkOnInit: true,
  });

  $: $networkConfig.password = $password.value;

  const values = form(password);

  let showWiFiPassword = false;
  let showEAPPassword = false;

  onMount(() => {
    interval = createInterval();
    return () => clearInterval(interval);
  });
</script>

<Page let:C>
  {#if step === 1}
    <C.Stepper numberOfSteps={5} currentStep={3} />
    <C.Title>Select Wi-Fi network</C.Title>
    <div
      class="flex flex-col bg-white-500 p-2 rounded-xl shadow items-center gap-2 overflow-y-auto h-1/2"
    >
      {#if networks === undefined}
        <Loading size="md" />
        Searching for networks
      {:else}
        <Loading size="md" />
        {#each networks.networks as network}
          <!-- svelte-ignore a11y-click-events-have-key-events -->
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <div
            class="flex justify-between items-center w-full cursor-pointer hover:bg-dark-5 transition-colors rounded-lg p-3"
            on:click={() => {
              step = 2;
              selectedNetwork = network;
              $networkConfig.ssid = network.ssid;
              clearInterval(interval);
            }}
          >
            <div class="flex gap-4 items-center">
              <div class="size-10 rounded-full bg-dark-5 flex items-center justify-center">
                {#if network.strength > 80}
                  <WifiHigh size="20" class="text-dark-70" />
                {:else if network.strength > 50}
                  <WifiMedium size="20" class="text-dark-70" />
                {:else}
                  <WifiLow size="20" class="text-dark-70" />
                {/if}
              </div>
              <p class="text-base">{network.ssid}</p>
            </div>
            <ArrowRight size="20" class="text-dark-70" />
          </div>
        {/each}
      {/if}
    </div>
    <div class="flex flex-row-reverse gap-2">
      <Button
        variant="btn-outline btn-primary"
        on:click={() => {
          const newStep = step - 1;
          if (newStep === 0) {
            goto('/setup/three');
          } else {
            step = step - 1;
          }
          selectedNetwork = undefined;
        }}>Previous</Button
      >
    </div>
  {:else if step === 2 && selectedNetwork}
    <C.Stepper numberOfSteps={5} currentStep={3} />
    <C.Title>Join "{selectedNetwork.ssid}"</C.Title>

    <div class="flex flex-col bg-white-500 rounded-xl shadow p-5 gap-4">
      <h5>Enter password</h5>
      <div class="flex flex-row">
        <Input
          placeholder="Password"
          isPassword={!showWiFiPassword}
          name="password"
          bind:value={$password.value}
        />
        <div class="self-center -ml-16">
          <Button
            variant="btn-ghost"
            on:click={(e) => {
              showWiFiPassword = !showWiFiPassword;
            }}
          >
            <Eye weight="bold" class="text-dark-70" />
          </Button>
        </div>
      </div>
    </div>
    {#if selectedNetwork.security === 'EAP'}
      <div class="flex flex-col bg-white-500 rounded-xl shadow p-5 gap-4">
        <h5>Enterprise login</h5>
        <Input
          placeholder="Username"
          isPassword={false}
          name="eapLogin"
          bind:value={$networkConfig.eap_login}
        />
        <div class="flex flex-row">
          <Input
            placeholder="Password"
            isPassword={!showEAPPassword}
            name="eapPassword"
            bind:value={$networkConfig.eap_password}
          />
          <div class="self-center -ml-16">
            <Button
              variant="btn-ghost"
              on:click={(e) => {
                showEAPPassword = !showEAPPassword;
              }}
            >
              <Eye />
            </Button>
          </div>
        </div>
      </div>
    {/if}
    <div class="flex flex-col-reverse sm:flex-row sm:justify-end gap-2">
      <Button
        variant="btn-outline btn-primary"
        on:click={() => {
          backward();
          createInterval();
        }}>Previous</Button
      >
      <Button variant="btn-primary" on:click={() => (step = 3)} disabled={!$values.valid}
        >Continue</Button
      >
    </div>
  {:else if step === 3}
    <C.Stepper numberOfSteps={5} currentStep={4} />
    <C.Title>Network configuration</C.Title>
    <WifiSettings
      configChoice={configStandard}
      goBack={() => (step = 2)}
      goForward={() => {
        step = 4;
      }}
    />
  {:else if step === 4 && selectedNetwork}
    <ConnectModal goBack={() => (step = 3)} />
  {/if}
</Page>
