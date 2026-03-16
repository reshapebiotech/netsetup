<script lang="ts">
  export let variant:
    | 'btn-primary'
    | 'btn-secondary'
    | 'btn-success'
    | 'btn-error'
    | 'btn-ghost'
    | 'btn-outline btn-primary'
    | 'btn-link no-underline' = 'btn-primary';
  export let size: 'xs' | 'sm' | 'md' | 'lg' = 'md';
  export let loading = false;
  export let disabled = false;
  export let iconButton = false;
  export let type: 'button' | 'submit' | 'reset' = 'button';
</script>

{#if iconButton}
  <button {type} class={`btn btn-circle btn-${size}`} {disabled} on:click>
    {#if loading}
      <span class="loading loading-spinner"></span>
    {:else}
      <slot />
    {/if}
  </button>
{:else}
  <button
    {type}
    class={`w-full sm:w-fit btn ${variant} ${variant === 'btn-primary' ? 'text-white-500' : ''} rounded- px-6 py-4`}
    {disabled}
    on:click
  >
    {#if loading}
      <span class="loading loading-spinner"></span>
    {:else}
      <slot />
    {/if}
  </button>
{/if}
