<script lang="ts">
  import { cn } from '$lib/utils/cn';

  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let variant: 'primary' | 'secondary' | 'destructive' = 'primary';
  export let disabled = false;
  export let loading = false;
  export let className = '';
  export let onclick: ((event: MouseEvent) => void) | undefined = undefined;
  $: isDisabled = disabled || loading;

  const styles = {
    primary: 'bg-primary text-primary-foreground hover:opacity-90',
    secondary: 'bg-muted text-foreground hover:bg-muted/80',
    destructive: 'bg-destructive text-destructive-foreground hover:opacity-90'
  };
</script>

<button
  {type}
  disabled={isDisabled}
  aria-busy={loading}
  on:click={onclick}
  class={cn(
    'inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-semibold transition disabled:cursor-not-allowed disabled:opacity-60',
    styles[variant],
    className
  )}
>
  {#if loading}
    <span class="h-4 w-4 animate-spin rounded-full border-2 border-current border-r-transparent"></span>
  {/if}
  <slot />
</button>
