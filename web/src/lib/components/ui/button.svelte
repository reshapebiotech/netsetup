<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import { cn } from '#lib/utils/cn.js';

  type Props = Omit<HTMLButtonAttributes, 'class'> & {
    variant?: 'primary' | 'secondary' | 'destructive';
    loading?: boolean;
    className?: string;
  };

  let {
    type = 'button', variant = 'primary', disabled = false, loading = false,
    className = '', children, ...attributes
  }: Props = $props();
  const isDisabled = $derived(disabled || loading);

  const styles = {
    primary: 'bg-primary text-primary-foreground hover:opacity-90',
    secondary: 'bg-muted text-foreground hover:bg-muted/80',
    destructive: 'bg-destructive text-destructive-foreground hover:opacity-90'
  };
</script>

<button
  {...attributes}
  {type}
  disabled={isDisabled}
  aria-busy={loading}
  class={cn(
    'inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-semibold transition disabled:cursor-not-allowed disabled:opacity-60',
    styles[variant],
    className
  )}
>
  {#if loading}
    <span class="h-4 w-4 animate-spin rounded-full border-2 border-current border-r-transparent"></span>
  {/if}
  {@render children?.()}
</button>
