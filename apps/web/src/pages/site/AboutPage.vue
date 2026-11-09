<script setup lang="ts">
import { computed } from 'vue'
import { messages } from '@/i18n'

const about = computed(() => messages().site.about)
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <p class="site-kicker">{{ about.kicker }}</p>
      <h1>{{ about.title }}</h1>
      <p class="site-lede">{{ about.lede }}</p>
    </section>

    <section class="site-section site-container" aria-labelledby="about-principles">
      <h2 id="about-principles">{{ about.principlesTitle }}</h2>
      <ul class="site-cards">
        <li v-for="item in about.principles" :key="item.title" class="site-card">
          <h3>{{ item.title }}</h3>
          <p>{{ item.text }}</p>
        </li>
      </ul>
    </section>

    <section class="site-section site-container" aria-labelledby="about-status">
      <h2 id="about-status">{{ about.statusTitle }}</h2>
      <ul class="site-list site-list--rules">
        <li v-for="item in about.statusItems" :key="item">{{ item }}</li>
      </ul>
    </section>
  </div>
</template>
