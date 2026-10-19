<script lang="ts">
  import type { Snippet } from 'svelte';

  let { primary, secondary, note }: { primary?: Snippet; secondary?: Snippet; note?: string } = $props();

  function reserveSpace(element: HTMLElement) {
    const root = document.documentElement;
    const pinned = window.matchMedia('(max-width: 640px)');
    const measure = () => {
      const height = pinned.matches ? Math.ceil(element.getBoundingClientRect().height) : 0;
      root.style.setProperty('--netsetup-actions-height', `${height}px`);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    pinned.addEventListener('change', measure);
    return () => {
      observer.disconnect();
      pinned.removeEventListener('change', measure);
      root.style.removeProperty('--netsetup-actions-height');
    };
  }
</script>

<section aria-label="Page actions" class="page-actions" {@attach reserveSpace}>
  <div class="actions-content">
    {#if note}<p class="text-xs text-muted-foreground">{note}</p>{/if}
    <div class="actions-row">
      {#if secondary}<div class="secondary">{@render secondary()}</div>{/if}
      {#if primary}<div class="primary">{@render primary()}</div>{/if}
    </div>
  </div>
</section>

<style>
  .actions-content { display: grid; gap: 0.625rem; }
  .actions-row { display: flex; flex-wrap: wrap; align-items: center; gap: 0.75rem; }
  .secondary, .primary { min-width: 0; max-width: 100%; }
  .primary { margin-left: auto; }
  @media (max-width: 640px) {
    .page-actions {
      position: fixed;
      inset: auto 0 0;
      z-index: 10;
      border-top: 1px solid hsl(var(--border));
      background: hsl(var(--background));
      padding-inline: env(safe-area-inset-left, 0px) env(safe-area-inset-right, 0px);
      padding-block: 0.875rem calc(0.875rem + env(safe-area-inset-bottom, 0px));
      box-shadow: 0 -0.5rem 1.5rem hsl(var(--background) / 0.5);
    }
    .actions-content { width: min(100% - 3rem, 56rem); margin-inline: auto; }
  }
  @media (max-width: 540px) {
    .actions-content { width: calc(100% - 2rem); }
  }
</style>
