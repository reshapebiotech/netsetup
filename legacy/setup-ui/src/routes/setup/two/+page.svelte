<script>
  import { goto } from '$app/navigation';
  import Badge from '$lib/components/badge/Badge.svelte';
  import Button from '$lib/components/button/Button.svelte';
  import Page from '$lib/components/page/Page.svelte';
</script>

<Page let:C>
  <C.Stepper numberOfSteps={5} currentStep={2} />
  <C.Title>Isolating your robot</C.Title>
  <C.Badge status="warning">RECOMMENDED</C.Badge>
  <C.Description>
    Isolating your robot from the internet is recommended for security reasons. This will prevent
    your robot from communicating with the internet and other devices on the network.
  </C.Description>

  <div class="flex flex-col- gap-3"></div>
  <div class="flex flex-col gap-2">
    <h4>Requirements</h4>
    <div class="bg-white-500 rounded p-4 flex flex-col gap-6">
      <div class="section flex-col">
        <h5>Domains</h5>
        <div class="flex gap-1">
          <Badge status="neutral">*.RESHAPEBIOTECH.COM</Badge>
          <p class="inline-block">domain and subdomains must be reachable.</p>
        </div>
      </div>
      <div class="section">
        <h5>TLS/HTTPS interception</h5>
        <p>Not supported</p>
      </div>
      <div class="section">
        <h5>IP Adresses</h5>
        <p>
          Cannot be restricted -- there is no set of static outbound IP addresses that your robot
          communicates with.
        </p>
      </div>
    </div>
  </div>
  <div class="flex gap-2 justify-end">
    <Button variant="btn-outline btn-primary" on:click={() => goto('/setup/one')}>Previous</Button>
    <Button variant="btn-primary" on:click={() => goto('/setup/three')}>Continue</Button>
  </div>
</Page>

<style lang="postcss">
  .section {
    @apply flex flex-col gap-1;
  }
</style>
