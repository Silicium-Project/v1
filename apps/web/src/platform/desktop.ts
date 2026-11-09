// Bridge to the desktop shell. In a browser every call fails with "unavailable": the browser
// never holds a signing key. Only the desktop window exposes the Tauri IPC object.

export type DesktopCode =
  | 'no_key'
  | 'bad_code'
  | 'storage'
  | 'unavailable'
  | 'no_binary'
  | 'port_busy'
  | 'start_failed'
  | 'not_running'
  | 'unreachable'
  | 'unauthorized'
  | 'bad_answer'

export class DesktopFailure extends Error {
  constructor(readonly code: DesktopCode) {
    super(code)
    this.name = 'DesktopFailure'
  }
}

interface TauriBridge {
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>
}

const KNOWN: readonly DesktopCode[] = [
  'no_key',
  'bad_code',
  'storage',
  'no_binary',
  'port_busy',
  'start_failed',
  'not_running',
  'unreachable',
  'unauthorized',
  'bad_answer',
]

export function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop()) throw new DesktopFailure('unavailable')
  const bridge = (window as unknown as { __TAURI_INTERNALS__: TauriBridge }).__TAURI_INTERNALS__
  try {
    return (await bridge.invoke(command, args)) as T
  } catch (failure) {
    const code = typeof failure === 'string' && (KNOWN as readonly string[]).includes(failure) ? failure : 'unavailable'
    throw new DesktopFailure(code as DesktopCode)
  }
}

/** Public key of this machine, or null if it has none yet. */
export function machineKey(): Promise<string | null> {
  return call<string | null>('machine_key')
}

/** Creates the machine key (never replaces an existing one) and returns its public key. */
export function createMachineKey(): Promise<string> {
  return call<string>('create_machine_key')
}

/** Signs a link code. Returns the public key and the signature, both hex. */
export function signLinkCode(code: string): Promise<{ public_key: string; signature: string }> {
  return call('sign_link_code', { code })
}

export interface NodeStatus {
  running: boolean
  identity: string | null
}

/** A task this worker holds: its last attempt runs, or waits for verification. */
export interface WorkerTask {
  job: string
  fragment: number
  attempt: number
  status: string
  submitted: boolean
  started_at_ms: number
  lease_expires_at_ms: number
}

/** How one of this worker's attempts ended. `finalized` is the only one that counts as done. */
export interface WorkerHistoryEntry {
  job: string
  fragment: number
  attempt: number
  outcome: 'finalized' | 'superseded' | 'released' | 'rejected' | 'expired'
  at_ms: number
  approvals: number
}

export interface WorkerReport {
  worker: string
  roles: { compute: boolean; verify: boolean }
  jobs_touched: number
  events_by_kind: Record<string, number>
  recent: Array<{ job: string; sequence: number; kind: string; issued_at_ms: number }>
  /** Network reputation of this machine as a verifier, read on-chain. `null` until the account exists. */
  /** Lamports confirmed on-chain for this machine: as worker and as verifier. */
  earnings: { worker_lamports: number; verifier_lamports: number; lamports: number; fragments_settled: number; checked_at_ms: number }
  standing_onchain: number | null
  /** Posterior mean of a Beta(1,1) prior: 0.5 for a newcomer. */
  reputation: number
  /** Decayed, weighted counts behind `reputation`. */
  reputation_evidence: { successes: number; failures: number }
  /** `active`: tasks and verifications running now. `max_parallel`: what the reputation allows. */
  capacity: { active: number; max_parallel: number; cpu_cores: number }
  in_progress: WorkerTask[]
  history: WorkerHistoryEntry[]
  history_totals: { finalized: number; other: number }
  settlements_seen: Record<string, number>
  payments: boolean
}

/** Whether the node process runs, and the identity it uses. */
export function nodeStatus(): Promise<NodeStatus> {
  return call<NodeStatus>('node_status')
}

export function nodeStart(): Promise<void> {
  return call<void>('node_start')
}

export function nodeStop(): Promise<void> {
  return call<void>('node_stop')
}

/** The node's own worker report (GET /v1/worker on its loopback control API). */
export function workerReport(): Promise<WorkerReport> {
  return call<WorkerReport>('worker_report')
}

/** A link code that opened the application and waits for the window, if any. Taking it clears it. */
export function takePendingLink(): Promise<string | null> {
  return call<string | null>('take_pending_link')
}