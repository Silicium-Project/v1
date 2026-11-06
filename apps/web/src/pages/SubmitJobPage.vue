<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { t } from '@/i18n'
import { profile } from '@/api/endpoints'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { committee as loadCommittee, publishJob, putContent, type CommitteeAnswer } from '@/api/wallet-endpoints'
import { currentToken, endSession } from '@/state/session'
import { connectWallet, signWithWallet, WalletFailure } from '@/platform/wallet'
import { encodeBase58 } from '@/signing/base58'
import { fromHex } from '@/signing/cbor'
import { isSide, RENDER_WORKLOAD, submitJob, type SubmitDeps } from '@/signing/submit'
import FormField from '@/components/FormField.vue'
import Notice from '@/components/Notice.vue'

// Upload limit of this page. The API accepts more; a job input this big is not practical yet.
const MAX_BYTES = 8 * 1024 * 1024
const MAX_FRAGMENTS = 16
const STEP_KEYS = ['submit.stepUpload', 'submit.stepSignSpec', 'submit.stepSignEvent', 'submit.stepPublish']

const router = useRouter()
const info = ref<CommitteeAnswer | null>(null)
const accountAddress = ref('')
const label = ref('')
const workload = ref('calcul/sha256')
const fragments = ref('1')
const width = ref('')
const height = ref('')
const sizeError = ref('')
const file = ref<File | null>(null)
const busy = ref(false)
const stepText = ref('')
const error = ref('')
const fileError = ref('')
const fragmentsError = ref('')
const jobId = ref('')

const canSubmit = computed(() => info.value !== null && info.value.committee.length > 0)
const isRender = computed(() => workload.value === RENDER_WORKLOAD)

async function load(): Promise<void> {
  const token = currentToken()
  if (!token) return
  try {
    const [committeeAnswer, account] = await Promise.all([loadCommittee(token), profile(token)])
    info.value = committeeAnswer
    accountAddress.value = account.address
  } catch (failure) {
    if (isSessionEnded(failure)) {
      await endSession(router)
      return
    }
    error.value = t(errorMessageKey(failure))
  }
}

function onFile(event: Event): void {
  const chosen = (event.target as HTMLInputElement).files?.[0] ?? null
  fileError.value = chosen && chosen.size > MAX_BYTES ? t('submit.tooLarge') : ''
  file.value = fileError.value ? null : chosen
}

function validate(): boolean {
  const count = Number(fragments.value)
  fragmentsError.value = Number.isInteger(count) && count >= 1 && count <= MAX_FRAGMENTS ? '' : t('submit.fragmentsHint')
  const sized = !isRender.value || (isSide(Number(width.value)) && isSide(Number(height.value)))
  sizeError.value = sized ? '' : t('submit.imageSizeError')
  if (!file.value && !fileError.value) fileError.value = t('submit.noFile')
  return !fragmentsError.value && !sizeError.value && !fileError.value && file.value !== null
}

function randomNonce(): number {
  const bytes = new Uint8Array(6)
  crypto.getRandomValues(bytes)
  return bytes.reduce((value, byte) => value * 256 + byte, 0)
}

function depsFor(token: string): SubmitDeps {
  return {
    putContent: (data) => putContent(token, data),
    sign: (message, display) => signWithWallet(message, display),
    publish: (id, event, name) => publishJob(token, id, event, name),
    sha256: async (data) => new Uint8Array(await crypto.subtle.digest('SHA-256', data as BufferSource)),
    now: () => Date.now(),
    randomNonce,
    progress: (step) => {
      const index = ['upload', 'sign-spec', 'sign-event', 'publish'].indexOf(step)
      stepText.value = t('submit.step', { step: index + 1, label: t(STEP_KEYS[index]) })
    },
  }
}

async function submit(): Promise<void> {
  error.value = ''
  jobId.value = ''
  if (!validate() || !file.value || !info.value) return
  const token = currentToken()
  if (!token) return
  if (!accountAddress.value) {
    error.value = t('wallet.noWalletAccount')
    return
  }
  busy.value = true
  try {
    const key = await connectWallet()
    if (encodeBase58(key) !== accountAddress.value) {
      error.value = t('wallet.mismatch')
      return
    }
    const data = new Uint8Array(await file.value.arrayBuffer())
    const result = await submitJob(
      {
        requester: key,
        data,
        workload: workload.value,
        fragments: Number(fragments.value),
        width: Number(width.value),
        height: Number(height.value),
        committee: info.value.committee.map(fromHex),
        threshold: info.value.threshold,
        label: label.value.trim(),
      },
      depsFor(token),
    )
    jobId.value = result.jobId
    await router.push({ name: 'job', params: { id: result.jobId } })
  } catch (failure) {
    if (failure instanceof WalletFailure) error.value = t(`wallet.${failure.code}`)
    else if (isSessionEnded(failure)) await endSession(router)
    else error.value = t(errorMessageKey(failure))
  } finally {
    busy.value = false
    stepText.value = ''
  }
}

onMounted(load)
</script>

<template>
  <section class="stack" aria-labelledby="submit-title">
    <h1 id="submit-title" tabindex="-1">{{ t('submit.title') }}</h1>
    <p>{{ t('submit.intro') }}</p>

    <Notice kind="info" :message="info && info.payments === false ? t('submit.noPayment') : ''" />
    <Notice kind="error" :message="info && !canSubmit ? t('submit.noCommittee') : ''" />
    <Notice kind="error" :message="error" />

    <form class="card stack" novalidate @submit.prevent="submit">
      <FormField id="job-label" v-model="label" :label="t('submit.label')" autocomplete="off" />

      <div class="field">
        <label for="workload">{{ t('submit.workload') }}</label>
        <select id="workload" v-model="workload" class="input">
          <option value="calcul/sha256">{{ t('submit.workloadSha256') }}</option>
          <option value="rendu/blocs">{{ t('submit.workloadRender') }}</option>
        </select>
      </div>

      <FormField
        id="fragments"
        v-model="fragments"
        type="number"
        :label="t('submit.fragments')"
        :hint="t('submit.fragmentsHint')"
        :error="fragmentsError"
        required
      />

      <div v-if="isRender" class="stack">
        <FormField
          id="image-width"
          v-model="width"
          type="number"
          :label="t('submit.imageWidth')"
          :hint="t('submit.imageHint')"
          :error="sizeError"
          required
        />
        <FormField id="image-height" v-model="height" type="number" :label="t('submit.imageHeight')" required />
      </div>

      <div class="field">
        <label for="input-file">{{ t('submit.file') }}</label>
        <p id="file-hint" class="field__hint">{{ t('submit.fileHint') }}</p>
        <input
          id="input-file"
          class="input"
          type="file"
          aria-describedby="file-hint"
          :aria-invalid="fileError ? 'true' : undefined"
          @change="onFile"
        />
        <p v-if="fileError" class="field__error">{{ fileError }}</p>
      </div>

      <p class="muted">{{ t('submit.signTwice') }}</p>

      <div class="actions">
        <button type="submit" class="btn btn-primary" :disabled="busy || !canSubmit">
          {{ t('submit.submit') }}
        </button>
        <p v-if="busy" role="status" class="muted">{{ stepText || t('common.loading') }}</p>
      </div>
    </form>

    <Notice kind="success" :message="jobId ? t('submit.done', { id: jobId }) : ''" />
  </section>
</template>
