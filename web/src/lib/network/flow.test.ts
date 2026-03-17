import { describe, expect, it } from 'vitest';
import { canTransition } from '$lib/network/flow';

describe('flow transitions', () => {
  it('allows editing -> test_submitting', () => {
    expect(canTransition('editing', 'test_submitting')).toBe(true);
  });

  it('does not allow save_handoff -> editing', () => {
    expect(canTransition('save_handoff', 'editing')).toBe(false);
  });

  it('allows reconnect recovery path', () => {
    expect(canTransition('test_waiting_api', 'test_polling')).toBe(true);
  });
});
