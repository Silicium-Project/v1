<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { t } from '@/i18n'
import { formatDateTime } from '@/i18n/format'
import { getJob, listJobs, type JobSummary } from '@/api/endpoints'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { currentToken, endSession } from '@/state/session'
import { normalizeState } from '@/status'
import { WORKLOADS } from '@/workloads/generated'
import EmptyState from '@/components/EmptyState.vue'
import Notice from '@/components/Notice.vue'
import StatusBadge from '@/components/StatusBadge.vue'

interface Info {
  state: string
  workload: string
}

type SortKey = 'date' | 'status' | 'name' | 'type'

const router = useRouter()
const jobs = ref<JobSummary[]>([])
const loading = ref(true)
const error = ref('')
/** État normalisé et type de chaque calcul, lus sur le nœud (job id → infos). */
const infos = ref<Record<string, Info>>({})
const sortKey = ref<SortKey>('date')
const descending = ref(true)
/** Ordre d'affichage des états pour le tri par statut. */
const STATE_ORDER = ['running', 'finalized', 'cancelled', 'expired', 'not_on_node', 'unknown']

function infoOf(job: JobSummary): Info {
  return infos.value[job.id] ?? { state: 'unknown', workload: '' }
}

/** Nom du type de calcul, tel qu'affiché dans le formulaire ; l'identifiant brut si inconnu. */
function typeLabel(workload: string): string {
  const spec = WORKLOADS.find((item) => item.id === workload)
  return spec ? t(spec.labelKey) : workload || t('jobs.unknownType')
}

function compare(a: JobSummary, b: JobSummary): number {
  switch (sortKey.value) {
    case 'status': {
      const rank = (job: JobSummary) => {
        const index = STATE_ORDER.indexOf(infoOf(job).state)
        return index === -1 ? STATE_ORDER.length : index
      }
      return rank(a) - rank(b)
    }
    case 'name':
      return (a.label || '').localeCompare(b.label || '', 'fr')
    case 'type':
      return typeLabel(infoOf(a).workload).localeCompare(typeLabel(infoOf(b).workload), 'fr')
    default:
      return a.created_at.localeCompare(b.created_at)
  }
}

/** Liste triée : la clé choisie, puis la date la plus récente d'abord en cas d'égalité. */
const sorted = computed(() => {
  const direction = descending.value ? -1 : 1
  return [...jobs.value].sort((a, b) => direction * compare(a, b) || b.created_at.localeCompare(a.created_at))
})

async function loadInfos(token: string, list: JobSummary[]): Promise<void> {
  const answers = await Promise.all(
    list.map(async (job): Promise<[string, Info]> => {
      try {
        const { status } = await getJob(token, job.id)
        return [job.id, { state: normalizeState(status.state), workload: status.workload ?? '' }]
      } catch {
        return [job.id, { state: 'unknown', workload: '' }]
      }
    }),
  )
  infos.value = Object.fromEntries(answers)
}

async function load(): Promise<void> {
  const token = currentToken()
  if (!token) return
  loading.value = true
  error.value = ''
  try {
    jobs.value = (await listJobs(token)).jobs
    void loadInfos(token, jobs.value)
  } catch (failure) {
    if (isSessionEnded(failure)) {
      await endSession(router)
      return
    }
    error.value = t(errorMessageKey(failure))
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

<template>
  <section class="stack" aria-labelledby="jobs-title">
    <div class="toolbar">
      <h1 id="jobs-title">{{ t('jobs.title') }}</h1>
      <button type="button" class="btn" :disabled="loading" @click="load">{{ t('common.refresh') }}</button>
    </div>

    <p v-if="loading" role="status" class="muted">{{ t('common.loading') }}</p>
    <Notice kind="error" :message="error" />

    <EmptyState v-if="!loading && !error && jobs.length === 0" :title="t('jobs.empty')">
      <p>{{ t('jobs.emptyText') }}</p>
      <RouterLink class="btn btn-primary" to="/soumettre">{{ t('jobs.submitLink') }}</RouterLink>
    </EmptyState>

    <template v-if="jobs.length > 0">
      <p class="muted">{{ t('jobs.count', { count: jobs.length }) }}</p>
      <div class="jobs-sort">
        <label for="jobs-sort">{{ t('jobs.sortBy') }}</label>
        <select id="jobs-sort" v-model="sortKey" class="input">
          <option value="date">{{ t('jobs.sort.date') }}</option>
          <option value="status">{{ t('jobs.sort.status') }}</option>
          <option value="name">{{ t('jobs.sort.name') }}</option>
          <option value="type">{{ t('jobs.sort.type') }}</option>
        </select>
        <button type="button" class="btn" :aria-pressed="descending" @click="descending = !descending">
          {{ descending ? t('jobs.order.desc') : t('jobs.order.asc') }}
        </button>
      </div>
      <ul class="list jobs-scroll" tabindex="0" :aria-label="t('jobs.title')">
        <li v-for="job in sorted" :key="job.id">
          <RouterLink :to="{ name: 'job', params: { id: job.id } }" class="list__link">
            <span class="list__title">{{ job.label || t('jobs.noLabel') }}</span>
            <span class="list__meta">
              <span class="muted">{{ typeLabel(infoOf(job).workload) }}</span>
              <StatusBadge :state="infoOf(job).state" />
              <time :datetime="job.created_at">{{ formatDateTime(job.created_at) }}</time>
            </span>
          </RouterLink>
        </li>
      </ul>
    </template>
  </section>
</template>

<style scoped>
.jobs-sort {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}
.jobs-sort select {
  width: auto;
}
.jobs-scroll {
  max-height: 36rem;
  overflow-y: auto;
  padding-right: 0.5rem;
}
</style>
