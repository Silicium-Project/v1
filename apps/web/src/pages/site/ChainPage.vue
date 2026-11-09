<script setup lang="ts">
import { computed } from 'vue'
import { messages, t } from '@/i18n'

const chain = computed(() => messages().site.chain)
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <p class="site-kicker">{{ chain.kicker }}</p>
      <h1>{{ chain.title }}</h1>
      <p class="site-lede">{{ chain.lede }}</p>
      <p class="site-note">{{ t('site.testnet') }}</p>
    </section>

    <section class="site-section site-container site-columns" aria-labelledby="chain-on">
      <div>
        <h2 id="chain-on">{{ chain.onTitle }}</h2>
        <ul class="site-list">
          <li v-for="item in chain.onItems" :key="item">{{ item }}</li>
        </ul>
      </div>
      <div>
        <h2>{{ chain.offTitle }}</h2>
        <ul class="site-list">
          <li v-for="item in chain.offItems" :key="item">{{ item }}</li>
        </ul>
      </div>
    </section>

    <section class="site-section site-container" aria-labelledby="chain-limits">
      <h2 id="chain-limits">{{ chain.limitsTitle }}</h2>
      <ul class="site-list site-list--rules">
        <li v-for="item in chain.limitsItems" :key="item">{{ item }}</li>
      </ul>
    </section>
  </div>
</template>
