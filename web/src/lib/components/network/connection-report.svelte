<script lang="ts">
  import Icon from '#lib/components/ui/icon.svelte';
  import Alert from '#lib/components/ui/alert.svelte';
  import { resultSummary } from '#lib/network/results.js';
  import type { ConnectivityReport } from '#lib/network/types.js';

  let { report }: { report: ConnectivityReport } = $props();
</script>

<div class="space-y-5">
  <Alert tone={report.reshape_reachable ? 'success' : 'warning'}>
    <span class="flex items-start gap-2"><Icon name={report.reshape_reachable ? 'check' : 'alert'} className="mt-0.5 h-4 w-4" />{resultSummary(report)}</span>
  </Alert>
  <dl class="report-checks">
    <div><dt>Network connected</dt><dd>{report.interface} <span class="text-muted-foreground">·</span> {report.address}</dd></div>
    <div><dt>Gateway reachable</dt><dd class={report.gateway_reachable ? 'text-success' : 'text-muted-foreground'}>{report.gateway_reachable === null ? 'No gateway configured' : report.gateway_reachable ? 'Verified' : 'Could not verify'}</dd></div>
    <div><dt>Internet access</dt><dd class={report.internet_verified ? 'text-success' : 'text-warning'}>{report.internet_verified ? 'Verified' : 'Could not verify'}</dd></div>
    <div><dt>Reshape service</dt><dd class={report.reshape_reachable ? 'text-success' : 'text-warning'}>{report.reshape_reachable ? 'Reachable' : 'Could not reach'}</dd></div>
  </dl>
</div>

<style>
  .report-checks { display: grid; font-size: 0.875rem; }
  .report-checks > div { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 1rem; padding-block: 0.75rem; }
  .report-checks > div + div { border-top: 1px solid hsl(var(--border) / 0.6); }
  dt { color: hsl(var(--muted-foreground)); }
  dd { overflow-wrap: anywhere; text-align: right; }
  @media (max-width: 360px) {
    .report-checks > div { gap: 0.5rem; font-size: 0.8125rem; }
  }
</style>
