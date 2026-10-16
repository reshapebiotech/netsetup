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
    'w-full rounded-md border border-input bg-white px-3 py-2 text-base text-foreground outline-none ring-offset-background focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-60 sm:text-sm',
    className
  )}
/>
