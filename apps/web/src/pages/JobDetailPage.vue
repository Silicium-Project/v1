<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { t } from '@/i18n'
import { formatDateTime, formatTime } from '@/i18n/format'
import { getJob, type JobStatus, type JobSummary } from '@/api/endpoints'
import { errorCodeOf, errorMessageKey, isSessionEnded } from '@/api/errors'
import { currentToken, endSession } from '@/state/session'
import { normalizeState, stateDescriptionKey } from '@/status'
import Notice from '@/components/Notice.vue'
import StatusBadge from '@/components/StatusBadge.vue'

const route = useRoute()
const router = useRouter()
const jobId = computed(() => String(route.params.id ?? ''))

const job = ref<JobSummary | null>(null)
const status = ref<JobStatus | null>(null)
const loading = ref(true)
const error = ref('')
const notFound = ref(false)
const updatedAt = ref('')

async function load(): Promise<void> {
  const token = currentToken()
  if (!token) return
  loading.value = true
  error.value = ''
  notFound.value = false
  try {
    const answer = await getJob(token, jobId.value)
    job.value = answer.job
    status.value = answer.status
    updatedAt.value = new Date().toISOString()
  } catch (failure) {
    if (isSessionEnded(failure)) {
      await endSession(router)
      return
    }
    if (errorCodeOf(failure) === 'not_found') {
      notFound.value = true
      job.value = null
    } else {
      error.value = t(errorMessageKey(failure))
    }
  } finally {
    loading.value = false
  }
}

const state = computed(() => normalizeState(status.value?.state ?? 'unknown'))
const taskEntries = computed(() => Object.entries(status.value?.tasks ?? {}))

onMounted(load)
watch(jobId, load)
</script>

<template>
  <section class="stack" aria-labelledby="job-title">
    <p>
      <RouterLink to="/calculs">
        <span aria-hidden="true">←</span>&nbsp;{{ t('common.back') }}
      </RouterLink>
    </p>

    <p v-if="loading" role="status" class="muted">{{ t('common.loading') }}</p>
    <Notice kind="error" :message="error" />
    <Notice kind="info" :message="notFound ? t('job.notFound') : ''" />

    <template v-if="job && status">
      <h1 id="job-title" tabindex="-1">{{ job.label || t('jobs.noLabel') }}</h1>

      <div class="card stack">
        <dl class="dl">
          <dt>{{ t('job.id') }}</dt>
          <dd><code>{{ job.id }}</code></dd>
          <dt>{{ t('job.createdAt') }}</dt>
          <dd><time :datetime="job.created_at">{{ formatDateTime(job.created_at) }}</time></dd>
          <dt>{{ t('job.status') }}</dt>
          <dd>
            <StatusBadge :state="state" />
            <p class="muted">{{ t(stateDescriptionKey(state)) }}</p>
          </dd>
        </dl>

        <Notice v-if="state === 'finalized'" kind="info" :message="t('job.finalizedNote')" />
        <Notice v-if="state === 'unknown'" kind="info" :message="t('job.unknownNote')" />

        <div>
          <h2>{{ t('job.tasks') }}</h2>
          <ul v-if="taskEntries.length > 0">
            <li v-for="[name, count] in taskEntries" :key="name">{{ name }} : {{ count }}</li>
          </ul>
          <p v-else class="muted">{{ t('job.noTasks') }}</p>
        </div>
      </div>

      <div class="actions">
        <button type="button" class="btn" :disabled="loading" @click="load">{{ t('common.refresh') }}</button>
        <p role="status" class="muted">{{ t('job.refreshed', { time: formatTime(updatedAt) }) }}</p>
      </div>
    </template>
  </section>
</template>
