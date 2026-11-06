<script setup lang="ts">
import { computed } from 'vue'
import { messages } from '@/i18n'

// Long-form pages (privacy, legal notice, accessibility statement) share one layout.
const props = defineProps<{ page: 'privacy' | 'legal' | 'accessibility' }>()

const content = computed(() => messages()[props.page])
</script>

<template>
  <article class="content">
    <h1 tabindex="-1">{{ content.title }}</h1>
    <section v-for="(section, index) in content.sections" :key="index">
      <h2>{{ section.heading }}</h2>
      <p v-for="(paragraph, step) in section.paragraphs" :key="step">{{ paragraph }}</p>
    </section>
  </article>
</template>
