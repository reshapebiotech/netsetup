<script lang="ts">
  import { goto } from '$app/navigation';
  import Button from '$lib/components/ui/button.svelte';
  import Card from '$lib/components/ui/card.svelte';
</script>

<div class="space-y-6">
  <header class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">New Profile</p>
    <h1 class="text-3xl font-bold">Choose connection type</h1>
  </header>

  <div class="grid gap-4 sm:grid-cols-2">
    <Card className="space-y-3">
      <h2 class="text-xl font-semibold">Wi-Fi</h2>
      <p class="text-sm text-muted-foreground">Scan and test wireless networks, including enterprise credentials.</p>
      <Button onclick={() => goto('/setup/new/wifi')}>Use Wi-Fi</Button>
    </Card>

    <Card className="space-y-3">
      <h2 class="text-xl font-semibold">Ethernet</h2>
      <p class="text-sm text-muted-foreground">Configure wired network with DHCP or manual IPv4 + DNS settings.</p>
      <Button onclick={() => goto('/setup/new/ethernet')}>Use Ethernet</Button>
    </Card>
  </div>
</div>
