<script setup lang="ts">
import { computed } from 'vue'

// Line of the online count over the samples taken since the page was opened, on a light grid.
const props = defineProps<{ values: number[]; summary: string }>()

const WIDTH = 640
const HEIGHT = 180
const PAD = 16
const GRID = [0.25, 0.5, 0.75]

const scaled = computed(() => {
  const values = props.values
  const top = Math.max(1, ...values)
  const step = values.length > 1 ? (WIDTH - 2 * PAD) / (values.length - 1) : 0
  return values.map((value, index) => ({
    x: PAD + index * step,
    y: HEIGHT - PAD - (value / top) * (HEIGHT - 2 * PAD),
  }))
})
const points = computed(() => scaled.value.map((point) => `${point.x},${point.y}`).join(' '))
const last = computed(() => (scaled.value.length > 0 ? scaled.value[scaled.value.length - 1] : null))
const gridLines = GRID.map((ratio) => HEIGHT - PAD - ratio * (HEIGHT - 2 * PAD))
</script>

<template>
  <svg class="chart chart--line" :viewBox="`0 0 ${WIDTH} ${HEIGHT}`" role="img" :aria-label="summary">
    <line
      v-for="(y, index) in gridLines"
      :key="index"
      :x1="PAD"
      :x2="WIDTH - PAD"
      :y1="y"
      :y2="y"
      class="chart__grid"
    />
    <line :x1="PAD" :x2="WIDTH - PAD" :y1="HEIGHT - PAD" :y2="HEIGHT - PAD" class="chart__axis" />
    <polyline v-if="scaled.length > 1" :points="points" class="chart__line" />
    <circle v-if="last" :cx="last.x" :cy="last.y" r="5" class="chart__dot chart__dot--on" />
  </svg>
</template>
