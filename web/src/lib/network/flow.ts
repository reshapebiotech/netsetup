export type FlowState =
  | 'editing'
  | 'test_submitting'
  | 'test_waiting_api'
  | 'test_polling'
  | 'test_success'
  | 'test_failed'
  | 'save_submitting'
  | 'save_handoff';

const allowed: Record<FlowState, FlowState[]> = {
  editing: ['test_submitting'],
  test_submitting: ['test_waiting_api', 'test_polling', 'test_failed'],
  test_waiting_api: ['test_polling', 'test_failed'],
  test_polling: ['test_success', 'test_failed', 'test_waiting_api'],
  test_success: ['save_submitting', 'editing'],
  test_failed: ['editing'],
  save_submitting: ['save_handoff'],
  save_handoff: []
};

export function canTransition(from: FlowState, to: FlowState) {
  return allowed[from].includes(to);
}
