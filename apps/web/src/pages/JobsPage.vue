<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { t } from '@/i18n'
import { formatDateTime } from '@/i18n/format'
import { listJobs, type JobSummary } from '@/api/endpoints'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { currentToken, endSession } from '@/state/session'
import EmptyState from '@/components/EmptyState.vue'
import Notice from '@/components/Notice.vue'

const router = useRouter()
const jobs = ref<JobSummary[]>([])
const loading = ref(true)
const error = ref('')

async function load(): Promise<void> {
  const token = currentToken()
  if (!token) return
  loading.value = true
  error.value = ''
  try {
    jobs.value = (await listJobs(token)).jobs
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
      <ul class="list">
        <li v-for="job in jobs" :key="job.id">
          <RouterLink :to="{ name: 'job', params: { id: job.id } }" class="list__link">
            <span class="list__title">{{ job.label || t('jobs.noLabel') }}</span>
            <span class="list__meta">
              <time :datetime="job.created_at">{{ formatDateTime(job.created_at) }}</time>
            </span>
          </RouterLink>
        </li>
      </ul>
    </template>
  </section>
</template>
