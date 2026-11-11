<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { t } from '@/i18n'
import { jobInput, jobResult } from '@/api/endpoints'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { currentToken, endSession } from '@/state/session'
import Notice from '@/components/Notice.vue'
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

/** Lignes affichées ; l'export contient toujours tous les passages. */
const SHOWN = 100
const rows = ref<PassageRow[]>([])
const loading = ref(true)
const error = ref('')
const shown = computed(() => rows.value.slice(0, SHOWN))

function preview(vector: number[]): string {
  return vector.slice(0, 4).map((value) => value.toFixed(4)).join(' · ') + ' …'
}

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
      <div class="table-wrap">
        <table class="table">
          <thead>
            <tr>
              <th scope="col">{{ t('job.embedding.colIndex') }}</th>
              <th scope="col">{{ t('job.embedding.colText') }}</th>
              <th scope="col">{{ t('job.embedding.colVector') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in shown" :key="row.index">
              <td>{{ row.index }}</td>
              <td>{{ row.text }}</td>
              <td><code>{{ preview(row.vector) }}</code></td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-if="rows.length > SHOWN" class="muted">{{ t('job.embedding.truncated', { shown: SHOWN, count: rows.length }) }}</p>
    </template>
  </section>
</template>

<style scoped>
.table-wrap {
  overflow-x: auto;
}
table {
  width: 100%;
  border-collapse: collapse;
}
th,
td {
  text-align: left;
  vertical-align: top;
  padding: 0.4rem 0.6rem;
  border-bottom: 1px solid var(--color-border);
}
td code {
  white-space: nowrap;
}
</style>
