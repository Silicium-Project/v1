<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { t } from '@/i18n'
import { DocumentError, passagesWithOrigins } from '@/workloads/documents'
import { passagesFile } from '@/workloads/embedding'

const props = defineProps<{ maxBytes: number }>()
const emit = defineEmits<{
  change: [file: File | null, error: string]
  files: [names: string[]]
  /** Origine de chaque passage (fichier, page, ligne), dans l'ordre des fichiers choisis. */
  origins: [origins: string[]]
}>()

/** Fichiers choisis, cumulés d'un choix à l'autre (chaque ajout s'ajoute à la liste). */
const selected = ref<File[]>([])
/** Texte extrait de tous les fichiers choisis (aucune saisie à la main). */
const extracted = ref('')
const busy = ref(false)
const fileError = ref('')
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

/** Clé d'un fichier : même nom, taille et date = même fichier, pas deux fois dans la liste. */
function sameFile(a: File, b: File): boolean {
  return a.name === b.name && a.size === b.size && a.lastModified === b.lastModified
}

/** Extrait le texte de tous les fichiers de la liste, en entier. Un échec nomme les fichiers concernés. */
async function extract(files: File[]): Promise<void> {
  fileError.value = ''
  if (files.length === 0) {
    extracted.value = ''
    emit('origins', [])
    emit('files', [])
    return
  }
  busy.value = true
  try {
    const extraction = await passagesWithOrigins(files)
    extracted.value = extraction.text
    emit('origins', extraction.origins)
    emit('files', files.map((file) => file.name))
  } catch (error) {
    extracted.value = ''
    emit('origins', [])
    const key = error instanceof DocumentError ? ERROR_KEYS[error.code] : undefined
    const listed = error instanceof DocumentError ? error.names.join(', ') : ''
    fileError.value = t(key ?? 'submit.embeddingFileInvalid', { names: listed })
  } finally {
    busy.value = false
  }
}

/** Ajoute les fichiers choisis à la liste existante, puis extrait l'ensemble. */
async function onFiles(event: Event) {
  const input = event.target as HTMLInputElement
  const added = Array.from(input.files ?? [])
  input.value = ''
  if (added.length === 0) return
  const merged = [...selected.value]
  for (const file of added) {
    if (!merged.some((existing) => sameFile(existing, file))) merged.push(file)
  }
  selected.value = merged
  await extract(merged)
}

/** Retire un fichier de la liste, puis extrait le reste. */
async function remove(index: number) {
  selected.value = selected.value.filter((_, at) => at !== index)
  await extract(selected.value)
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
    <ul v-if="selected.length > 0" class="embedding-files">
      <li v-for="(file, index) in selected" :key="`${file.name}-${file.size}-${file.lastModified}`">
        <span>{{ file.name }}</span>
        <button
          type="button"
          class="btn"
          :disabled="busy"
          :aria-label="t('submit.embeddingFileRemove', { name: file.name })"
          @click="remove(index)"
        >
          {{ t('submit.embeddingFileRemoveLabel') }}
        </button>
      </li>
    </ul>
    <p id="embedding-count" class="muted" role="status">
      {{ t('submit.embeddingCount', { count: check.count, bytes: check.bytes }) }}
    </p>
    <p v-if="errorText" class="field__error" role="alert">{{ errorText }}</p>
  </div>
</template>

<style scoped>
.embedding-files {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 0.4rem;
}
.embedding-files li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}
</style>
