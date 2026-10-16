<script lang="ts">
  import { cn } from '#lib/utils/cn.js';
  let { value = $bindable(''), options = [], disabled = false, className = '' }: {
    value?: string; options?: Array<{ label: string; value: string }>;
    disabled?: boolean; className?: string;
  } = $props();
</script>

<select bind:value {disabled} class={cn('min-h-11 w-full rounded-lg border border-input bg-background/60 px-3 py-2.5 text-base text-foreground outline-none focus:border-primary focus:ring-1 focus:ring-primary disabled:opacity-50 sm:text-sm', className)}>
  {#each options as option (option.value)}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>
