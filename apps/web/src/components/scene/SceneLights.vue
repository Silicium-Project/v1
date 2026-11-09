<script setup lang="ts">
import { ref } from 'vue'
import { t } from '@/i18n'
import ColorField from './ColorField.vue'
import NumberField from './NumberField.vue'
import { newLight } from '@/scene/factory'
import type { SceneLight, SceneModel } from '@/scene/types'

// Les lumières : un point de lumière (ombres nettes) ou une boule lumineuse (ombres douces).
const props = defineProps<{ model: SceneModel }>()
const selected = ref<number | null>(null)

function add(kind: SceneLight['kind']): void {
  const light = newLight({ kind, x: 3, y: 2, z: 5, intensity: kind === 'area' ? 320 : 400 })
  props.model.lights.push(light)
  selected.value = light.id
}

function remove(id: number): void {
  const index = props.model.lights.findIndex((light) => light.id === id)
  if (index >= 0) props.model.lights.splice(index, 1)
  if (selected.value === id) selected.value = null
}
</script>

<template>
  <section class="scene-lights" :aria-labelledby="'scene-lights-title'">
    <h3 id="scene-lights-title">{{ t('scene.lights.title') }}</h3>
    <div class="scene-objects__add" role="group" :aria-label="t('scene.lights.add')">
      <button type="button" class="btn" @click="add('point')">+ {{ t('scene.lights.kind.point') }}</button>
      <button type="button" class="btn" @click="add('area')">+ {{ t('scene.lights.kind.area') }}</button>
    </div>
    <p v-if="model.lights.length === 0" class="muted">{{ t('scene.lights.none') }}</p>
    <ul v-else class="scene-objects__list">
      <li v-for="light in model.lights" :key="light.id">
        <button
          type="button"
          class="btn btn-quiet"
          :aria-pressed="selected === light.id ? 'true' : 'false'"
          @click="selected = light.id"
        >
          {{ t(`scene.lights.kind.${light.kind}`) }} #{{ light.id }}
        </button>
        <button type="button" class="btn btn-quiet" @click="remove(light.id)">{{ t('scene.lights.remove') }}</button>
      </li>
    </ul>

    <template v-for="light in model.lights" :key="`form-${light.id}`">
      <div v-if="selected === light.id" class="scene-form__grid">
        <NumberField :id="`light-${light.id}-x`" :label="t('scene.form.x')" v-model="light.x" :step="0.1" />
        <NumberField :id="`light-${light.id}-y`" :label="t('scene.form.y')" v-model="light.y" :step="0.1" />
        <NumberField :id="`light-${light.id}-z`" :label="t('scene.form.z')" v-model="light.z" :step="0.1" />
        <NumberField :id="`light-${light.id}-i`" :label="t('scene.lights.intensity')" v-model="light.intensity" :min="0" :step="10" />
        <NumberField
          v-if="light.kind === 'area'"
          :id="`light-${light.id}-r`"
          :label="t('scene.lights.radius')"
          v-model="light.radius"
          :min="0.01"
          :step="0.05"
        />
        <ColorField :id="`light-${light.id}-c`" :label="t('scene.lights.colour')" v-model="light.color" />
      </div>
    </template>
  </section>
</template>
