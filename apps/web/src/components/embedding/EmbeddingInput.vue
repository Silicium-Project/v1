<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { t } from '@/i18n'
import { DocumentError, passagesOf } from '@/workloads/documents'
import { passagesFile } from '@/workloads/embedding'

const props = defineProps<{ maxBytes: number }>()
const emit = defineEmits<{ change: [file: File | null, error: string] }>()

/** Texte extrait de tous les fichiers choisis (aucune saisie à la main). */
const extracted = ref('')
const busy = ref(false)
const fileError = ref('')
const names = ref<string[]>([])
const check = computed(() => passagesFile(extracted.value, props.maxBytes))

const ERROR_KEYS: Record<string, string> = {
  unsupported: 'submit.embeddingFileUnsupported',
  invalid: 'submit.embeddingFileInvalid',
  noText: 'submit.embeddingFileNoText',
  tooLarge: 'submit.embeddingFileTooLarge',
}

const errorText = computed(() => {
  if (fileError.value) return fileError.value
  if (check.value.error === 'empty' && extracted.value !== '') return t('submit.embeddingEmpty')
  if (check.value.error === 'tooLarge') return t('submit.embeddingTooLarge')
  return ''
})

/** Extrait le texte de tous les fichiers choisis, en entier. Un échec nomme les fichiers concernés. */
async function onFiles(event: Event) {
  const input = event.target as HTMLInputElement
  const files = Array.from(input.files ?? [])
  if (files.length === 0) return
  busy.value = true
  fileError.value = ''
  names.value = files.map((file) => file.name)
  try {
    extracted.value = await passagesOf(files)
  } catch (error) {
    extracted.value = ''
    const key = error instanceof DocumentError ? ERROR_KEYS[error.code] : undefined
    const listed = error instanceof DocumentError ? error.names.join(', ') : ''
    fileError.value = t(key ?? 'submit.embeddingFileInvalid', { names: listed })
  } finally {
    busy.value = false
    input.value = ''
  }
}

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
      multiple
      :disabled="busy"
      aria-describedby="embedding-file-hint embedding-count"
      @change="onFiles"
    />
    <p v-if="busy" class="muted" role="status">{{ t('submit.embeddingFileBusy') }}</p>
    <p v-else-if="names.length > 0 && !fileError" class="muted" role="status">
      {{ t('submit.embeddingFilesChosen', { names: names.join(', ') }) }}
    </p>
    <p id="embedding-count" class="muted" role="status">
      {{ t('submit.embeddingCount', { count: check.count, bytes: check.bytes }) }}
    </p>
    <p v-if="errorText" class="field__error" role="alert">{{ errorText }}</p>
  </div>
</template>
