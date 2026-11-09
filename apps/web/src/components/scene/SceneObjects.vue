<script setup lang="ts">
import { ref } from 'vue'
import { t } from '@/i18n'
import { importObj } from '@/scene/engine'
import { newBox, newId, newMesh, newPanel, newPlane, newSphere } from '@/scene/factory'
import { BUILTIN_MODELS, builtinModel, type BuiltinModel } from '@/scene/models'
import { meshFromBlock } from '@/scene/mesh'
import type { ObjectKind, SceneModel, SceneObject } from '@/scene/types'

// La liste des objets de la scène : ajout (primitives, modèles intégrés, fichier .obj), sélection,
// duplication et suppression.
const props = defineProps<{ model: SceneModel; selectedId: number | null }>()
const emit = defineEmits<{ select: [id: number | null] }>()

const error = ref('')
const objFile = ref<HTMLInputElement | null>(null)
const KINDS: ObjectKind[] = ['sphere', 'plane', 'box', 'panel']

function add(object: SceneObject): void {
  props.model.objects.push(object)
  emit('select', object.id)
}

function addPrimitive(kind: ObjectKind): void {
  const spot = props.model.objects.length
  const offset = { x: 4 + (spot % 3), y: (spot % 5) - 2, z: 0 }
  if (kind === 'sphere') add(newSphere(offset))
  else if (kind === 'plane') add(newPlane({ x: offset.x, y: offset.y, z: -1 }))
  else if (kind === 'box') add(newBox(offset))
  else if (kind === 'panel') add(newPanel(offset))
}

function addBuiltin(name: BuiltinModel): void {
  add(newMesh({ ...builtinModel(name), x: 5, y: 0, z: -1 }))
}

async function importFile(event: Event): Promise<void> {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  error.value = ''
  try {
    // Le moteur lit le .obj (même import que la v0 : centré, z-up, taille 1) ; on en tire le maillage.
    const block = await importObj(await file.text(), 'glossy', 1, 'y')
    add(newMesh({ ...meshFromBlock(block), x: 5, y: 0, z: -1 }))
  } catch (failure) {
    error.value = failure instanceof Error ? failure.message : String(failure)
  } finally {
    if (objFile.value) objFile.value.value = ''
  }
}

function duplicate(object: SceneObject): void {
  const copy = JSON.parse(JSON.stringify(object)) as SceneObject
  add({ ...copy, id: newId(), x: copy.x + 0.5, y: copy.y + 0.5 } as SceneObject)
}

function remove(id: number): void {
  const index = props.model.objects.findIndex((object) => object.id === id)
  if (index >= 0) props.model.objects.splice(index, 1)
  if (props.selectedId === id) emit('select', null)
}

const kindLabel = (kind: ObjectKind) => t(`scene.objects.kind.${kind}`)
</script>

<template>
  <section class="scene-objects" :aria-labelledby="'scene-objects-title'">
    <h3 id="scene-objects-title">{{ t('scene.objects.title') }}</h3>
    <div class="scene-objects__add" role="group" :aria-label="t('scene.objects.add')">
      <button v-for="kind in KINDS" :key="kind" type="button" class="btn" @click="addPrimitive(kind)">
        + {{ kindLabel(kind) }}
      </button>
      <label class="btn">
        {{ t('scene.objects.importObj') }}
        <input
          ref="objFile"
          class="sr-only"
          type="file"
          accept=".obj"
          @change="importFile"
        />
      </label>
    </div>
    <div class="scene-objects__builtin" role="group" :aria-label="t('scene.objects.builtin')">
      <button v-for="name in BUILTIN_MODELS" :key="name" type="button" class="btn btn-quiet" @click="addBuiltin(name)">
        + {{ t(`scene.objects.builtinName.${name}`) }}
      </button>
    </div>
    <p v-if="error" class="field__error" role="alert">{{ error }}</p>
    <p v-if="model.objects.length === 0" class="muted">{{ t('scene.objects.none') }}</p>
    <ul v-else class="scene-objects__list">
      <li
        v-for="object in model.objects"
        :key="object.id"
        class="scene-objects__item"
        :class="{ 'scene-objects__item--selected': selectedId === object.id }"
      >
        <button
          type="button"
          class="btn btn-quiet scene-objects__name"
          :aria-pressed="selectedId === object.id ? 'true' : 'false'"
          @click="emit('select', object.id)"
        >
          {{ kindLabel(object.kind) }} #{{ object.id }}
        </button>
        <button type="button" class="btn btn-quiet" @click="duplicate(object)">{{ t('scene.objects.duplicate') }}</button>
        <button
          type="button"
          class="btn btn-danger"
          :aria-label="`${t('scene.objects.remove')} ${kindLabel(object.kind)} #${object.id}`"
          @click="remove(object.id)"
        >
          {{ t('scene.objects.remove') }}
        </button>
      </li>
    </ul>
  </section>
</template>
