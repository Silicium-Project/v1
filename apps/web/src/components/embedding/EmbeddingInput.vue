<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { t } from '@/i18n'
import { DocumentError, passagesOf } from '@/workloads/documents'
import { passagesFile } from '@/workloads/embedding'

const props = defineProps<{ maxBytes: number }>()
const emit = defineEmits<{ change: [file: File | null, error: string] }>()

const text = ref('')
const busy = ref(false)
const fileError = ref('')
const check = computed(() => passagesFile(text.value, props.maxBytes))

const FILE_ERRORS: Record<string, string> = {
  unsupported: 'submit.embeddingFileUnsupported',
  invalid: 'submit.embeddingFileInvalid',
  noText: 'submit.embeddingFileNoText',
  tooLarge: 'submit.embeddingFileTooLarge',
}

const errorText = computed(() => {
  if (fileError.value) return fileError.value
  if (check.value.error === 'empty') return text.value === '' ? '' : t('submit.embeddingEmpty')
  if (check.value.error === 'tooLarge') return t('submit.embeddingTooLarge')
  return ''
})

/** Remplit la zone de texte avec les passages extraits : l'utilisateur peut les relire et les modifier. */
async function onFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  busy.value = true
  fileError.value = ''
  try {
    text.value = await passagesOf(file)
  } catch (error) {
    const key = error instanceof DocumentError ? FILE_ERRORS[error.code] : undefined
    fileError.value = t(key ?? 'submit.embeddingFileInvalid')
  } finally {
    busy.value = false
    input.value = ''
  }
}

// Une correction à la main efface l'erreur d'un fichier refusé.
watch(text, () => {
  fileError.value = ''
})

watch(
  check,
  (value) => emit('change', value.file, value.error),
  { immediate: true },
)
</script>

<template>
  <div class="field">
    <label for="embedding-file">{{ t('submit.embeddingFileLabel') }}</label>
    <p id="embedding-file-hint" class="field__hint">{{ t('submit.embeddingFileHint') }}</p>
    <input
      id="embedding-file"
      type="file"
      accept=".pdf,.zip,.txt,.md,application/pdf,application/zip,text/plain,text/markdown"
      :disabled="busy"
      aria-describedby="embedding-file-hint"
      @change="onFile"
    />
    <p v-if="busy" class="muted" role="status">{{ t('submit.embeddingFileBusy') }}</p>
    <label for="embedding-text">{{ t('submit.embeddingLabel') }}</label>
    <p id="embedding-hint" class="field__hint">{{ t('submit.embeddingHint') }}</p>
    <textarea
      id="embedding-text"
      v-model="text"
      class="input"
      rows="10"
      aria-describedby="embedding-hint embedding-count"
      :aria-invalid="errorText ? 'true' : undefined"
      spellcheck="false"
    ></textarea>
    <p id="embedding-count" class="muted" role="status">
      {{ t('submit.embeddingCount', { count: check.count, bytes: check.bytes }) }}
    </p>
    <p v-if="errorText" class="field__error" role="alert">{{ errorText }}</p>
  </div>
</template>
