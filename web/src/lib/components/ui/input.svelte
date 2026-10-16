<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import { cn } from '#lib/utils/cn.js';

  type Props = Omit<HTMLInputAttributes, 'value' | 'class'> & {
    value?: string;
    className?: string;
  };

  let { value = $bindable(''), type = 'text', className = '', ...attributes }: Props = $props();
</script>

<input
  {...attributes}
  bind:value
  {type}
  class={cn(
    'w-full min-h-11 rounded-lg border border-input bg-background/60 px-3 py-2.5 text-base text-foreground outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary disabled:cursor-not-allowed disabled:opacity-60 sm:text-sm',
    className
  )}
/>
