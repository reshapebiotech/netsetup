<script>
  import { goto } from '$app/navigation';
  import Button from '$lib/components/button/Button.svelte';
  import Page from '$lib/components/page/Page.svelte';

  import { Check } from 'phosphor-svelte';
</script>

<Page let:C>
  <C.Stepper numberOfSteps={5} currentStep={1} />
  <C.Title>Set up your device</C.Title>

  <div class="flex flex-col gap-2">
    <div class="pb-6">
      <ul class="flex flex-col bg-white-500 rounded-xl shadow p-6 gap-4">
        <h5 class="pb-2">What you'll need</h5>
        <li class="flex gap-2 items-center text-sm sm:text-base">
          <Check class="text-green-500 min-w-6" width="24" weight="bold" /> Ethernet or Wi-Fi network
          connection
        </li>
        <li class="flex gap-2 items-center text-sm sm:text-base">
          <Check class="text-green-500 min-w-6" width="24" weight="bold" /> Minimum internet connection
          of 3 Mb/s upload and download per device
        </li>
        <li class="flex gap-2 items-center text-sm sm:text-base">
          <Check class="text-green-500  min-w-6" width="24" weight="bold" /> Any custom network settings
          (check with your IT Manager)
        </li>
      </ul>
    </div>
    <div class="flex flex-col items-end">
      <Button variant="btn-primary" on:click={() => goto('/setup/three')}>Get started</Button>
    </div>
  </div>
</Page>
