<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { t } from '@/i18n'
import { profile, putJobOrigins } from '@/api/endpoints'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { networkTerms, publishJob, putContent, type TermsAnswer } from '@/api/wallet-endpoints'
import { currentToken, endSession } from '@/state/session'
import { depositEscrow } from '@/chain/deposit'
import { maxPayout, type EscrowSpec } from '@/chain/escrow'
import { connectWallet, signWithWallet, WalletFailure } from '@/platform/wallet'
import { encodeBase58 } from '@/signing/base58'
import { fromHex } from '@/signing/cbor'
import { submitJob, type SubmitDeps, type SubmitInput } from '@/signing/submit'
import { OPEN_WORKLOADS as WORKLOADS } from '@/workloads/find'
import { findWorkload, isImageSide } from '@/workloads/find'
import { sceneToCfg } from '@/scene/cfg'
import { sceneTemplate } from '@/scene/templates'
import type { SceneModel } from '@/scene/types'
import { clearDraft, loadDraft, saveDraft } from '@/state/submit-draft'
import { loadPreferences } from '@/state/preferences'
import FormField from '@/components/FormField.vue'
import Notice from '@/components/Notice.vue'
import SceneEditor from '@/components/scene/SceneEditor.vue'
import BatchInput from '@/components/batch/BatchInput.vue'
import EmbeddingInput from '@/components/embedding/EmbeddingInput.vue'
import { labelOf } from '@/workloads/documents'
import { startPreload } from '@/search/preload'
import { batchKindOf } from '@/workloads/batch'

// Fallback upload limit, used only if the workload descriptor has none.
const DEFAULT_MAX_BYTES = 8 * 1024 * 1024
const STEP_KEYS = ['submit.stepUpload', 'submit.stepSignSpec', 'submit.stepDeposit', 'submit.stepSignEvent', 'submit.stepPublish']

const router = useRouter()
const terms = ref<TermsAnswer | null>(null)
const termsError = ref('')
const accountAddress = ref('')
const draft = loadDraft(WORKLOADS.map((item) => item.id))
const label = ref(draft?.label ?? '')
const workload = ref(draft?.workload ?? loadPreferences(WORKLOADS.map((item) => item.id)).defaultWorkload ?? WORKLOADS[0]?.id ?? '')
const paramValues = reactive<Record<'width' | 'height', string>>({ width: '', height: '' })
const sizeError = ref('')
const file = ref<File | null>(null)
const busy = ref(false)
const stepText = ref('')
const error = ref('')
const fileError = ref('')
const jobId = ref('')
let termsRequest = 0

const spec = computed(() => findWorkload(workload.value))
const params = computed(() => spec.value?.params ?? [])
const maxBytes = computed(() => spec.value?.maxInputBytes ?? DEFAULT_MAX_BYTES)
// Format attendu du fichier : une scène .cfg pour le rendu, n'importe quel fichier sinon.
const isScene = computed(() => workload.value === 'rendu/raytracer')
const fileAccept = computed(() => (isScene.value ? '.cfg,text/plain' : undefined))
const fileHintKey = computed(() => (isScene.value ? 'submit.fileHintScene' : 'submit.fileHintAny'))
const canSubmit = computed(() => terms.value !== null && file.value !== null)
/** Bytes the node stores as the input: the file, with each declared parameter (u32) in front. */
const payloadSize = computed(() => (file.value ? file.value.size + 4 * params.value.length : 0))

/** A declared parameter as a number, or undefined while the field is empty. */
function numeric(name: 'width' | 'height'): number | undefined {
  return paramValues[name] === '' ? undefined : Number(paramValues[name])
}

function numericParams(): Partial<Record<'width' | 'height', number>> {
  return { width: numeric('width'), height: numeric('height') }
}

