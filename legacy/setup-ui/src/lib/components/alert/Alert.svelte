<script lang="ts">
  export let type: 'info' | 'success' | 'warning' | 'error' = 'info';
</script>

<div role="alert" class={`alert alert-${type} p-2 sm:p-4 rounded-lg`}>
  <slot />
</div>
