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
    primary: 'border border-transparent bg-primary text-primary-foreground hover:brightness-110',
    secondary: 'border border-border bg-muted/40 text-foreground hover:bg-muted',
    destructive: 'border border-destructive/20 bg-destructive/5 text-destructive hover:bg-destructive/15'
  };
</script>

<button
  {...attributes}
  {type}
  disabled={isDisabled}
  aria-busy={loading}
  class={cn(
    'inline-flex min-h-11 items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-60',
    styles[variant],
    className
  )}
>
  {#if loading}
    <span class="h-4 w-4 animate-spin rounded-full border-2 border-current border-r-transparent"></span>
  {/if}
  {@render children?.()}
</button>
