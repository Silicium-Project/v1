<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { t } from '@/i18n'
import { formatDateTime } from '@/i18n/format'
import {
  DesktopFailure,
  isDesktop,
  nodeStart,
  nodeStatus,
  nodeStop,
  workerReport,
  type NodeStatus,
  type WorkerReport,
} from '@/platform/desktop'
import Notice from '@/components/Notice.vue'

// The worker page only works in the desktop application, which runs the node. The browser
// has no node to ask.
const desktop = isDesktop()
const status = ref<NodeStatus | null>(null)
const report = ref<WorkerReport | null>(null)
const busy = ref(false)
const error = ref('')
const info = ref('')
const KINDS = [
  'job_published',
  'claim',
  'accept',
  'renew',
  'release',
  'submit',
  'verify_offer',
  'verifier_accept',
  'attest',
  'finalize',
  'cancel',
] as const

let timer: ReturnType<typeof setInterval> | undefined
let attempts = 0

const kindRows = computed(() =>
  KINDS.filter((kind) => (report.value?.events_by_kind[kind] ?? 0) > 0).map((kind) => ({
    kind,
    label: t(`worker.kinds.${kind}`),
    count: report.value?.events_by_kind[kind] ?? 0,
  })),
)

const roleNames = computed(() => {
  const roles = report.value?.roles
  if (!roles) return []
  const names: string[] = []
  if (roles.compute) names.push(t('worker.roleCompute'))
  if (roles.verify) names.push(t('worker.roleVerify'))
  if (roles.coordinate) names.push(t('worker.roleCoordinate'))
  return names
})

function failureText(failure: unknown): string {
  return failure instanceof DesktopFailure ? t(`worker.errors.${failure.code}`) : t('errors.generic')
}

async function refresh(): Promise<void> {
  if (!desktop) return
  try {
    status.value = await nodeStatus()
    if (!status.value.running) {
      report.value = null
      return
    }
    report.value = await workerReport()
    error.value = ''
    stopPolling()
  } catch (failure) {
    if (failure instanceof DesktopFailure && failure.code === 'unreachable') {
      // The node is still starting: the report comes a few seconds later.
      error.value = t('worker.errors.unreachable')
      return
    }
    error.value = failureText(failure)
  }
}

function startPolling(): void {
  stopPolling()
  attempts = 0
  timer = setInterval(() => {
    attempts++
    void refresh()
    if (attempts >= 15) stopPolling()
  }, 2000)
}

function stopPolling(): void {
  if (timer !== undefined) clearInterval(timer)
  timer = undefined
}

async function start(): Promise<void> {
  busy.value = true
  error.value = ''
  info.value = ''
  try {
    await nodeStart()
    info.value = t('worker.started')
    await refresh()
    startPolling()
  } catch (failure) {
    error.value = failureText(failure)
  } finally {
    busy.value = false
  }
}

async function stop(): Promise<void> {
  busy.value = true
  error.value = ''
  stopPolling()
  try {
    await nodeStop()
    info.value = t('worker.stopped')
    await refresh()
  } catch (failure) {
    error.value = failureText(failure)
  } finally {
    busy.value = false
  }
}

onMounted(refresh)
onBeforeUnmount(stopPolling)
</script>

<template>
  <section class="stack" aria-labelledby="worker-title">
    <h1 id="worker-title" tabindex="-1">{{ t('worker.title') }}</h1>

    <Notice v-if="!desktop" kind="info" :message="t('worker.webOnly')" />

    <template v-else>
      <Notice kind="error" :message="error" />
      <Notice kind="success" :message="info" />

      <section class="card stack" aria-labelledby="node-title">
        <h2 id="node-title">
          {{ status?.running ? t('worker.statusRunning') : t('worker.statusStopped') }}
        </h2>
        <dl class="dl">
          <dt>{{ t('worker.identity') }}</dt>
          <dd><code>{{ status?.identity ?? '—' }}</code></dd>
          <template v-if="report">
            <dt>{{ t('worker.roles') }}</dt>
            <dd>{{ roleNames.join(', ') || t('worker.none') }}</dd>
          </template>
        </dl>
        <div class="actions">
          <button type="button" class="btn btn-primary" :disabled="busy || status?.running" @click="start">
            {{ t('worker.start') }}
          </button>
          <button type="button" class="btn" :disabled="busy || !status?.running" @click="stop">
            {{ t('worker.stop') }}
          </button>
          <button type="button" class="btn btn-quiet" :disabled="busy" @click="refresh">
            {{ t('common.refresh') }}
          </button>
        </div>
      </section>

      <template v-if="report">
        <section class="card stack" aria-labelledby="activity-title">
          <h2 id="activity-title">{{ t('worker.activity') }}</h2>
          <dl class="dl">
            <dt>{{ t('worker.jobsTouched') }}</dt>
            <dd>{{ report.jobs_touched }}</dd>
            <dt>{{ t('worker.reputation') }}</dt>
            <dd>{{ report.reputation.toFixed(2) }}</dd>
          </dl>
          <div>
            <h3>{{ t('worker.byKind') }}</h3>
            <ul v-if="kindRows.length">
              <li v-for="row in kindRows" :key="row.kind">{{ row.label }} : {{ row.count }}</li>
            </ul>
            <p v-else class="muted">{{ t('worker.noActivity') }}</p>
          </div>
          <div>
            <h3>{{ t('worker.recent') }}</h3>
            <ul v-if="report.recent.length" class="mono-list">
              <li v-for="(event, index) in report.recent" :key="index">
                <time :datetime="new Date(event.issued_at_ms).toISOString()">
                  {{ formatDateTime(new Date(event.issued_at_ms).toISOString()) }}
                </time>
                — {{ t(`worker.kinds.${event.kind}`) }} — <code>{{ event.job.slice(0, 12) }}</code>
              </li>
            </ul>
          </div>
          <div>
            <h3>{{ t('worker.settlements') }}</h3>
            <p v-if="Object.keys(report.settlements_seen).length === 0" class="muted">{{ t('worker.none') }}</p>
            <ul v-else>
              <li v-for="(count, status) in report.settlements_seen" :key="status">{{ status }} : {{ count }}</li>
            </ul>
          </div>
          <Notice kind="info" :message="t('worker.noPayments')" />
        </section>
      </template>
    </template>
  </section>
</template>
