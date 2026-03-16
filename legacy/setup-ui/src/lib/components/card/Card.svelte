<script lang="ts">
  import Radio from '../radio/Radio.svelte';

  const C = {
    Radio,
  };
</script>

<div class="card bg-white-500 p-4 rounded-xl shadow hover:shadow-hover transition-shadow">
  <slot {C} />
</div>
