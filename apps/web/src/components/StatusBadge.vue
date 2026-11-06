<script setup lang="ts">
import { computed } from 'vue'
import { t } from '@/i18n'
import { stateLabelKey, stateTone, type Tone } from '@/status'

// The state is always written out as text. The colour and the icon only reinforce it, so
// the status never depends on colour alone (WCAG 1.4.1).
const props = defineProps<{ state: string }>()

const label = computed(() => t(stateLabelKey(props.state)))
const tone = computed(() => stateTone(props.state))

const icons: Record<Tone, string> = {
  info: 'M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Zm0 2a4.5 4.5 0 1 1 0 9 4.5 4.5 0 0 1 0-9Z',
  success: 'M6.4 11.2 3.2 8l1.1-1.1 2.1 2.1 5.2-5.2 1.1 1.1Z',
  neutral: 'M4 7.25h8v1.5H4z',
  warning: 'M7.25 3h1.5v6.5h-1.5zM7.25 11h1.5v1.5h-1.5z',
}
</script>

<template>
  <span class="badge" :class="`badge--${tone}`">
    <svg class="badge__icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <path :d="icons[tone]" />
    </svg>
    <span>{{ label }}</span>
  </span>
</template>
