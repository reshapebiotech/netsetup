<script lang="ts">
  import { SpinnerGap } from 'phosphor-svelte';
  export let status: 'success' | 'warning' | 'error' | 'loading' = 'loading';
</script>

<div class="flex gap-1 px-2 py-1 items-center bg-dark-5 rounded-full">
  <div
    class="w-4 h-4 rounded-full"
    class:bg-green-500={status === 'success'}
    class:bg-red-300={status === 'error'}
    class:bg-yellow-500={status === 'warning'}
    class:bg-gray-500={status === 'loading'}
  >
    {#if status === 'loading'}
      <SpinnerGap class="h-full" />{/if}
  </div>
</div>
