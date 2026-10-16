<script lang="ts">
  import type { Snippet } from 'svelte';
  let { tone = 'info', children }: { tone?: 'info' | 'error' | 'success' | 'warning'; children: Snippet } = $props();
  const styles = {
    info: 'border-border bg-muted/40 text-foreground',
    error: 'border-destructive/25 bg-destructive/10 text-destructive',
    success: 'border-success/20 bg-success/8 text-success',
    warning: 'border-warning/20 bg-warning/8 text-warning'
  };
</script>

<div role={tone === 'error' ? 'alert' : undefined} class={`rounded-lg border px-4 py-3 text-sm leading-relaxed ${styles[tone]}`}>
  {@render children()}
</div>
