<script lang="ts">
  export let tone: 'info' | 'error' = 'info';
  const styles = {
    info: 'border-primary/30 bg-primary/10 text-foreground',
    error: 'border-destructive/30 bg-destructive/10 text-foreground'
  };
</script>

<div class={`rounded-md border p-3 text-sm ${styles[tone]}`}>
  <slot />
</div>