async function load(): Promise<void> {
  const token = currentToken()
  if (!token) return
  try {
    accountAddress.value = (await profile(token)).address
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
  fileError.value = chosen && chosen.size > maxBytes.value ? t('submit.tooLarge') : ''
  file.value = fileError.value ? null : chosen
}

/** Lot construit par le formulaire de lots : son fichier d'entrée, ou rien tant qu'il est invalide. */
const batchKind = computed(() => batchKindOf(workload.value))
const isEmbedding = computed(() => workload.value === 'embedding/e5-small')
function onText(chosen: File | null, error: string): void {
  file.value = chosen && !error ? chosen : null
}
/** Origine de chaque passage du fichier choisi, envoyée avec le job. */
const passageOrigins = ref<string[]>([])
function onOrigins(origins: string[]): void {
  passageOrigins.value = origins
}

/** Sans libellé saisi, le job prend le nom du ou des fichiers (64 caractères au plus). */
function onFileNames(names: string[]): void {
  if (label.value.trim() === '') label.value = labelOf(names)
}
function onBatch(payload: Uint8Array<ArrayBuffer> | null, error: string): void {
  file.value = payload && !error ? new File([payload], 'lot.silbatch', { type: 'application/octet-stream' }) : null
}

/**
 * A raytracing job's input is the scene the editor writes: its file is rebuilt from the model, and
 * the image size is the scene's own resolution (the engine checks both agree).
 */
const sceneModel = reactive(draft?.scene ?? sceneTemplate('demo')) as SceneModel
watch(
  () => ({ workload: workload.value, label: label.value, scene: sceneModel }),
  (value) => saveDraft(value),
  { deep: true },
)
let sceneSync: ReturnType<typeof setTimeout> | undefined

function syncScene(): void {
  file.value = new File([sceneToCfg(sceneModel, label.value.trim())], 'scene.cfg', { type: 'text/plain' })
  fileError.value = ''
  paramValues.width = String(sceneModel.width)
  paramValues.height = String(sceneModel.height)
}

watch(
  sceneModel,
  () => {
    clearTimeout(sceneSync)
    sceneSync = setTimeout(syncScene, 300)
  },
  { deep: true },
)

watch(workload, () => {
  if (params.value.length > 0) syncScene()
  else file.value = null
})
// Brouillon restauré : le fichier de scène se reconstruit depuis le modèle.
if (params.value.length > 0) syncScene()

/** Le réseau fixe fragments, tarifs, vérificateurs et comité : on les lit pour ce fichier. */
async function loadTerms(): Promise<void> {
  const request = ++termsRequest
  const token = currentToken()
  terms.value = null
  termsError.value = ''
  sizeError.value = ''
  const current = spec.value
  if (!current || !token || !file.value || payloadSize.value === 0) return
  if (!current.params.every((name) => isImageSide(Number(paramValues[name]), current))) {
    sizeError.value = t('submit.imageSizeError')
    return
  }
  try {
    const answer = await networkTerms(token, {
      workload: workload.value,
      size: payloadSize.value,
      width: numeric('width'),
      height: numeric('height'),
    })
    if (request === termsRequest) terms.value = answer
  } catch (failure) {
    if (request !== termsRequest) return
    if (isSessionEnded(failure)) await endSession(router)
    else termsError.value = t(errorMessageKey(failure))
  }
}

watch([file, workload, paramValues], loadTerms, { deep: true })

/** Dépôt estimé, lamports, sans le loyer du compte. Null pour les workloads sans dépôt sur le site. */
const deposit = computed<bigint | null>(() => {
  if (!terms.value || !spec.value?.depositSupported || payloadSize.value === 0) return null
  try {
    return maxPayout(escrowSpecOf(terms.value, payloadSize.value))
  } catch {
    return null
  }
})

function escrowSpecOf(answer: TermsAnswer, size: number): EscrowSpec {
  const values = answer.terms
  return {
    requester: new Uint8Array(32),
    workload: workload.value,
    inputSize: size,
    width: numeric('width') ?? 0,
    height: numeric('height') ?? 0,
    fragments: values.fragments,
    verificationsRequired: values.verifications_required,
    rewardPerUnit: values.reward_per_unit,
    verifierRewardPerUnit: values.verifier_reward_per_unit,
    expiresAtMs: 0,
  }
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
    deposit: (spec, jobId) => depositEscrow(spec, fromHex(jobId)),
    sha256: async (data) => new Uint8Array(await crypto.subtle.digest('SHA-256', data as BufferSource)),
    now: () => Date.now(),
    randomNonce,
    progress: (step) => {
      const index = ['upload', 'sign-spec', 'deposit', 'sign-event', 'publish'].indexOf(step)
      stepText.value = t('submit.step', { step: index + 1, label: t(STEP_KEYS[index]) })
    },
  }
}

