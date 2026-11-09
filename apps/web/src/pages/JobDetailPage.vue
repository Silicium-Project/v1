<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { t } from '@/i18n'
import { formatDateTime, formatTime } from '@/i18n/format'
import { getJob, jobResult, type JobStatus, type JobSummary } from '@/api/endpoints'
import { cancelJob } from '@/api/wallet-endpoints'
import { errorCodeOf, errorMessageKey, isSessionEnded } from '@/api/errors'
import { connectWallet, signWithWallet } from '@/platform/wallet'
import { fromHex } from '@/signing/cbor'
import { escrowConfig } from '@/chain/config'
import { escrowAddress } from '@/chain/escrow'
import { encodeSignedEvent } from '@/signing/job'
import { cancelEvent, cancelSigningMessage, encodeCancelBody } from '@/signing/cancel'
import { currentToken, endSession } from '@/state/session'
import { normalizeState, stateDescriptionKey } from '@/status'
import { finalizedCount, isLiveState, POLL_MS } from '@/status-progress'
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
const downloading = ref(false)
const downloadError = ref('')
const cancelAsk = ref(false)
const cancelling = ref(false)
const cancelError = ref('')

/** Annule le calcul : le requester signe son événement `Cancel` avec son wallet, le nœud le vérifie. */
async function cancelCalc(): Promise<void> {
  const token = currentToken()
  const head = status.value?.requester_head
  if (!token || !job.value || !head) return
  cancelling.value = true
  cancelError.value = ''
  try {
    const requester = await connectWallet()
    const event = cancelEvent(fromHex(job.value.id), requester, head, Date.now())
    const signature = await signWithWallet(cancelSigningMessage(event), 'utf8')
    await cancelJob(token, job.value.id, encodeSignedEvent(event, encodeCancelBody(), signature))
    cancelAsk.value = false
    await load(true)
  } catch (failure) {
    if (isSessionEnded(failure)) await endSession(router)
    else cancelError.value = t('job.cancelFailed')
  } finally {
    cancelling.value = false
  }
}

// Silent refreshes (while the job moves) keep the page still: no loading line, no flicker.
async function load(silent = false): Promise<void> {
  const token = currentToken()
  if (!token) return
  if (!silent) loading.value = true
  error.value = ''
  notFound.value = false
  try {
    const answer = await getJob(token, jobId.value)
    job.value = answer.job
    status.value = answer.status
    updatedAt.value = new Date().toISOString()
    syncPolling()
    if (state.value === 'finalized' && status.value.image) void loadPreview()
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
    if (!silent) loading.value = false
  }
}

/** Télécharge le résultat d'un calcul finalisé : PNG pour le rendu, texte sinon. Le serveur fournit le fichier. */
async function downloadResult(): Promise<void> {
  const token = currentToken()
  if (!token || !job.value) return
  downloading.value = true
  downloadError.value = ''
  try {
    const file = await jobResult(token, job.value.id)
    const url = URL.createObjectURL(file)
    const link = document.createElement('a')
    link.href = url
    link.download = `silicium-${job.value.id.slice(0, 8)}.${status.value?.image ? 'png' : 'txt'}`
    document.body.append(link)
    link.click()
    link.remove()
    // Revoke after the browser had time to start the download.
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  } catch (failure) {
    if (isSessionEnded(failure)) await endSession(router)
    else downloadError.value = t(errorMessageKey(failure))
  } finally {
    downloading.value = false
  }
}

// The picture of a finished render job, shown above the download: fetched once per job.
const previewUrl = ref('')
let previewOf = ''

async function loadPreview(): Promise<void> {
  const token = currentToken()
  if (!token || !job.value || previewOf === job.value.id) return
  try {
    const file = await jobResult(token, job.value.id)
    if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
    previewUrl.value = URL.createObjectURL(file)
    previewOf = job.value.id
  } catch (failure) {
    if (isSessionEnded(failure)) await endSession(router)
  }
}

const state = computed(() => normalizeState(status.value?.state ?? 'unknown'))
// Compte d'escrow du calcul : son historique de transactions est ce qui règle le calcul sur devnet.
const chainUrl = computed(() => {
  if (!job.value) return ''
  const { program } = escrowConfig()
  const address = escrowAddress(program, fromHex(job.value.id))
  return `https://explorer.solana.com/address/${address.toBase58()}?cluster=devnet`
})

