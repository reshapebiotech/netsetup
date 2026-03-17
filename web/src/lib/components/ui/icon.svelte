<script lang="ts">
  export let name:
    | 'wifi'
    | 'ethernet'
    | 'settings'
    | 'refresh'
    | 'plus'
    | 'home'
    | 'help'
    | 'link'
    | 'check'
    | 'alert'
    | 'save'
    | 'back'
    | 'network'
    | 'eye'
    | 'eye-off';
  export let className = 'h-4 w-4';
</script>

{#if name === 'wifi'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M5 12.55a11 11 0 0 1 14.08 0" />
    <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
    <path d="M12 20h.01" />
    <path d="M2 8.82a16 16 0 0 1 20 0" />
  </svg>
{:else if name === 'ethernet'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M6 8h12v8H6z" />
    <path d="M8 16v2M12 16v2M16 16v2" />
    <path d="M4 6h16" />
  </svg>
{:else if name === 'settings'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1 1.55V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1-1.55 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.55-1H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.55-1 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h.01a1.7 1.7 0 0 0 1-1.55V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1 1.55h.01a1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v.01a1.7 1.7 0 0 0 1.55 1H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.55 1z" />
  </svg>
{:else if name === 'refresh'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M21 2v6h-6" />
    <path d="M3 12a9 9 0 0 1 15.5-6.36L21 8" />
    <path d="M3 22v-6h6" />
    <path d="M21 12a9 9 0 0 1-15.5 6.36L3 16" />
  </svg>
{:else if name === 'plus'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M12 5v14M5 12h14" />
  </svg>
{:else if name === 'home'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M3 10.5 12 3l9 7.5" />
    <path d="M5 9.5V21h14V9.5" />
  </svg>
{:else if name === 'help'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <circle cx="12" cy="12" r="10" />
    <path d="M9.1 9a3 3 0 1 1 5.8 1c0 2-2.9 2-2.9 4" />
    <path d="M12 17h.01" />
  </svg>
{:else if name === 'link'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M10 13a5 5 0 0 0 7.07 0l2.83-2.83a5 5 0 0 0-7.07-7.07L10 5" />
    <path d="M14 11a5 5 0 0 0-7.07 0L4.1 13.83a5 5 0 0 0 7.07 7.07L14 19" />
  </svg>
{:else if name === 'check'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <circle cx="12" cy="12" r="10" />
    <path d="m8 12 3 3 5-5" />
  </svg>
{:else if name === 'alert'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M12 9v4M12 17h.01" />
    <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.72 3h16.92a2 2 0 0 0 1.72-3l-8.47-14.14a2 2 0 0 0-3.42 0z" />
  </svg>
{:else if name === 'save'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
    <path d="M17 21v-8H7v8M7 3v5h8" />
  </svg>
{:else if name === 'back'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M19 12H5" />
    <path d="m12 19-7-7 7-7" />
  </svg>
{:else if name === 'eye'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M2.06 12.35a1 1 0 0 1 0-.7C3.49 8.23 7.07 6 12 6s8.51 2.23 9.94 5.65a1 1 0 0 1 0 .7C20.51 15.77 16.93 18 12 18s-8.51-2.23-9.94-5.65z" />
    <circle cx="12" cy="12" r="3" />
  </svg>
{:else if name === 'eye-off'}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="m3 3 18 18" />
    <path d="M10.58 10.58A2 2 0 0 0 12 14a2 2 0 0 0 1.42-.58" />
    <path d="M9.88 5.09A10.94 10.94 0 0 1 12 5c4.93 0 8.51 2.23 9.94 5.65a1 1 0 0 1 0 .7 10.91 10.91 0 0 1-3.07 4.3" />
    <path d="M6.61 6.61A10.94 10.94 0 0 0 2.06 11.65a1 1 0 0 0 0 .7C3.49 15.77 7.07 18 12 18c1.67 0 3.17-.26 4.5-.72" />
  </svg>
{:else}
  <svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <rect x="3" y="4" width="18" height="6" rx="1" />
    <rect x="3" y="14" width="18" height="6" rx="1" />
    <path d="M7 7h.01M7 17h.01" />
  </svg>
{/if}
