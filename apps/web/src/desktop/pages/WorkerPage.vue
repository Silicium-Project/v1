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
import WorkerMetrics from '@/desktop/components/worker/WorkerMetrics.vue'
import WorkerTasks from '@/desktop/components/worker/WorkerTasks.vue'
import { currentLinkRecord, type LinkRecord } from '@/desktop/link-record'

// The worker page only works in the desktop application, which runs the node. The browser
// has no node to ask.
const desktop = isDesktop()
const status = ref<NodeStatus | null>(null)
const report = ref<WorkerReport | null>(null)
const busy = ref(false)
const error = ref('')
const info = ref('')
// Liaison à un compte, lue sur cette machine (voir desktop/link-record.ts).
const linked = ref<LinkRecord | null>(null)
const linkChecked = ref(false)
const linkedText = computed(() =>
  linked.value
    ? t('desktop.linkedSince', {
        date: formatDateTime(linked.value.linked_at),
        label: linked.value.label || '—',
      })
    : '',
)
const POLL_MS = 5000
// Les renouvellements de bail ne sont pas une activité : ils restent dans le journal, pas ici.
const recentEvents = computed(() => (report.value?.recent ?? []).filter((event) => event.kind !== 'renew'))
const KINDS = [
  'job_published',
  'claim',
  'accept',
  'release',
  'submit',
  'verify_offer',
  'verifier_accept',
  'attest',
  'finalize',
  'cancel',
] as const

let timer: ReturnType<typeof setInterval> | undefined

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
  } catch (failure) {
    if (failure instanceof DesktopFailure && failure.code === 'unreachable') {
      // The node is still starting: the report comes a few seconds later.
      error.value = t('worker.errors.unreachable')
      return
    }
    error.value = failureText(failure)
  }
}

async function start(): Promise<void> {
  busy.value = true
  error.value = ''
  info.value = ''
  try {
    await nodeStart()
    // A node that exits at once (another process holds its port, a broken config) must not be
    // reported as started: wait a moment, then read the status.
    await new Promise((resolve) => setTimeout(resolve, 1500))
    await refresh()
    if (status.value?.running) {
      info.value = t('worker.started')
    } else {
      error.value = t('worker.errors.start_failed')
    }
  } catch (failure) {
    error.value = failureText(failure)
  } finally {
    busy.value = false
  }
}

async function stop(): Promise<void> {
  busy.value = true
  error.value = ''
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

onMounted(() => {
  void currentLinkRecord().then((record) => {
    linked.value = record
    linkChecked.value = true
  })
  void refresh()
  if (desktop) timer = setInterval(() => void refresh(), POLL_MS)
})
onBeforeUnmount(() => {
  if (timer !== undefined) clearInterval(timer)
})
</script>

<template>
  <section class="stack" aria-labelledby="worker-title">
    <h1 id="worker-title" tabindex="-1">{{ t('worker.title') }}</h1>

    <Notice v-if="!desktop" kind="info" :message="t('worker.webOnly')" />

    <template v-else>
      <template v-if="linkChecked">
        <Notice v-if="linked" kind="success" :message="linkedText" />
        <Notice v-else kind="info" :message="t('desktop.notLinked')" />
      </template>
      <Notice kind="error" :message="error" />
      <Notice kind="success" :message="info" />

      <section v-if="!status?.running" class="card stack" aria-labelledby="activate-title">
        <h2 id="activate-title">{{ t('worker.activateTitle') }}</h2>
        <p>{{ t('worker.activateText') }}</p>
        <div class="actions">
          <button type="button" class="btn btn-primary" :disabled="busy" @click="start">
            {{ t('worker.start') }}
          </button>
          <button type="button" class="btn btn-quiet" :disabled="busy" @click="refresh">
            {{ t('common.refresh') }}
          </button>
        </div>
      </section>

      <template v-else>
        <section class="card stack" aria-labelledby="node-title">
          <div class="worker-head">
            <h2 id="node-title">{{ t('worker.activeTitle') }}</h2>
            <button type="button" class="btn" :disabled="busy" @click="stop">
              {{ t('worker.stop') }}
            </button>
          </div>
          <dl class="dl">
            <dt>{{ t('worker.identity') }}</dt>
            <dd><code>{{ status?.identity ?? '—' }}</code></dd>
            <template v-if="report">
              <dt>{{ t('worker.roles') }}</dt>
              <dd>{{ roleNames.join(', ') || t('worker.none') }}</dd>
            </template>
          </dl>
        </section>

        <template v-if="report">
          <h2>{{ t('worker.dashboard') }}</h2>
          <WorkerMetrics :report="report" />
          <WorkerTasks :report="report" />

          <section class="card stack" aria-labelledby="activity-title">
            <h2 id="activity-title">{{ t('worker.activity') }}</h2>
            <dl class="dl">
              <dt>{{ t('worker.jobsTouched') }}</dt>
              <dd>{{ report.jobs_touched }}</dd>
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
              <ul v-if="recentEvents.length" class="mono-list">
                <li v-for="(event, index) in recentEvents" :key="index">
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
                <li v-for="(count, name) in report.settlements_seen" :key="name">{{ name }} : {{ count }}</li>
              </ul>
            </div>
          </section>
        </template>
      </template>
    </template>
  </section>
</template>
