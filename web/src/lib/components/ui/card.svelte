<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '#lib/utils/cn.js';
  let { className = '', children }: { className?: string; children: Snippet } = $props();
</script>

<section class={cn('rounded-xl border border-border bg-card p-5 text-card-foreground sm:p-6', className)}>
  {@render children()}
</section>
