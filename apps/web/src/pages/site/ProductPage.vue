<script setup lang="ts">
import { computed } from 'vue'
import { messages } from '@/i18n'

const product = computed(() => messages().site.product)
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <p class="site-kicker">{{ product.kicker }}</p>
      <h1>{{ product.title }}</h1>
      <p class="site-lede">{{ product.lede }}</p>
    </section>

    <section class="site-section site-container site-columns" aria-labelledby="product-choose">
      <div>
        <h2 id="product-choose">{{ product.chooseTitle }}</h2>
        <ul class="site-list">
          <li v-for="item in product.chooseItems" :key="item">{{ item }}</li>
        </ul>
      </div>
      <div>
        <h2>{{ product.networkTitle }}</h2>
        <p>{{ product.networkText }}</p>
      </div>
    </section>

    <section class="site-section site-container site-columns" aria-labelledby="product-verify">
      <div>
        <h2 id="product-verify">{{ product.verifyTitle }}</h2>
        <p>{{ product.verifyText }}</p>
      </div>
    </section>
  </div>
</template>
