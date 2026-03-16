<script lang="ts">
  import { ArrowRight, WarningCircle } from 'phosphor-svelte';
  export let variant: 'bg-yellow-500' | 'bg-red-300';
</script>

<button class="card {variant} p-4 rounded-lg shadow" on:click>
  <div class="flex items-center w-full gap-4">
    <div class="flex items-center justify-center w-10 h-10 rounded-full">
      <div>
        <WarningCircle size="24" />
      </div>
    </div>
    <div class="flex-1 text-start">
      <slot />
    </div>
    <div class="btn btn-outline btn-xs text-sm font-normal rounded-full">
      <span class="pt-0.5">Configure</span>
      <ArrowRight size="16" />
    </div>
  </div>
</button>
