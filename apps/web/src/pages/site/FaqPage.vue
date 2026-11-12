<script setup lang="ts">
import { computed } from 'vue'
import { messages } from '@/i18n'

const faq = computed(() => messages().site.faqPage)
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <h1>{{ faq.title }}</h1>
      <p class="site-lede">{{ faq.lede }}</p>
    </section>

    <section class="site-section site-container" aria-labelledby="faq-list">
      <h2 id="faq-list" class="sr-only">{{ faq.title }}</h2>
      <div class="site-faq">
        <details v-for="item in faq.items" :key="item.question">
          <summary>{{ item.question }}</summary>
          <p>{{ item.answer }}</p>
        </details>
      </div>
    </section>
  </div>
</template>
