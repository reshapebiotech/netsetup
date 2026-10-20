import type { NetworkDraft } from './types.js';

export function draftUrl(draft: NetworkDraft): string {
  const base = `/setup/new/${draft.transport}`;
  const id = encodeURIComponent(draft.test_id);
  switch (draft.state.status) {
    case 'Testing': return `${base}/testing/${id}`;
    case 'Saving': return `${base}/save-handoff/${id}`;
    default: return `${base}/result/${id}`;
  }
}

export function draftDescription(draft: NetworkDraft): string {
  switch (draft.state.status) {
    case 'Testing': return 'Connection test in progress';
    case 'Connected': return 'Test complete · Not saved yet';
    case 'Saving': return 'Saving connection · Awaiting confirmation';
    case 'Failed': return 'Not saved · Edit settings and try again';
    case 'Saved': return 'Connection saved';
  }
}
