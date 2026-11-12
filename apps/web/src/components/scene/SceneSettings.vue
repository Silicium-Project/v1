<script setup lang="ts">
import { computed } from 'vue'
import { t } from '@/i18n'
import SliderField from './SliderField.vue'
import { RENDER_QUALITIES, RESOLUTION_PRESETS, renderCost, type RenderQuality, type SceneModel } from '@/scene/types'

// Image, caméra, qualité du rendu final, ciel et exposition.
const props = defineProps<{ model: SceneModel }>()
const QUALITIES = Object.keys(RENDER_QUALITIES) as RenderQuality[]

const cost = computed(() => renderCost(props.model))

function applyResolution(id: string): void {
  const preset = RESOLUTION_PRESETS.find((item) => item.id === id)
  if (preset) {
    props.model.width = preset.width
    props.model.height = preset.height
  }
}
</script>

<template>
  <section class="scene-settings" :aria-labelledby="'scene-settings-title'">
    <h3 id="scene-settings-title">{{ t('scene.settings.title') }}</h3>
    <div class="field">
      <label for="scene-resolution">{{ t('scene.settings.resolution') }}</label>
      <select id="scene-resolution" class="input" @change="applyResolution(($event.target as HTMLSelectElement).value)">
        <option v-for="preset in RESOLUTION_PRESETS" :key="preset.id" :value="preset.id">
          {{ t(`scene.settings.presets.${preset.id}`) }} ({{ preset.width }} × {{ preset.height }})
        </option>
      </select>
    </div>
    <div class="scene-form__grid">
      <SliderField id="scene-width" :label="t('scene.settings.width')" v-model="model.width" :min="1" :max="4096" :step="1" />
      <SliderField id="scene-height" :label="t('scene.settings.height')" v-model="model.height" :min="1" :max="4096" :step="1" />
      <SliderField id="scene-fov" :label="t('scene.settings.fov')" v-model="model.fov" :min="1" :max="179" :step="1" />
    </div>

    <h4>{{ t('scene.settings.camera') }}</h4>
    <div class="scene-form__grid">
      <NumberField id="cam-x" :label="t('scene.form.x')" v-model="model.camera.x" :step="0.1" />
      <NumberField id="cam-y" :label="t('scene.form.y')" v-model="model.camera.y" :step="0.1" />
      <NumberField id="cam-z" :label="t('scene.form.z')" v-model="model.camera.z" :step="0.1" />
      <NumberField id="cam-yaw" :label="t('scene.settings.yaw')" v-model="model.camera.yaw" :step="1" />
      <NumberField id="cam-pitch" :label="t('scene.settings.pitch')" v-model="model.camera.pitch" :min="-89" :max="89" :step="1" />
      <NumberField id="cam-aperture" :label="t('scene.settings.aperture')" v-model="model.camera.aperture" :min="0" :step="0.005" />
      <NumberField id="cam-focus" :label="t('scene.settings.focus')" v-model="model.camera.focus" :min="0.01" :step="0.1" />
    </div>

    <h4>{{ t('scene.settings.render') }}</h4>
    <div class="field">
      <label for="scene-quality">{{ t('scene.settings.quality') }}</label>
      <select
        id="scene-quality"
        class="input"
        :value="model.quality ?? 'draft'"
        @change="model.quality = ($event.target as HTMLSelectElement).value as RenderQuality"
      >
        <option v-for="quality in QUALITIES" :key="quality" :value="quality">{{ t(`scene.settings.qualityName.${quality}`) }}</option>
      </select>
      <p class="field__hint">{{ t('scene.settings.cost', { rays: Math.round(cost).toLocaleString() }) }}</p>
    </div>
    <div class="scene-form__grid">
      <NumberField
        id="scene-sky"
        :label="t('scene.settings.sky')"
        :model-value="model.sky ?? 1"
        :min="0"
        :step="0.05"
        @update:model-value="model.sky = $event"
      />
      <NumberField
        id="scene-exposure"
        :label="t('scene.settings.exposure')"
        :model-value="model.exposure ?? 1"
        :min="0"
        :step="0.05"
        @update:model-value="model.exposure = $event"
      />
    </div>
  </section>
</template>
