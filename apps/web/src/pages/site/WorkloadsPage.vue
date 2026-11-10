<script setup lang="ts">
import { computed } from 'vue'
import { messages } from '@/i18n'

const page = computed(() => messages().site.workloadsPage)
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <p class="site-kicker">{{ page.kicker }}</p>
      <h1 id="workloads-title" tabindex="-1">{{ page.title }}</h1>
      <p class="site-lede">{{ page.lede }}</p>
    </section>

    <section
      v-for="item in page.items"
      :key="item.id"
      class="site-section site-container workload"
      :aria-labelledby="`workload-${item.id}`"
    >
      <h2 :id="`workload-${item.id}`">{{ item.title }}</h2>
      <p class="site-lede">{{ item.summary }}</p>

      <dl class="workload__facts">
        <div>
          <dt>{{ page.labels.input }}</dt>
          <dd>{{ item.input }}</dd>
        </div>
        <div>
          <dt>{{ page.labels.options }}</dt>
          <dd>{{ item.options }}</dd>
        </div>
        <div>
          <dt>{{ page.labels.output }}</dt>
          <dd>{{ item.output }}</dd>
        </div>
        <div>
          <dt>{{ page.labels.verify }}</dt>
          <dd>{{ item.verify }}</dd>
        </div>
        <div>
          <dt>{{ page.labels.limits }}</dt>
          <dd>{{ item.limits }}</dd>
        </div>
      </dl>

      <div class="workload__actions">
        <a class="btn" :href="item.example.href" :download="item.example.file">
          {{ page.labels.download }} · {{ item.example.file }}
        </a>
        <RouterLink class="btn btn-primary" to="/soumettre">{{ page.labels.submit }}</RouterLink>
      </div>
    </section>
  </div>
</template>

<style scoped>
.workload__facts {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
  gap: 1rem;
  margin: 1.5rem 0;
}

.workload__facts > div {
  padding: 1rem 1.25rem;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-card);
  box-shadow: var(--shadow-card);
}

.workload__facts dt {
  font-weight: 600;
  margin-bottom: 0.25rem;
}

.workload__facts dd {
  margin: 0;
  color: var(--color-text-muted);
}

.workload__warning {
  color: var(--color-warning);
  font-weight: 600;
}

.workload__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}
</style>
