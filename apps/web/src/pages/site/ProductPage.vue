<script setup lang="ts">
import { computed } from 'vue'
import { messages } from '@/i18n'

const product = computed(() => messages().site.product)
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <h1>{{ product.title }}</h1>
      <p class="site-lede">{{ product.lede }}</p>
    </section>

    <section class="site-section site-container" aria-labelledby="product-process">
      <h2 id="product-process">{{ product.processTitle }}</h2>
      <ol class="site-flow">
        <li v-for="(step, index) in product.processSteps" :key="step.title" class="site-flow__step">
          <span class="site-flow__number" aria-hidden="true">{{ index + 1 }}</span>
          <h3>{{ step.title }}</h3>
          <p>{{ step.text }}</p>
        </li>
      </ol>
    </section>

    <section class="site-section site-container" aria-labelledby="product-ledger">
      <h2 id="product-ledger">{{ product.ledgerTitle }}</h2>
      <div class="site-ledger">
        <dl>
          <div v-for="row in product.ledgerRows" :key="row.label" class="site-ledger__row">
            <dt>{{ row.label }}</dt>
            <dd>{{ row.text }}</dd>
          </div>
        </dl>
        <p class="site-ledger__note">{{ product.ledgerNote }}</p>
      </div>
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
