<script lang="ts">
  import { goto } from '$app/navigation';
  import NavigationCard from '$lib/components/card/NavigationCard.svelte';
  import Page from '$lib/components/page/Page.svelte';
  import { robotMetadata } from '$lib/stores';
  import { resetSettings, resetNetworkConfig } from '$lib/utils';
  import { ClockCounterClockwise, Globe, Wrench } from 'phosphor-svelte';
</script>

<Page let:C>
  <C.Section>
    <div class="flex flex-row justify-between items-center">
      <C.Title>Get started</C.Title>
      <C.RobotTag robotName={$robotMetadata?.name ?? ''} />
    </div>
  </C.Section>
  <div class="flex flex-col gap-4">
    <NavigationCard
      onNavigate={async () => {
        resetNetworkConfig();
        await goto('/setup/one');
      }}
      iconColor="bg-periwinkle-1"
    >
      <div slot="icon">
        <Wrench size="24" class="text-periwinkle-5" />
      </div>
      <div>
        <h5>Set up device</h5>
        <p class="text-dark-70">Connect your device to the internet</p>
      </div>
    </NavigationCard>
    <hr class="border-dark-10 my-4" />
    <h5>Other actions</h5>
    <NavigationCard onNavigate={() => goto('/settings')}>
      <div slot="icon">
        <Globe size="24" class="text-dark-70" />
      </div>
      <h5>Network settings</h5>
    </NavigationCard>
    <NavigationCard onNavigate={resetSettings}>
      <div slot="icon">
        <ClockCounterClockwise size="24" class="text-dark-70" />
      </div>
      <h5>Reset settings</h5>
    </NavigationCard>
  </div>
</Page>
