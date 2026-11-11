<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { t } from '@/i18n'
import { passagesFile } from '@/workloads/embedding'

const props = defineProps<{ maxBytes: number }>()
const emit = defineEmits<{ change: [file: File | null, error: string] }>()

const text = ref('')
const check = computed(() => passagesFile(text.value, props.maxBytes))
const errorText = computed(() => {
  if (check.value.error === 'empty') return text.value === '' ? '' : t('submit.embeddingEmpty')
  if (check.value.error === 'tooLarge') return t('submit.embeddingTooLarge')
  return ''
})

watch(
  check,
  (value) => emit('change', value.file, value.error),
  { immediate: true },
)
</script>

<template>
  <div class="field">
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
