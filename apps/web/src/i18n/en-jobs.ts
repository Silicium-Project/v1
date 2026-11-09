// Cancellation of a job by its requester (see JobDetailPage.vue). Merged into `job` of en.ts.
export const enJobCancel = {
  cancel: 'Cancel the job',
  cancelNote:
    'The network stops working on this job. The payment is refunded only at the job expiry plus 24 h (escrow rule).',
  cancelAsk: 'Confirm the cancellation with your wallet?',
  cancelConfirm: 'Cancel with the wallet',
  cancelKeep: 'Keep the job',
  cancelFailed: 'The cancellation failed. Check that the connected wallet is the requester’s.',
}
