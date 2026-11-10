<script setup lang="ts">
import { computed } from 'vue'
import { messages, t } from '@/i18n'
import LiveNetwork from '@/components/network/LiveNetwork.vue'

const home = computed(() => messages().site.home)
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
          <RouterLink class="btn" to="/installation">{{ home.installCta }}</RouterLink>
        </div>
        <p class="site-note">{{ t('site.testnet') }}</p>
      </div>
      <div class="site-hero__panel">
        <LiveNetwork />
      </div>
    </section>

    <section class="site-section site-container" aria-labelledby="home-values">
      <p class="site-kicker">{{ home.valuesKicker }}</p>
      <h2 id="home-values">{{ home.valuesTitle }}</h2>
      <ul class="site-values">
        <li v-for="item in home.values" :key="item.title" class="site-value">
          <h3>{{ item.title }}</h3>
          <p>{{ item.text }}</p>
        </li>
      </ul>
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
      <p class="site-inline-link">
        <RouterLink to="/produit">{{ home.processTitle }}</RouterLink>
        ·
        <RouterLink to="/blockchain">{{ home.chainLink }}</RouterLink>
      </p>
    </section>

    <section class="site-section site-container site-band" aria-labelledby="home-worker">
      <div>
        <p class="site-kicker">{{ home.workerKicker }}</p>
        <h2 id="home-worker">{{ home.workerTitle }}</h2>
        <p>{{ home.workerText }}</p>
      </div>
      <RouterLink class="btn btn-primary" to="/installation">{{ home.workerLink }}</RouterLink>
    </section>

    <section class="site-section site-container" aria-labelledby="home-faq">
      <h2 id="home-faq">{{ home.faqKicker }}</h2>
      <div class="site-faq">
        <details v-for="item in home.faq" :key="item.question">
          <summary>{{ item.question }}</summary>
          <p>{{ item.answer }}</p>
        </details>
      </div>
      <p class="site-inline-link">
        <RouterLink to="/a-propos#faq">{{ messages().site.faqPage.title }}</RouterLink>
      </p>
    </section>
  </div>
</template>
