<script lang="ts">
  import { goto } from '$app/navigation';
  import PageActions from '#lib/components/ui/page-actions.svelte';
  import Button from '#lib/components/ui/button.svelte';
  import Card from '#lib/components/ui/card.svelte';
  import Icon from '#lib/components/ui/icon.svelte';
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">New Profile</p>
    <h1 class="text-3xl font-semibold">Choose connection type</h1>
    <p class="text-sm text-muted-foreground">Choose how to connect. Your existing connection stays in place until you test and save a replacement.</p>
  </header>

  <div class="grid gap-4 sm:grid-cols-2">
    <Card className="space-y-3">
      <h2 class="flex items-center gap-2 text-xl font-semibold">
        <Icon name="wifi" className="h-5 w-5 text-primary" />
        Wi-Fi
      </h2>
      <p class="text-sm text-muted-foreground">Scan and test wireless networks, including enterprise credentials.</p>
      <Button onclick={() => goto('/setup/new/wifi')}>
        <span class="mr-2 inline-flex items-center"><Icon name="wifi" /></span>
        Use Wi-Fi
      </Button>
    </Card>

    <Card className="space-y-3">
      <h2 class="flex items-center gap-2 text-xl font-semibold">
        <Icon name="ethernet" className="h-5 w-5 text-primary" />
        Ethernet
      </h2>
      <p class="text-sm text-muted-foreground">Configure wired network with DHCP or manual IPv4 + DNS settings.</p>
      <Button onclick={() => goto('/setup/new/ethernet')}>
        <span class="mr-2 inline-flex items-center"><Icon name="ethernet" /></span>
        Use Ethernet
      </Button>
    </Card>
  </div>
  <PageActions>{#snippet secondary()}<Button variant="secondary" onclick={() => goto('/')}>Cancel</Button>{/snippet}</PageActions>
</div>