async function submit(): Promise<void> {
  error.value = ''
  jobId.value = ''
  if (!terms.value || !file.value || sizeError.value || fileError.value) return
  const token = currentToken()
  if (!token) return
  if (!accountAddress.value) {
    error.value = t('wallet.noWalletAccount')
    return
  }
  // Le job met plusieurs minutes : le modèle de recherche se charge pendant ce temps.
  if (isEmbedding.value) startPreload()
  busy.value = true
  try {
    const key = await connectWallet()
    if (encodeBase58(key) !== accountAddress.value) {
      error.value = t('wallet.mismatch')
      return
    }
    const data = new Uint8Array(await file.value.arrayBuffer())
    const values = terms.value.terms
    const input: SubmitInput = {
      requester: key,
      data,
      workload: workload.value,
      params: numericParams(),
      label: label.value.trim(),
      terms: {
        fragments: values.fragments,
        rewardPerUnit: values.reward_per_unit,
        verifierRewardPerUnit: values.verifier_reward_per_unit,
        verificationsRequired: values.verifications_required,
        leaseMs: values.lease_ms,
        maxAttempts: values.max_attempts,
        expiresInMs: values.expires_in_ms,
      },
    }
    const result = await submitJob(input, depsFor(token))
    jobId.value = result.jobId
    // L'origine des passages est gardée avec le job (propriétaire seulement). Un échec ici ne bloque pas le calcul.
    if (isEmbedding.value && passageOrigins.value.length > 0) {
      await putJobOrigins(token, result.jobId, passageOrigins.value).catch(() => undefined)
    }
    clearDraft()
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

onMounted(() => {
  void load()
  void loadTerms()
})
</script>

<template>
  <section class="stack" aria-labelledby="submit-title">
    <h1 id="submit-title" tabindex="-1">{{ t('submit.title') }}</h1>
    <p>{{ t('submit.intro') }}</p>

    <Notice kind="info" :message="terms && terms.payments === false ? t('submit.noPayment') : ''" />
    <Notice kind="error" :message="termsError" />
    <Notice kind="error" :message="error" />

    <form class="card stack" novalidate @submit.prevent="submit">
      <FormField id="job-label" v-model="label" :label="t('submit.label')" autocomplete="off" />

      <div class="field">
        <label for="workload">{{ t('submit.workload') }}</label>
        <select id="workload" v-model="workload" class="input">
          <option v-for="item in WORKLOADS" :key="item.id" :value="item.id">{{ t(item.labelKey) }}</option>
        </select>
      </div>

      <div v-if="params.length > 0" class="stack">
        <p class="field__hint">{{ t('submit.imageHint') }}</p>
        <SceneEditor :model="sceneModel" />
        <p v-if="sizeError" class="field__error" role="alert">{{ sizeError }}</p>
      </div>

      <BatchInput
        v-else-if="batchKind"
        :key="workload"
        :kind="batchKind"
        :max-bytes="maxBytes"
        @change="onBatch"
      />

      <EmbeddingInput
        v-else-if="isEmbedding"
        :max-bytes="maxBytes"
        @change="onText"
        @files="onFileNames"
        @origins="onOrigins"
      />

      <div v-else class="field">
        <label for="input-file">{{ t('submit.file') }}</label>
        <p id="file-hint" class="field__hint">{{ t(fileHintKey) }}</p>
        <input
          id="input-file"
          class="input"
          type="file"
          :accept="fileAccept"
          aria-describedby="file-hint"
          :aria-invalid="fileError ? 'true' : undefined"
          @change="onFile"
        />
        <p v-if="fileError" class="field__error">{{ fileError }}</p>
      </div>

      <p v-if="terms" class="muted" role="status">
        {{
          deposit !== null
            ? t('submit.networkTerms', {
                fragments: terms.terms.fragments,
                reward: terms.terms.reward_per_unit,
                verifier: terms.terms.verifier_reward_per_unit,
                deposit: deposit.toString(),
              })
            : t('submit.networkTermsNoDeposit', {
                fragments: terms.terms.fragments,
                reward: terms.terms.reward_per_unit,
                verifier: terms.terms.verifier_reward_per_unit,
              })
        }}
      </p>

      <p class="muted">{{ t('submit.signTwice') }}</p>

      <div class="actions">
        <!-- type="button": Entrée dans un champ ne doit pas lancer la signature du wallet. -->
        <button type="button" class="btn btn-primary" :disabled="busy || !canSubmit" @click="submit">
          {{ t('submit.submit') }}
        </button>
        <p v-if="busy" role="status" class="muted">{{ stepText || t('common.loading') }}</p>
      </div>
    </form>

    <Notice kind="success" :message="jobId ? t('submit.done', { id: jobId }) : ''" />
  </section>
</template>
