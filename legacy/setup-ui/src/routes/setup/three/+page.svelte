<script lang="ts">
  import Card from '$lib/components/card/Card.svelte';
  import Page from '$lib/components/page/Page.svelte';
  import Button from '$lib/components/button/Button.svelte';
  import { goto } from '$app/navigation';
  import { WifiHigh, Network } from 'phosphor-svelte';

  let checkedValue: 'ethernet' | 'wifi' | '' = '';

  const continueToNextPage = () => {
    if (checkedValue === 'ethernet') {
      goto('/setup/ethernet/one');
    } else {
      goto('/setup/wifi/one');
    }
  };
</script>

<Page let:C>
  <C.Stepper numberOfSteps={5} currentStep={2} />
  <C.Title>Select network type</C.Title>
  <div class="flex flex-col gap-4">
    <label for="ethernet" class="cursor-pointer">
      <Card let:C>
        <div class="flex justify-between items-center">
          <div class="flex items-center gap-4">
            <div class="flex items-center justify-center size-10 bg-dark-5 rounded-full shrink-0">
              <Network size="24" class="text-dark-70" />
            </div>
            <div class="flex flex-col">
              <h5>Ethernet</h5>
              <p class="text-sm text-dark-70">Recommended for best performance</p>
            </div>
          </div>
          <C.Radio id="ethernet" bind:group={checkedValue} value="ethernet" />
        </div>
      </Card>
    </label>
    <label for="wifi" class="cursor-pointer">
      <Card let:C>
        <div class="flex justify-between items-center">
          <div class="flex items-center gap-4">
            <div class="flex items-center justify-center size-10 bg-dark-5 rounded-full shrink-0">
              <WifiHigh size="24" class="text-dark-70" />
            </div>
            <div class="flex flex-col">
              <h5>Wi-Fi</h5>
              <p class="text-sm text-dark-70">Minimum 3 Mb/s upload and download recommended</p>
            </div>
          </div>
          <C.Radio id="wifi" bind:group={checkedValue} value="wifi" />
        </div>
      </Card>
    </label>
  </div>
  <div class="flex flex-col sm:flex-row-reverse gap-3 mt-1">
    <Button variant="btn-primary" on:click={continueToNextPage}>Continue</Button>
    <Button variant="btn-outline btn-primary" on:click={() => goto('/setup/one')}>Previous</Button>
  </div>
</Page>
