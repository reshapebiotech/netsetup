<script lang="ts">
  import { isLoading } from '$lib/stores';
  import { ArrowRight, SpinnerGap } from 'phosphor-svelte';

  let loading = false;

  export let iconColor: string = 'bg-dark-5';
  export let onNavigate: (() => Promise<any>) | (() => any) = async () => {};

  const handleClick = async () => {
    $isLoading = true;
    loading = true;
    await onNavigate();
    loading = false;
    $isLoading = false;
  };
</script>

<button
  class="card bg-white-500 p-4 rounded-xl shadow hover:shadow-hover transition-shadow"
  on:click={handleClick}
>
  <div class="flex items-center w-full gap-4">
    <div class="flex items-center justify-center size-10 {iconColor} rounded-full">
      <slot name="icon" />
    </div>
    <div class="flex-1 text-start">
      <slot />
    </div>
    {#if loading}
      <SpinnerGap size="24" class="animate-spin text-dark-70" />
    {:else}
      <ArrowRight size="24" class="text-dark-70" />
    {/if}
  </div>
</button>
