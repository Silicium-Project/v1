<script setup lang="ts">
import { computed } from 'vue'
import { t } from '@/i18n'
import type { WorkerReport } from '@/platform/desktop'
import { powerPercent } from '@/desktop/worker-view'

const props = defineProps<{ report: WorkerReport }>()

const capacity = computed(() => props.report.capacity)
const power = computed(() => powerPercent(capacity.value.active, capacity.value.max_parallel))
// Réputation réseau : score on-chain sur 10 000, affiché en pourcentage (10 000 = 100 %).
const networkPct = computed(() => {
  const standing = props.report.standing_onchain
  return standing === null ? null : Math.min(100, Math.max(0, Math.round(standing / 100)))
})
// Fiabilité locale : score entre 0 et 1, affiché en pourcentage.
const localPct = computed(() => Math.min(100, Math.max(0, Math.round(props.report.reputation * 100))))
// 1 SOL = 1 000 000 000 lamports. Les gains viennent de la chaîne, jamais d'un calcul local.
const sol = (lamports: number) => `${(lamports / 1_000_000_000).toFixed(6)} SOL`
</script>

<template>
  <div class="worker-metrics">
    <section class="card stack" aria-labelledby="metric-reputation">
      <h2 id="metric-reputation">{{ t('worker.reputationTitle') }}</h2>
      <p class="metric-value">{{ networkPct === null ? '—' : `${networkPct} %` }}</p>
      <meter class="worker-meter" :value="networkPct ?? 0" min="0" max="100">{{ networkPct ?? 0 }} %</meter>
      <p class="muted">{{ networkPct === null ? t('worker.standingNone') : t('worker.standingHelp') }}</p>

      <h3>{{ t('worker.localTitle') }} : {{ localPct }} %</h3>
      <meter class="worker-meter" :value="localPct" min="0" max="100">{{ localPct }} %</meter>
      <p class="muted">{{ t('worker.reputationHelp') }}</p>
      <p class="muted">
        {{
          t('worker.evidence', {
            successes: report.reputation_evidence.successes.toFixed(1),
            failures: report.reputation_evidence.failures.toFixed(1),
          })
        }}
      </p>
    </section>

    <section class="card stack" aria-labelledby="metric-power">
      <h2 id="metric-power">{{ t('worker.powerTitle') }}</h2>
      <p class="metric-value">{{ power }} %</p>
      <meter class="worker-meter" :value="power" min="0" max="100">{{ power }} %</meter>
      <p>
        {{ t('worker.powerText', { active: capacity.active, allowed: capacity.max_parallel }) }}
      </p>
      <p class="muted">{{ t('worker.powerCores', { cores: capacity.cpu_cores }) }}</p>
      <p>{{ t('worker.powerLoad', { node: capacity.node_cpu_pct, machine: capacity.machine_cpu_pct }) }}</p>
      <p class="muted">{{ t('worker.powerLoadHelp') }}</p>
    </section>

    <section class="card stack" aria-labelledby="metric-gains">
      <h2 id="metric-gains">{{ t('worker.gainsTitle') }}</h2>
      <p class="metric-value">{{ sol(report.earnings.lamports) }}</p>
      <p class="muted">{{ t('worker.gainsText') }}</p>
      <p class="muted">
        {{
          t('worker.gainsDetail', {
            count: report.earnings.fragments_settled,
            worker: report.earnings.worker_lamports,
            verifier: report.earnings.verifier_lamports,
          })
        }}
      </p>
      <p>{{ t('worker.finalizedCount', { count: report.history_totals.finalized }) }}</p>
    </section>
  </div>
</template>
