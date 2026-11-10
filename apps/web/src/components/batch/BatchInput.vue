<script setup lang="ts">
import { computed, reactive, ref, shallowRef, watch } from 'vue'
import { t } from '@/i18n'
import {
  BATCH_EXTENSIONS,
  FIT_CONTAIN,
  FIT_COVER,
  FIT_STRETCH,
  FLAG_FLIP_HORIZONTAL,
  FLAG_FLIP_VERTICAL,
  FLAG_GRAYSCALE,
  IMAGE_FORMATS,
  MAX_RECORDS,
  NO_OPTIONS,
  buildBatch,
  checkOptions,
  type BatchHeader,
  type BatchKind,
} from '@/workloads/batch'
import { readZip, ZipError } from '@/workloads/zip'

/** Plafond d'un côté d'image, comme le descripteur `rendu/raytracer` (4 096 pixels). */
const MAX_SIDE = 4096
/** Format JPEG dans `IMAGE_FORMATS` : le seul où la qualité compte. */
const JPEG = 1

const props = defineProps<{ kind: BatchKind; maxBytes: number }>()
/** Le lot prêt à envoyer (octets), ou null, avec le message d'erreur s'il y en a un. */
const emit = defineEmits<{ change: [payload: Uint8Array<ArrayBuffer> | null, error: string] }>()

interface Item {
  name: string
  data: Uint8Array
}

const items = shallowRef<Item[]>([])
const skipped = ref(0)
const reading = ref(false)
const readError = ref('')
const optionError = ref('')

const form = reactive({
  format: 0,
  width: '',
  height: '',
  fit: FIT_STRETCH,
  quality: '',
  rotate: 0,
  grayscale: false,
  flipHorizontal: false,
  flipVertical: false,
})

const formats = IMAGE_FORMATS
const accept = computed(() => [...BATCH_EXTENSIONS[props.kind].map((ext) => `.${ext}`), '.zip'].join(','))
const totalBytes = computed(() => items.value.reduce((sum, item) => sum + item.data.byteLength, 0))
const isJpeg = computed(() => props.kind === 'image' && form.format === JPEG)
const sized = computed(() => form.width !== '' || form.height !== '')
const shownError = computed(() => readError.value || optionError.value)

function number(value: string): number {
  return value === '' ? 0 : Number(value)
}

function sizeIsWrong(value: string): boolean {
  if (value === '') return false
  const size = Number(value)
  return !Number.isInteger(size) || size < 1 || size > MAX_SIDE
}

/** Les options du formulaire, mises sous leur forme canonique : une option sans effet vaut 0. */
function headerFromForm(): BatchHeader {
  return {
    kind: 'image',
    format: form.format,
    width: number(form.width),
    height: number(form.height),
    options: {
      ...NO_OPTIONS,
      quality: isJpeg.value ? number(form.quality) : 0,
      fit: sized.value ? form.fit : FIT_STRETCH,
      rotate: form.rotate,
      flags:
        (form.grayscale ? FLAG_GRAYSCALE : 0) |
        (form.flipHorizontal ? FLAG_FLIP_HORIZONTAL : 0) |
        (form.flipVertical ? FLAG_FLIP_VERTICAL : 0),
    },
  }
}

/** Assemble le lot et le remet au formulaire parent ; une erreur d'option ou de lecture bloque l'envoi. */
function publish(): void {
  if (readError.value) return emit('change', null, readError.value)
  if (items.value.length === 0) return emit('change', null, '')
  if (props.kind === 'image' && (sizeIsWrong(form.width) || sizeIsWrong(form.height))) {
    optionError.value = t('submit.batchSizeError')
    return emit('change', null, optionError.value)
  }
  const header = headerFromForm()
  const problem = checkOptions(header.kind, header.format, header.options)
  optionError.value = problem ? t('submit.batchOptionError') : ''
  if (problem) return emit('change', null, optionError.value)
  try {
    emit('change', buildBatch(header, items.value.map((item) => item.data)), '')
  } catch {
    optionError.value = t('submit.batchOptionError')
    emit('change', null, optionError.value)
  }
}

function usable(name: string): boolean {
  const extension = name.split('.').pop()?.toLowerCase() ?? ''
  return (BATCH_EXTENSIONS.image as readonly string[]).includes(extension)
}

function sorted(list: Item[]): Item[] {
  return list.sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }))
}

async function readFiles(files: File[]): Promise<Item[]> {
  const kept = files.filter((file) => usable(file.name))
  skipped.value = files.length - kept.length
  return sorted(await Promise.all(kept.map(async (file) => ({ name: file.name, data: new Uint8Array(await file.arrayBuffer()) }))))
}

async function readArchive(file: File): Promise<Item[]> {
  const bytes = new Uint8Array(await file.arrayBuffer())
  const entries = await readZip(bytes, props.maxBytes)
  const kept = entries.filter((entry) => usable(entry.name))
  skipped.value = entries.length - kept.length
  return kept
}

