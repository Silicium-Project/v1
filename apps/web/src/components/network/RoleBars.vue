<script setup lang="ts">
import { computed } from 'vue'

// Horizontal bars, one per role. The values are also printed, so the chart is not the only source.
const props = defineProps<{ rows: Array<{ label: string; value: number }>; summary: string }>()

const BAR_MAX = 240
const largest = computed(() => Math.max(1, ...props.rows.map((row) => row.value)))
</script>

<template>
  <svg class="chart chart--bars" viewBox="0 0 320 130" role="img" :aria-label="summary">
    <g v-for="(row, index) in props.rows" :key="row.label" :transform="`translate(0 ${index * 40})`">
      <text x="0" y="14" class="chart__label">{{ row.label }}</text>
      <rect x="0" y="20" :width="Math.max(4, (row.value / largest) * BAR_MAX)" height="12" rx="6" class="chart__bar" />
      <text :x="Math.max(4, (row.value / largest) * BAR_MAX) + 8" y="31" class="chart__value">{{ row.value }}</text>
    </g>
  </svg>
</template>
