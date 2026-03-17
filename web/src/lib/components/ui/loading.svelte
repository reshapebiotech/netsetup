<script lang="ts">
  export let label = 'Loading…';
  export let size: 'sm' | 'md' = 'md';
  const cls = {
    sm: 'h-4 w-4 border-2',
    md: 'h-5 w-5 border-2'
  };
</script>

<div class="flex items-center gap-2 text-sm text-muted-foreground" role="status" aria-live="polite">
  <span class={`${cls[size]} animate-spin rounded-full border-primary border-r-transparent`}></span>
  <span>{label}</span>
</div>