const taskEntries = computed(() => Object.entries(status.value?.tasks ?? {}))
const fragments = computed(() => status.value?.fragments ?? [])
const finalCount = computed(() => finalizedCount(fragments.value))
const live = computed(() => job.value !== null && status.value !== null && isLiveState(state.value))

const NODE_STATUSES = [
  'Open',
  'Claimed',
  'LeaseExpired',
  'Released',
  'Completed',
  'Verified',
  'Disputed',
  'Rejected',
  'Failed',
  'Finalized',
  'Cancelled',
  'Expired',
]

/** Node status name in French or English; an unknown name is shown as the node sent it. */
function nodeStatusLabel(name: string): string {
  return NODE_STATUSES.includes(name) ? t(`job.fragmentStatus.${name}`) : name
}

// The page follows a moving job by itself and stops once it is final, cancelled or unknown.
let poll: ReturnType<typeof setInterval> | undefined
function syncPolling(): void {
  if (live.value && poll === undefined) {
    poll = setInterval(() => void load(true), POLL_MS)
  } else if (!live.value && poll !== undefined) {
    clearInterval(poll)
    poll = undefined
  }
}

onMounted(() => void load())
onUnmounted(() => {
  if (poll !== undefined) clearInterval(poll)
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
})
watch(jobId, () => void load())
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

        <div v-if="fragments.length > 0" class="job-progress">
          <h2>{{ t('job.progress') }}</h2>
          <p>{{ t('job.progressSummary', { done: finalCount, total: fragments.length }) }}</p>
          <progress :value="finalCount" :max="fragments.length">{{ finalCount }} / {{ fragments.length }}</progress>
          <ul class="job-progress__list">
            <li v-for="fragment in fragments" :key="fragment.index">
              <strong>{{ t('job.fragment', { index: fragment.index }) }}</strong>
              <span>{{ nodeStatusLabel(fragment.status) }}</span>
              <span class="muted">{{ t('job.attempts', { n: fragment.attempts }) }} · {{ t('job.approvals', { n: fragment.approvals }) }}</span>
            </li>
          </ul>
          <p v-if="live" class="muted">{{ t('job.autoRefresh') }}</p>
        </div>
      </div>

      <figure v-if="previewUrl" class="job-preview">
        <img :src="previewUrl" :alt="t('job.previewAlt')" />
        <figcaption class="muted">{{ t('job.previewCaption') }}</figcaption>
      </figure>
      <Notice kind="error" :message="downloadError" />
      <div v-if="state === 'running' && status.requester_head" class="stack">
        <p class="muted">{{ t('job.cancelNote') }}</p>
        <Notice kind="error" :message="cancelError" />
        <div v-if="!cancelAsk" class="actions">
          <button type="button" class="btn" @click="cancelAsk = true">{{ t('job.cancel') }}</button>
        </div>
        <div v-else class="actions">
          <p>{{ t('job.cancelAsk') }}</p>
          <button type="button" class="btn btn-primary" :disabled="cancelling" @click="cancelCalc">
            {{ cancelling ? t('common.loading') : t('job.cancelConfirm') }}
          </button>
          <button type="button" class="btn" :disabled="cancelling" @click="cancelAsk = false">{{ t('job.cancelKeep') }}</button>
        </div>
      </div>
      <div class="actions">
        <button v-if="state === 'finalized'" type="button" class="btn btn-primary" :disabled="downloading" @click="downloadResult">
          {{ status.image ? t('job.downloadResult') : t('job.downloadResultText') }}
        </button>
        <p v-if="state === 'finalized'" class="muted">{{ status.image ? t('job.downloadNote') : t('job.downloadNoteText') }}</p>
        <a v-if="chainUrl" class="btn" :href="chainUrl" target="_blank" rel="noopener noreferrer">{{ t('job.viewOnChain') }}</a>
        <button type="button" class="btn" :disabled="loading" @click="load()">{{ t('common.refresh') }}</button>
        <p role="status" class="muted">{{ t('job.refreshed', { time: formatTime(updatedAt) }) }}</p>
      </div>
    </template>
  </section>
</template>
