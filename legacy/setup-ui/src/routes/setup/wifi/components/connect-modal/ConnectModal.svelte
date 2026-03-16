<script lang="ts">
  import { goto } from '$app/navigation';
  import Alert from '$lib/components/alert/Alert.svelte';
  import Button from '$lib/components/button/Button.svelte';
  import Modal from '$lib/components/modal/Modal.svelte';
  import { networkConfig, robotMetadata } from '$lib/stores';
  import { WarningCircle, X } from 'phosphor-svelte';

  const reshapeNetwork = $robotMetadata?.name ?? '';

  export let goBack: () => void;
</script>

<Modal let:C>
  <C.Dialog id="my_modal_1" open modalBoxSx="w-full sm:min-w-[620px]">
    <div class="max-[600px]:pb-10">
      <div class="border-b-1 py-3 pb-2">
        <div class="flex justify-end px-4">
          <Button iconButton size="sm" on:click={goBack}><X /></Button>
        </div>
      </div>
      <div class="flex flex-col gap-8 py-8 px-6">
        <div class="flex flex-col">
          <h3>Next steps</h3>
        </div>
        <div class="flex flex-col gap-4">
          <div class="flex gap-4 items-center">
            <div
              class="flex items-center justify-center size-10 shrink-0 bg-gray-51 rounded-full font-semibold"
            >
              1
            </div>
            <div>
              <p class="text-base">
                Your robot will disconnect the network <strong>{reshapeNetwork}</strong>
              </p>
            </div>
          </div>
          <div class="flex gap-4 items-center">
            <div
              class="flex items-center justify-center min-w-6 size-10 shrink-0 bg-gray-51 rounded-full font-semibold"
            >
              2
            </div>
            <div>
              <p class="text-base">
                Your robot will attempt to connect to <strong>{$networkConfig.ssid}</strong>
              </p>
            </div>
          </div>
        </div>
        <div class="flex flex-col gap-4">
          <Alert type="warning">
            <div class="flex gap-2 items-center">
              <WarningCircle class="w-6 h-6" />
              <p class="text-sm sm:text-base text-left">
                You'll be unable to connect to your robot during this process
              </p>
            </div>
          </Alert>
        </div>
      </div>
      <div class="border-t-1 py-4">
        <div class="flex justify-end px-4">
          <Button variant="btn-primary" on:click={() => goto('/setup/wifi/connecting')}>
            Connect to network
          </Button>
        </div>
      </div>
    </div>
  </C.Dialog>
</Modal>
