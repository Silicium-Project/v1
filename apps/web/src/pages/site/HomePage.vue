<script setup lang="ts">
import { computed } from 'vue'
import { messages, t } from '@/i18n'
import { SITE_CELLS } from './mosaic'

const home = computed(() => messages().site.home)
const cells = SITE_CELLS
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container">
      <div class="site-hero__copy">
        <p class="site-kicker">{{ home.kicker }}</p>
        <h1>{{ home.title }}</h1>
        <p class="site-lede">{{ home.lede }}</p>
        <div class="site-actions">
          <RouterLink class="btn btn-primary" to="/soumettre">{{ home.submit }}</RouterLink>
          <RouterLink class="btn" to="/produit">{{ home.how }}</RouterLink>
        </div>
        <p class="site-note">{{ t('site.testnet') }}</p>
      </div>
      <div class="site-mosaic" aria-hidden="true">
        <span
          v-for="(filled, index) in cells"
          :key="index"
          class="site-mosaic__cell"
          :class="{ 'site-mosaic__cell--on': filled }"
        ></span>
      </div>
    </section>

    <section class="site-section site-container" aria-labelledby="home-uses">
      <p class="site-kicker">{{ home.usesKicker }}</p>
      <h2 id="home-uses">{{ home.usesTitle }}</h2>
      <ul class="site-cards">
        <li v-for="item in home.uses" :key="item.tag" class="site-card">
          <h3>{{ item.tag }}</h3>
          <p>{{ item.text }}</p>
        </li>
      </ul>
    </section>

    <section class="site-section site-container" aria-labelledby="home-flow">
      <p class="site-kicker">{{ home.processKicker }}</p>
      <h2 id="home-flow">{{ home.processTitle }}</h2>
      <ol class="site-steps">
        <li v-for="(step, index) in home.steps" :key="step.title">
          <span class="site-steps__number">{{ index + 1 }}</span>
          <h3>{{ step.title }}</h3>
          <p>{{ step.text }}</p>
        </li>
      </ol>
      <p class="site-inline-link">
        <RouterLink to="/blockchain">{{ home.chainLink }}</RouterLink>
      </p>
    </section>

    <section class="site-section site-container site-band" aria-labelledby="home-worker">
      <div>
        <p class="site-kicker">{{ home.workerKicker }}</p>
        <h2 id="home-worker">{{ home.workerTitle }}</h2>
        <p>{{ home.workerText }}</p>
      </div>
      <RouterLink class="btn" to="/installation">{{ home.workerLink }}</RouterLink>
    </section>

    <section class="site-section site-container" aria-labelledby="home-faq">
      <h2 id="home-faq">{{ home.faqKicker }}</h2>
      <div class="site-faq">
        <details v-for="item in home.faq" :key="item.question">
          <summary>{{ item.question }}</summary>
          <p>{{ item.answer }}</p>
        </details>
      </div>
    </section>
  </div>
</template>
