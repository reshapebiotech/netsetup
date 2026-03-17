<script lang="ts">
  import { cn } from '$lib/utils/cn';
  export let value = '';
  export let type: 'text' | 'password' | 'number' = 'text';
  export let placeholder = '';
  export let disabled = false;
  export let className = '';
</script>

<input
  bind:value
  {type}
  {placeholder}
  {disabled}
  class={cn(
    'w-full rounded-md border border-input bg-white px-3 py-2 text-sm text-foreground outline-none ring-offset-background focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-60',
    className
  )}
/>
