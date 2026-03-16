<script lang="ts">
  export let status: 'success' | 'warning' | 'error' | 'neutral' = 'success';
</script>

<div class={`badge badge-${status} rounded pt-0.5`}>
  <slot />
</div>
