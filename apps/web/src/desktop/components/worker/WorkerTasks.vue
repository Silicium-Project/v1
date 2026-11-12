<script setup lang="ts">
import { t } from '@/i18n'
import { formatDateTime } from '@/i18n/format'
import type { WorkerReport } from '@/platform/desktop'
import { stageKey } from '@/desktop/worker-view'

defineProps<{ report: WorkerReport }>()

function isoOf(ms: number): string {
  return new Date(ms).toISOString()
}
</script>

<template>
  <div class="stack">
    <section class="card stack" aria-labelledby="tasks-running">
      <h2 id="tasks-running">{{ t('worker.tasksTitle') }}</h2>
      <p v-if="report.in_progress.length === 0" class="muted">{{ t('worker.noTasks') }}</p>
      <div v-else class="worker-table-wrap">
        <table class="worker-table">
          <thead>
            <tr>
              <th scope="col">{{ t('worker.colJob') }}</th>
              <th scope="col">{{ t('worker.colFragment') }}</th>
              <th scope="col">{{ t('worker.colStage') }}</th>
              <th scope="col">{{ t('worker.colSince') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="task in report.in_progress" :key="`${task.job}-${task.fragment}-${task.attempt}`">
              <td><code>{{ task.job.slice(0, 12) }}</code></td>
              <td>{{ task.fragment }}</td>
              <td>{{ t(`worker.stage.${stageKey(task)}`) }}</td>
              <td>
                <time :datetime="isoOf(task.started_at_ms)">{{ formatDateTime(isoOf(task.started_at_ms)) }}</time>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <section class="card stack" aria-labelledby="tasks-history">
      <h2 id="tasks-history">{{ t('worker.historyTitle') }}</h2>
      <p v-if="report.history.length === 0" class="muted">{{ t('worker.noHistory') }}</p>
      <div v-else class="worker-table-wrap worker-scroll" tabindex="0" :aria-label="t('worker.historyTitle')">
        <table class="worker-table">
          <thead>
            <tr>
              <th scope="col">{{ t('worker.colJob') }}</th>
              <th scope="col">{{ t('worker.colFragment') }}</th>
              <th scope="col">{{ t('worker.colOutcome') }}</th>
              <th scope="col">{{ t('worker.colAt') }}</th>
              <th scope="col">{{ t('worker.colApprovals') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="entry in report.history" :key="`${entry.job}-${entry.fragment}-${entry.attempt}`">
              <td><code>{{ entry.job.slice(0, 12) }}</code></td>
              <td>{{ entry.fragment }}</td>
              <td>{{ t(`worker.outcome.${entry.outcome}`) }}</td>
              <td>
                <time :datetime="isoOf(entry.at_ms)">{{ formatDateTime(isoOf(entry.at_ms)) }}</time>
              </td>
              <td>{{ entry.approvals }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
