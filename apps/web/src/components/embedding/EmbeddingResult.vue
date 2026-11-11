<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { t } from '@/i18n'
import { jobInput, jobResult } from '@/api/endpoints'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { currentToken, endSession } from '@/state/session'
import Notice from '@/components/Notice.vue'
import { QUERY_PREFIX, rank, type Hit } from '@/search/semantic'
import {
  decodeVectors,
  passageRows,
  passagesOf,
  toCsv,
  toJson,
  type PassageRow,
} from '@/workloads/embedding-result'

const props = defineProps<{ jobId: string }>()
const router = useRouter()

const rows = ref<PassageRow[]>([])
const loading = ref(true)
const error = ref('')
const question = ref('')
const searching = ref(false)
const percent = ref<number | null>(null)
const searchError = ref('')
const hits = ref<Hit[]>([])
const answered = ref(false)

async function load(): Promise<void> {
  const token = currentToken()
  if (!token) return
  loading.value = true
  error.value = ''
  try {
    const [input, result] = await Promise.all([jobInput(token, props.jobId), jobResult(token, props.jobId)])
    const passages = passagesOf(await input.text())
    const vectors = decodeVectors(new Uint8Array(await result.arrayBuffer()))
    rows.value = passageRows(passages, vectors)
  } catch (failure) {
    if (isSessionEnded(failure)) await endSession(router)
    else error.value = t(errorMessageKey(failure))
  } finally {
    loading.value = false
  }
}

/** Cherche dans les passages : la question est vectorisée dans le navigateur, puis comparée aux vecteurs. */
async function ask(): Promise<void> {
  const text = question.value.trim()
  if (text === '' || searching.value) return
  searching.value = true
  searchError.value = ''
  answered.value = false
  try {
    const { loadEmbedder } = await import('@/search/embedder')
    const embedder = await loadEmbedder((loaded, total) => {
      percent.value = total > 0 ? Math.round((loaded * 100) / total) : null
    })
    const vector = await embedder.embed(text, QUERY_PREFIX)
    hits.value = rank(vector, rows.value)
    answered.value = true
  } catch {
    searchError.value = t('job.search.failed')
  } finally {
    searching.value = false
    percent.value = null
  }
}

function download(content: string, type: string, extension: string): void {
  const url = URL.createObjectURL(new Blob([content], { type }))
  const link = document.createElement('a')
  link.href = url
  link.download = `silicium-${props.jobId.slice(0, 8)}.${extension}`
  document.body.append(link)
  link.click()
  link.remove()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

/** Export complet : tous les passages, avec leurs vecteurs. */
function exportAs(kind: 'json' | 'csv'): void {
  if (kind === 'json') download(toJson(rows.value), 'application/json', 'json')
  else download(toCsv(rows.value), 'text/csv;charset=utf-8', 'csv')
}

onMounted(load)
</script>

<template>
  <section class="embedding-result stack" aria-labelledby="embedding-result-title">
    <h2 id="embedding-result-title">{{ t('job.embedding.title') }}</h2>
    <Notice kind="error" :message="error" />
    <p v-if="loading" class="muted" role="status">{{ t('job.embedding.loading') }}</p>
    <template v-else-if="rows.length > 0">
      <p class="muted">{{ t('job.embedding.count', { count: rows.length, dims: 384 }) }}</p>
      <div class="actions">
        <button type="button" class="btn btn-primary" @click="exportAs('csv')">
          {{ t('job.embedding.exportCsv') }}
        </button>
        <button type="button" class="btn" @click="exportAs('json')">
          {{ t('job.embedding.exportJson') }}
        </button>
      </div>

      <form class="stack" role="search" @submit.prevent="ask">
        <label for="embedding-question">{{ t('job.search.label') }}</label>
        <div class="actions">
          <input
            id="embedding-question"
            v-model="question"
            type="search"
            class="input"
            :placeholder="t('job.search.placeholder')"
            :disabled="searching"
          />
          <button type="submit" class="btn btn-primary" :disabled="searching || question.trim() === ''">
            {{ t('job.search.submit') }}
          </button>
        </div>
        <template v-if="searching && percent !== null">
          <p class="muted" role="status">{{ t('job.search.loading', { percent }) }}</p>
          <progress :value="percent" max="100">{{ percent }} %</progress>
        </template>
        <p v-else-if="searching" class="muted" role="status">{{ t('job.search.searching') }}</p>
        <p v-if="searchError" class="field__error" role="alert">{{ searchError }}</p>
        <p v-if="answered && hits.length === 0" class="muted" role="status">{{ t('job.search.notFound') }}</p>
        <ol v-if="hits.length > 0" class="search-hits">
          <li v-for="hit in hits" :key="hit.index">
            <span class="muted">{{ t('job.search.passage', { index: hit.index, score: hit.score.toFixed(3) }) }}</span>
            <p>{{ hit.text }}</p>
          </li>
        </ol>
      </form>
    </template>
  </section>
</template>

<style scoped>
.search-hits {
  padding-left: 1.25rem;
}
.search-hits li + li {
  margin-top: 0.75rem;
}
.search-hits p {
  margin: 0.25rem 0 0;
}
</style>
