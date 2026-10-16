<script lang="ts">
  import Alert from '#lib/components/ui/alert.svelte';
  import { resultSummary } from '#lib/network/results.js';
  import type { ConnectivityReport } from '#lib/network/types.js';

  let { report }: { report: ConnectivityReport } = $props();
</script>

<div class="space-y-4">
  <Alert tone="info">{resultSummary(report)}</Alert>
  <dl class="grid grid-cols-2 gap-x-4 gap-y-2 text-sm [&>dd]:min-w-0 [&>dd]:break-words">
    <dt>Network connected</dt><dd>{report.interface} · {report.address}</dd>
    <dt>Gateway reachable</dt>
    <dd>{report.gateway_reachable === null ? 'No gateway configured' : report.gateway_reachable ? 'Verified' : 'Could not verify'}</dd>
    <dt>Internet access</dt><dd>{report.internet_verified ? 'Verified' : 'Could not verify'}</dd>
    <dt>Reshape service</dt><dd>{report.reshape_reachable ? 'Reachable' : 'Could not reach'}</dd>
  </dl>
</div>
