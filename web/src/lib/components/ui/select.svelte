<script lang="ts">
  import { cn } from '#lib/utils/cn.js';
  export let value = '';
  export let options: Array<{ label: string; value: string }> = [];
  export let disabled = false;
  export let className = '';
</script>

<select
  bind:value
  {disabled}
  class={cn('w-full rounded-md border border-input bg-white px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring', className)}
>
  {#each options as option}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>
