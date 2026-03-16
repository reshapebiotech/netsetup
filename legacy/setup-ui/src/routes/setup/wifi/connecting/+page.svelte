<script lang="ts">
  import { goto } from '$app/navigation';
  import API, { type types } from '$lib/api';
  import Button from '$lib/components/button/Button.svelte';
  import Page from '$lib/components/page/Page.svelte';
  import { networkConfig, robotMetadata } from '$lib/stores';
  import { resetNetworkConfig } from '$lib/utils';
  import { ArrowUpRight, Circle } from 'phosphor-svelte';
  import { onMount } from 'svelte';

  let errors: string[] = [];
  let connecting = false;

  const containsTuple = (array: any[], tuple: any[]) => {
    return array.some(
      (item) =>
        item.length === tuple.length &&
        item.every((value: any, index: number) => value === tuple[index])
    );
  };

  const determineResult = (states: [string, string][]) => {
    const errors: string[] = [];
    if (containsTuple(states, ['DEACTIVATED', 'NEED_AUTH'])) {
      errors.push('Failed to authenticate. Verify login details.');
    }
    if (containsTuple(states, ['DEACTIVATED', 'UNKNOWN'])) {
      errors.push('Failed to connect.');
    }
    if (containsTuple(states, ['DEACTIVATED', 'IP_CONFIG_INVALID'])) {
      errors.push('Failed to assign an IP address. Verify DNS/DHCP settings.');
    }
    if (containsTuple(states, ['DEACTIVATED', 'LOGIN_FAILED'])) {
      errors.push('Failed to log in. Verify login details.');
    }
    if (containsTuple(states, ['DEACTIVATED', 'DEVICE_DISCONNECTED'])) {
      errors.push('Failed to log in. Verify login details.');
    }
    if (containsTuple(states, ['DEACTIVATED', 'NO_SECRETS'])) {
      errors.push('Failed to log in. Verify login details.');
    }

    return errors;
  };

  const attemptConnection = async () => {
    connecting = true;

    const { data: pastState, error: stateError } = await API().GET('/state');
    if (stateError) {
      console.error(stateError);
      return;
    }

    const result: types['NetworkSettings-Input'] = {
      ssid: $networkConfig.ssid,
      password: $networkConfig.password,
      eap_login: $networkConfig.eap_login ? $networkConfig.eap_login : undefined,
      eap_password: $networkConfig.eap_password ? $networkConfig.eap_password : undefined,
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

    API()
      .POST('/wireless-settings', {
        body: result,
      })
      .then((r) => {
        if (r.error) {
          errors.push('Invalid network settings. Verify login details.');
        }
      });

    resetNetworkConfig();
    let remainingAttempts = 120;
    const interval = setInterval(async () => {
      try {
        const { data: currentState } = await API().GET('/state');
        const currentConnectionAttempt = currentState?.connection_attempt;

        const states = currentConnectionAttempt?.connection_result_states ?? [];
        connecting = false;

        if (containsTuple(states, ['ACTIVATED', 'UNKNOWN'])) {
          clearInterval(interval);
          goto('/');
          return;
        }

        const result = determineResult(states);
        if (result.length > 0) {
          errors = Array.from(new Set([...errors, ...result]));
          clearInterval(interval);
          return;
        }

        if (remainingAttempts > 0) {
          return;
        }
      } catch (error) {
        remainingAttempts = remainingAttempts - 1;
      }
    }, 1000);
  };
  onMount(() => {
    attemptConnection();
  });
</script>

<Page let:C>
  <C.Stepper numberOfSteps={5} currentStep={5} />
  <C.Section>
    <C.Title>Connecting your device...</C.Title>
    <C.Description
      >This may take a few minutes. Check your device’s indicator lights to confirm its connection
      status.</C.Description
    >
    <div>
      {#each errors as alert}
        <div role="alert" class="alert alert-error text-white-500">
          <span>{alert}</span>
        </div>
      {/each}
    </div>
  </C.Section>
  <div class="flex flex-col rounded-lg shadow p-4 sm:p-6 gap-6 bg-white-500">
    <h4>Indicator lights meaning</h4>
    <div class="flex gap-4 pb-2">
      <img alt="logo" src="/breathing_white.gif" class="size-6 sm:size-7 mt-1" />
      <div class="flex flex-col gap-1">
        <h5 class="max-[600px]:text-sm">Breathing white</h5>
        <p class="max-[600px]:text-sm">
          The device is powering on. Wait until the light changes to yellow to begin connecting.
        </p>
      </div>
    </div>
    <div class="flex gap-4 pb-2">
      <img alt="logo" src="/blinking_yellow.gif" class="size-6 sm:size-7 mt-1" />
      <div class="flex flex-col gap-1">
        <h5 class="max-[600px]:text-sm">Blinking yellow</h5>
        <p class="pb-1 max-[600px]:text-sm">
          Your device is attempting to connect. If the Reconnect to your robot's Wi-Fi network: <strong
            >{$robotMetadata.name?.toUpperCase()}</strong
          >.
        </p>
        <ul class="flex flex-col gap-2">
          <li class="flex gap-4 items-center max-[600px]:text-xs">
            <Circle weight="fill" size={8} />
            Your computer, phone or tablet might have connected to a different network while the robot
            tried to connect.
          </li>
          <li class="flex gap-4 items-center max-[600px]:text-xs">
            <Circle weight="fill" size={8} /> To reconnect to your robot, find the local access point
            named
            {$robotMetadata.name?.toUpperCase()} on your computer, phone or tablet.
          </li>
        </ul>
      </div>
    </div>
    <div class="flex gap-4 pb-2">
      <img alt="logo" src="/static_white.png" class="size-6 sm:size-7 mt-1" />
      <div class="flex flex-col gap-1">
        <h5 class="max-[600px]:text-sm">Static white</h5>
        <p class="max-[600px]:text-sm">
          Your device has connected successfully. This will cause your computer, phone or tablet to
          lose connection to the robot.
        </p>
      </div>
    </div>
  </div>
  <div class="flex flex-col-reverse sm:flex-row sm:justify-end gap-2 pb-32">
    <form action="mailto:support@reshapebiotech.com">
      <Button type="submit" variant="btn-outline btn-primary">
        <div class="gap-2 inline-flex items-center">
          Contact support
          <ArrowUpRight size={16} weight="bold" />
        </div>
      </Button>
    </form>
  </div>
</Page>
