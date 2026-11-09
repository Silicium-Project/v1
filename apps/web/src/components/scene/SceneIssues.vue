<script setup lang="ts">
import { t } from '@/i18n'
import type { EngineIssue } from '@/scene/engine'

// Les problèmes relevés par le moteur, avec leur ligne : ce qui sera ignoré ou refusé.
defineProps<{ issues: EngineIssue[] }>()
</script>

<template>
  <ul v-if="issues.length > 0" class="scene-issues" :aria-label="t('scene.issues.title')">
    <li v-for="(issue, index) in issues" :key="index" :class="`scene-issues__${issue.severity}`">
      <strong>{{ issue.severity === 'error' ? t('scene.issues.error') : t('scene.issues.warning') }}</strong>
      <span v-if="issue.line > 0"> {{ t('scene.issues.line', { line: issue.line }) }}</span>
      : {{ issue.message }}
      <em v-if="issue.suggestion"> ({{ t('scene.issues.suggestion', { word: issue.suggestion }) }})</em>
    </li>
  </ul>
</template>