async function onPick(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const picked = Array.from(input.files ?? [])
  readError.value = ''
  skipped.value = 0
  if (picked.length === 0) return
  reading.value = true
  try {
    const single = picked.length === 1 && picked[0]?.name.toLowerCase().endsWith('.zip')
    const next = single && picked[0] ? await readArchive(picked[0]) : await readFiles(picked)
    if (next.length === 0) readError.value = t('submit.batchEmpty')
    else if (next.length > MAX_RECORDS) readError.value = t('submit.batchTooMany')
    else if (next.reduce((sum, item) => sum + item.data.byteLength, 0) > props.maxBytes) {
      readError.value = t('submit.batchTooLarge')
    }
    items.value = readError.value ? [] : next
  } catch (failure) {
    readError.value = failure instanceof ZipError ? t('submit.batchZipError') : t('submit.batchReadError')
    items.value = []
  } finally {
    reading.value = false
    input.value = ''
  }
}

watch(items, publish)
watch(form, publish, { deep: true })
publish()
</script>

<template>
  <div class="stack">
    <div class="field">
      <label for="batch-files">{{ t('submit.batchFiles') }}</label>
      <p id="batch-files-hint" class="field__hint">{{ t('submit.batchFilesHint') }}</p>
      <input
        id="batch-files"
        class="input"
        type="file"
        multiple
        :accept="accept"
        aria-describedby="batch-files-hint"
        :aria-invalid="shownError ? 'true' : undefined"
        @change="onPick"
      />
    </div>

    <p v-if="reading" role="status" class="muted">{{ t('common.loading') }}</p>
    <p v-if="items.length > 0" role="status" class="muted">
      {{ t('submit.batchChosen', { count: items.length, mb: (totalBytes / 1048576).toFixed(2) }) }}
    </p>
    <p v-if="skipped > 0" class="field__hint">{{ t('submit.batchSkipped', { count: skipped }) }}</p>
    <ol v-if="items.length > 0" class="muted batch-list">
      <li v-for="item in items" :key="item.name">{{ item.name }}</li>
    </ol>
    <p v-if="shownError" class="field__error" role="alert">{{ shownError }}</p>

    <fieldset class="stack">
      <legend>{{ t('submit.batchOptions') }}</legend>
      <p class="field__hint">{{ t('submit.batchOptionsHint') }}</p>

      <div class="field">
        <label for="batch-format">{{ t('submit.batchFormat') }}</label>
        <select id="batch-format" v-model.number="form.format" class="input">
          <option v-for="item in formats" :key="item.code" :value="item.code">
            {{ t(`submit.batchFormat_${item.name}`) }}
          </option>
        </select>
      </div>

      <template v-if="kind === 'image'">
        <div class="field">
          <label for="batch-width">{{ t('submit.imageWidth') }}</label>
          <input id="batch-width" v-model="form.width" class="input" type="number" min="1" :max="MAX_SIDE" step="1" />
        </div>
        <div class="field">
          <label for="batch-height">{{ t('submit.imageHeight') }}</label>
          <input id="batch-height" v-model="form.height" class="input" type="number" min="1" :max="MAX_SIDE" step="1" />
        </div>
        <p class="field__hint">{{ t('submit.batchSizeHint') }}</p>

        <div class="field">
          <label for="batch-fit">{{ t('submit.batchFit') }}</label>
          <select id="batch-fit" v-model.number="form.fit" class="input" :disabled="!sized">
            <option :value="FIT_STRETCH">{{ t('submit.batchFit_stretch') }}</option>
            <option :value="FIT_CONTAIN">{{ t('submit.batchFit_contain') }}</option>
            <option :value="FIT_COVER">{{ t('submit.batchFit_cover') }}</option>
          </select>
        </div>

        <div class="field">
          <label for="batch-quality">{{ t('submit.batchQuality') }}</label>
          <input id="batch-quality" v-model="form.quality" class="input" type="number" min="1" max="100" step="1" :disabled="!isJpeg" />
        </div>

        <div class="field">
          <label for="batch-rotate">{{ t('submit.batchRotate') }}</label>
          <select id="batch-rotate" v-model.number="form.rotate" class="input">
            <option :value="0">{{ t('submit.batchRotate_0') }}</option>
            <option :value="1">{{ t('submit.batchRotate_90') }}</option>
            <option :value="2">{{ t('submit.batchRotate_180') }}</option>
            <option :value="3">{{ t('submit.batchRotate_270') }}</option>
          </select>
        </div>

        <div class="field">
          <label><input v-model="form.grayscale" type="checkbox" /> {{ t('submit.batchGrayscale') }}</label>
          <label><input v-model="form.flipHorizontal" type="checkbox" /> {{ t('submit.batchFlipHorizontal') }}</label>
          <label><input v-model="form.flipVertical" type="checkbox" /> {{ t('submit.batchFlipVertical') }}</label>
        </div>
      </template>
    </fieldset>
  </div>
</template>
