<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { t } from '@/i18n'
import { renderPreview } from '@/scene/preview'
import type { SceneModel } from '@/scene/types'

// Aperçu : le moteur redessine la scène après une courte pause, à chaque modification.
const props = defineProps<{ model: SceneModel }>()
const canvas = ref<HTMLCanvasElement | null>(null)
const busy = ref(false)
const error = ref('')
let timer: ReturnType<typeof setTimeout> | undefined
let request = 0

async function draw(): Promise<void> {
  const current = ++request
  busy.value = true
  error.value = ''
  try {
    const image = await renderPreview(props.model)
    if (current !== request || !canvas.value) return
    canvas.value.width = image.width
    canvas.value.height = image.height
    const context = canvas.value.getContext('2d')
    context?.putImageData(new ImageData(image.rgba, image.width, image.height), 0, 0)
  } catch (failure) {
    if (current === request) error.value = failure instanceof Error ? failure.message : String(failure)
  } finally {
    if (current === request) busy.value = false
  }
}

watch(
  () => props.model,
  () => {
    clearTimeout(timer)
    timer = setTimeout(draw, 350)
  },
  { deep: true, immediate: true },
)

onBeforeUnmount(() => clearTimeout(timer))
</script>

<template>
  <figure class="scene-preview">
    <canvas ref="canvas" class="scene-preview__canvas" :aria-label="t('scene.preview.label')" />
    <figcaption class="muted" role="status">
      {{ busy ? t('scene.preview.busy') : t('scene.preview.hint') }}
      <span v-if="error" class="field__error"> {{ error }}</span>
    </figcaption>
  </figure>
</template>
