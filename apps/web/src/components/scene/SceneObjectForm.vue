<script setup lang="ts">
import { computed } from 'vue'
import { t } from '@/i18n'
import ColorField from './ColorField.vue'
import NumberField from './NumberField.vue'
import { look, materialPreset } from '@/scene/factory'
import {
  MATERIAL_PRESETS,
  PLANE_ORIENTATIONS,
  type MaterialPreset,
  type PlaneOrientation,
  type SceneMaterial,
  type SceneObject,
} from '@/scene/types'

// Les réglages d'un objet sélectionné : position, forme, orientation et matière.
const props = defineProps<{ object: SceneObject }>()
const emit = defineEmits<{ delete: [] }>()

const name = computed(() => `${t(`scene.objects.kind.${props.object.kind}`)} #${props.object.id}`)
const preset = computed(() => materialPreset(props.object.material))
const PRESETS = Object.keys(MATERIAL_PRESETS) as MaterialPreset[]
const ORIENTATIONS = Object.keys(PLANE_ORIENTATIONS) as PlaneOrientation[]

/** L'orientation du plan sélectionné, ou `custom` si son normal n'en est pas une. */
const orientation = computed(() => {
  const object = props.object
  if (object.kind !== 'plane') return 'custom'
  const found = ORIENTATIONS.find((name) =>
    PLANE_ORIENTATIONS[name].every((value, index) => value === object.normal[index]),
  )
  return found ?? 'custom'
})

function applyPreset(name: string): void {
  if (name === 'custom') return
  const settings = MATERIAL_PRESETS[name as MaterialPreset]
  props.object.material = look(props.object.material.color, settings)
}

function applyOrientation(name: string): void {
  if (props.object.kind === 'plane' && name !== 'custom')
    props.object.normal = [...PLANE_ORIENTATIONS[name as PlaneOrientation]]
}

const material = (): SceneMaterial => props.object.material
</script>

<template>
  <section class="scene-form" :aria-labelledby="'scene-form-title'">
    <div class="scene-form__head">
      <h3 id="scene-form-title">{{ t('scene.form.selected', { name }) }}</h3>
      <button type="button" class="btn btn-danger" @click="emit('delete')">{{ t('scene.form.delete') }}</button>
    </div>
    <div class="scene-form__grid">
      <NumberField :id="'obj-x'" :label="t('scene.form.x')" v-model="object.x" :step="0.1" />
      <NumberField :id="'obj-y'" :label="t('scene.form.y')" v-model="object.y" :step="0.1" />
      <NumberField :id="'obj-z'" :label="t('scene.form.z')" v-model="object.z" :step="0.1" />

      <template v-if="object.kind === 'sphere'">
        <NumberField :id="'obj-radius'" :label="t('scene.form.radius')" v-model="object.radius" :min="0.01" :step="0.05" />
      </template>

      <template v-if="object.kind === 'plane'">
        <div class="field">
          <label for="obj-orientation">{{ t('scene.form.orientation') }}</label>
          <select id="obj-orientation" class="input" :value="orientation" @change="applyOrientation(($event.target as HTMLSelectElement).value)">
            <option v-for="name in ORIENTATIONS" :key="name" :value="name">{{ t(`scene.form.orientationName.${name}`) }}</option>
            <option value="custom" disabled>{{ t('scene.form.custom') }}</option>
          </select>
        </div>
      </template>

      <template v-if="object.kind === 'box'">
        <NumberField :id="'obj-sx'" :label="t('scene.form.sizeX')" v-model="object.size[0]" :min="0.01" :step="0.1" />
        <NumberField :id="'obj-sy'" :label="t('scene.form.sizeY')" v-model="object.size[1]" :min="0.01" :step="0.1" />
        <NumberField :id="'obj-sz'" :label="t('scene.form.sizeZ')" v-model="object.size[2]" :min="0.01" :step="0.1" />
        <NumberField :id="'obj-rz'" :label="t('scene.form.turn')" v-model="object.rotation[2]" :step="5" />
      </template>

      <template v-if="object.kind === 'panel'">
        <NumberField :id="'obj-w'" :label="t('scene.form.width')" v-model="object.width" :min="0.01" :step="0.1" />
        <NumberField :id="'obj-h'" :label="t('scene.form.height')" v-model="object.height" :min="0.01" :step="0.1" />
        <NumberField :id="'obj-rz'" :label="t('scene.form.turn')" v-model="object.rotation[2]" :step="5" />
      </template>

      <template v-if="object.kind === 'mesh'">
        <NumberField :id="'obj-size'" :label="t('scene.form.size')" v-model="object.size" :min="0.01" :step="0.1" />
        <NumberField :id="'obj-rz'" :label="t('scene.form.turn')" v-model="object.rotation[2]" :step="5" />
      </template>
    </div>

    <fieldset class="scene-form__material">
      <legend>{{ t('scene.form.material') }}</legend>
      <div class="field">
        <label for="obj-preset">{{ t('scene.form.preset') }}</label>
        <select id="obj-preset" class="input" :value="preset" @change="applyPreset(($event.target as HTMLSelectElement).value)">
          <option v-for="name in PRESETS" :key="name" :value="name">{{ t(`scene.form.presetName.${name}`) }}</option>
          <option value="custom" disabled>{{ t('scene.form.custom') }}</option>
        </select>
      </div>
      <ColorField :id="'obj-color'" :label="t('scene.form.colour')" v-model="object.material.color" />
      <template v-if="material().type === 'phong'">
        <NumberField :id="'obj-reflection'" :label="t('scene.form.reflection')" v-model="object.material.reflection" :min="0" :max="0.95" :step="0.05" />
        <NumberField :id="'obj-specular'" :label="t('scene.form.specular')" v-model="object.material.specular" :min="0" :max="2" :step="0.05" />
        <NumberField :id="'obj-shine'" :label="t('scene.form.shininess')" v-model="object.material.shininess" :min="1" :max="512" :step="1" />
      </template>
      <template v-if="material().type === 'phong' || material().type === 'metal'">
        <NumberField :id="'obj-rough'" :label="t('scene.form.roughness')" v-model="object.material.roughness" :min="0" :max="1" :step="0.05" />
      </template>
      <template v-if="material().type === 'glass'">
        <NumberField :id="'obj-ior'" :label="t('scene.form.ior')" v-model="object.material.ior" :min="1" :max="3" :step="0.05" />
      </template>
      <template v-if="material().type === 'emissive'">
        <NumberField :id="'obj-strength'" :label="t('scene.form.strength')" v-model="object.material.strength" :min="0" :step="0.5" />
      </template>
    </fieldset>
  </section>
</template>
