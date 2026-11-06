// Bridge to the desktop shell. In a browser every call fails with "unavailable": the browser
// never holds a signing key. Only the desktop window exposes the Tauri IPC object.

export type DesktopCode =
  | 'no_key'
  | 'bad_code'
  | 'storage'
  | 'unavailable'
  | 'no_binary'
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

export interface WorkerReport {
  worker: string
  roles: { compute: boolean; verify: boolean; coordinate: boolean }
  jobs_touched: number
  events_by_kind: Record<string, number>
  recent: Array<{ job: string; sequence: number; kind: string; issued_at_ms: number }>
  reputation: number
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
