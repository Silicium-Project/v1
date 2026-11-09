<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { t } from '@/i18n'
import { sceneToCfg } from '@/scene/cfg'
import { checkScene, type EngineReport } from '@/scene/engine'
import { sceneTemplate, SCENE_TEMPLATES, type SceneTemplate } from '@/scene/templates'
import type { SceneModel } from '@/scene/types'
import SceneIssues from './SceneIssues.vue'
import SceneLights from './SceneLights.vue'
import SceneObjectForm from './SceneObjectForm.vue'
import SceneObjects from './SceneObjects.vue'
import ScenePreview from './ScenePreview.vue'
import SceneSettings from './SceneSettings.vue'

// L'éditeur de scènes : le modèle se modifie en place (le parent le possède) ; l'éditeur en tire
// le fichier .cfg, les problèmes relevés par le moteur, et un aperçu rendu par le moteur.
const props = defineProps<{ model: SceneModel }>()

type Tab = 'objects' | 'lights' | 'settings'
const tab = ref<Tab>('objects')
const selectedId = ref<number | null>(null)
const report = ref<EngineReport | null>(null)
const checkError = ref('')
const TABS: Tab[] = ['objects', 'lights', 'settings']
let timer: ReturnType<typeof setTimeout> | undefined
let request = 0

const selectedObject = () => props.model.objects.find((object) => object.id === selectedId.value)

function removeSelected(): void {
  const index = props.model.objects.findIndex((object) => object.id === selectedId.value)
  if (index >= 0) props.model.objects.splice(index, 1)
  selectedId.value = null
}

/** Remplace le contenu du modèle par un modèle prêt à l'emploi (la copie reste celle de l'éditeur). */
function loadTemplate(name: string): void {
  if (!SCENE_TEMPLATES.includes(name as SceneTemplate)) return
  Object.assign(props.model, sceneTemplate(name as SceneTemplate))
  selectedId.value = null
}

async function check(): Promise<void> {
  const current = ++request
  try {
    const found = await checkScene(sceneToCfg(props.model, t('scene.editor.title')))
    if (current === request) {
      report.value = found
      checkError.value = ''
    }
  } catch (failure) {
    if (current === request) checkError.value = failure instanceof Error ? failure.message : String(failure)
  }
}

watch(
  () => props.model,
  () => {
    clearTimeout(timer)
    timer = setTimeout(check, 400)
  },
  { deep: true, immediate: true },
)

onBeforeUnmount(() => clearTimeout(timer))
</script>

<template>
  <div class="scene-editor">
    <div class="field">
      <label for="scene-template">{{ t('scene.templates.label') }}</label>
      <select id="scene-template" class="input" @change="loadTemplate(($event.target as HTMLSelectElement).value)">
        <option value="" selected disabled>{{ t('scene.templates.choose') }}</option>
        <option v-for="name in SCENE_TEMPLATES" :key="name" :value="name">{{ t(`scene.templates.${name}`) }}</option>
      </select>
    </div>

    <div class="scene-editor__body">
      <div class="scene-editor__controls scene-panel">
        <div class="scene-editor__tabs" role="tablist" :aria-label="t('scene.editor.tabs')">
          <button
            v-for="name in TABS"
            :key="name"
            type="button"
            role="tab"
            class="btn btn-quiet"
            :aria-selected="tab === name ? 'true' : 'false'"
            @click="tab = name"
          >
            {{ t(`scene.tabs.${name}`) }}
          </button>
        </div>

        <div v-if="tab === 'objects'">
          <SceneObjects :model="model" :selected-id="selectedId" @select="selectedId = $event" />
          <SceneObjectForm
            v-if="selectedObject()"
            :key="selectedId ?? 0"
            :object="selectedObject()!"
            @delete="removeSelected"
          />
          <p v-else-if="model.objects.length > 0" class="muted scene-editor__hint">{{ t('scene.form.none') }}</p>
        </div>
        <SceneLights v-else-if="tab === 'lights'" :model="model" />
        <SceneSettings v-else :model="model" />
      </div>

      <div class="scene-editor__preview scene-panel">
        <ScenePreview :model="model" />
        <SceneIssues v-if="report" :issues="report.issues" />
        <p v-if="checkError" class="field__error" role="alert">{{ checkError }}</p>
        <details class="scene-editor__text">
          <summary>{{ t('scene.text.title') }}</summary>
          <pre><code>{{ sceneToCfg(model, t('scene.editor.title')) }}</code></pre>
        </details>
      </div>
    </div>
  </div>
</template>
