<script lang="ts">
  import Icon from '#lib/components/ui/icon.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import { resultSummary, groupLevel, serviceLevel } from '#lib/network/results.js';
  import type { ConnectivityReport } from '#lib/network/types.js';

  let { report, compact = false }: { report: ConnectivityReport; compact?: boolean } = $props();
  const level = $derived(serviceLevel(report));
  const positive = $derived(level === 'full' || (level === null && report.internet_verified));
  const labels = { full: 'Full connectivity', partial: 'Partial connectivity', none: 'No connectivity' };
  const colors = { full: 'text-success', partial: 'text-warning', none: 'text-destructive' };
</script>

<div class={compact ? 'compact space-y-3' : 'space-y-5'}>
  <Alert tone={positive ? 'success' : level === 'none' ? 'error' : 'warning'}>
    <span class="flex items-start gap-2"><Icon name={positive ? 'check' : 'alert'} className="mt-0.5 h-4 w-4" />{resultSummary(report)}</span>
  </Alert>
  <dl class="report-checks">
    {#if !compact}<div><dt>Network connected</dt><dd>{report.interface} <span class="text-muted-foreground">·</span> {report.address}</dd></div>{/if}
    <div><dt>Gateway reachable</dt><dd class={report.gateway_reachable ? 'text-success' : 'text-muted-foreground'}>{report.gateway_reachable === null ? 'No gateway configured' : report.gateway_reachable ? 'Verified' : 'Could not verify'}</dd></div>
    <div><dt>Internet access</dt><dd class={report.internet_verified ? 'text-success' : 'text-warning'}>{report.internet_verified ? 'Verified' : 'Could not verify'}</dd></div>
  </dl>
  {#each report.endpoint_groups as group (group.name)}
    {@const state = groupLevel(group)}
    <section class="service-group" aria-label={group.name}>
      <header class="group-heading">
        <div>
          <h3 class="text-sm font-semibold">{group.name}</h3>
          <p class="mt-1 text-xs text-muted-foreground">{group.required ? (compact ? 'Required' : 'Required for normal operation') : (compact ? 'Additional' : 'Additional services')} · {group.endpoints.filter(endpoint => endpoint.reachable).length}/{group.endpoints.length} reachable</p>
        </div>
        <span class={`group-status ${colors[state]}`}>
          <Icon name={state === 'full' ? 'check' : 'alert'} className="h-4 w-4" />
          {labels[state]}
        </span>
      </header>
      <ul class="endpoint-list">
        {#each group.endpoints as endpoint (endpoint.url)}
          <li>
            <Icon name={endpoint.reachable ? 'check' : 'alert'} className={`mt-0.5 h-4 w-4 ${endpoint.reachable ? 'text-success' : 'text-destructive'}`} />
            {#if compact}
              <details class="endpoint-details min-w-0 flex-1">
                <summary>
                  <span class="min-w-0 flex-1 text-sm font-medium">{endpoint.name}</span>
                  <span class={`text-xs ${endpoint.reachable ? 'text-success' : 'text-destructive'}`}>{endpoint.reachable ? 'Reachable' : 'Could not reach'}</span>
                  <span class="disclosure text-muted-foreground" aria-hidden="true"></span>
                </summary>
                {#if endpoint.description}<p class="mt-2 text-xs leading-relaxed text-muted-foreground">{endpoint.description}</p>{/if}
                <p class="mt-1 break-all font-mono text-xs leading-relaxed text-muted-foreground">{endpoint.url}</p>
              </details>
            {:else}
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
                <span class="text-sm font-medium">{endpoint.name}</span>
                <span class={`text-xs ${endpoint.reachable ? 'text-success' : 'text-destructive'}`}>{endpoint.reachable ? 'Reachable' : 'Could not reach'}</span>
              </div>
              {#if endpoint.description}<p class="mt-1 text-xs leading-relaxed text-muted-foreground">{endpoint.description}</p>{/if}
              <p class="mt-1 break-all font-mono text-xs leading-relaxed text-muted-foreground">{endpoint.url}</p>
            </div>
            {/if}
          </li>
        {/each}
      </ul>
    </section>
  {/each}
  {#if report.endpoint_groups.length > 0}
    <p class="text-xs leading-relaxed text-muted-foreground">{compact ? 'Reachability checks do not verify sign-in or permissions.' : 'Checks confirm network access, including responses that require authentication. They do not verify sign-in or permission to use the service.'}</p>
  {/if}
</div>

<style>
  .service-group { overflow: hidden; border: 1px solid hsl(var(--border)); border-radius: 0.75rem; }
  .group-heading { display: flex; justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 0.75rem; padding: 1rem; background: hsl(var(--muted) / 0.25); }
  .group-status { display: inline-flex; align-items: center; gap: 0.375rem; font-size: 0.75rem; line-height: 1.5; }
  .endpoint-list li { display: flex; align-items: flex-start; gap: 0.75rem; padding: 1rem; border-top: 1px solid hsl(var(--border)); }
  .report-checks { display: grid; font-size: 0.875rem; }
  .report-checks > div { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 1rem; padding-block: 0.75rem; }
  .report-checks > div + div { border-top: 1px solid hsl(var(--border) / 0.6); }
  dt { color: hsl(var(--muted-foreground)); }
  dd { overflow-wrap: anywhere; text-align: right; }
  .compact .group-heading { padding: 0.75rem; gap: 0.5rem; }
  .compact .endpoint-list li { padding: 0.625rem 0.75rem; gap: 0.5rem; }
  .compact .report-checks > div { padding-block: 0.375rem; }
  .endpoint-details summary { display: flex; align-items: baseline; gap: 0.5rem; cursor: pointer; list-style: none; }
  .endpoint-details summary::-webkit-details-marker { display: none; }
  .endpoint-details summary:hover > span:first-child { text-decoration: underline; }
  .disclosure::after { content: '+'; }
  .endpoint-details[open] .disclosure::after { content: '−'; }
  @media (max-width: 360px) {
    .report-checks > div { gap: 0.5rem; font-size: 0.8125rem; }
  }
</style>
