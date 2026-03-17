<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { branding, loadBranding } from '$lib/branding';

  onMount(() => {
    void loadBranding();
  });
</script>

<svelte:head>
  <title>{$branding.product_name}</title>
  <link rel="icon" href={$branding.favicon_path} />
</svelte:head>

<div
  class="mx-auto min-h-screen w-full max-w-3xl px-4 py-8 sm:px-6 lg:px-8"
  style={$branding.background_path ? `background-image: url(${$branding.background_path}); background-size: cover; background-position: center;` : undefined}
>
  {#if $branding.logo_path}
    <div class="mb-6">
      <a href="/" aria-label="Back to home">
        <img src={$branding.logo_path} alt="{$branding.product_name} logo" class="h-auto max-h-16 w-auto object-contain" />
      </a>
    </div>
  {/if}
  <slot />
</div>
