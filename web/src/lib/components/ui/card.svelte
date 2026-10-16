<script lang="ts">
  import { cn } from '#lib/utils/cn.js';
  export let className = '';
</script>

<section class={cn('rounded-xl border border-border bg-card p-5 text-card-foreground shadow-xs', className)}>
  <slot />
</section>
