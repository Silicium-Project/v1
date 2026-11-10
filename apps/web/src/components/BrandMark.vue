<script setup lang="ts">
import { t } from '@/i18n'

// The Silicium mark: a blue diamond with « Si ». Decorative next to the name, so it is hidden from
// assistive technology; the link or heading that wraps it carries the name.
withDefaults(defineProps<{ size?: number }>(), { size: 32 })
</script>

<template>
  <span class="brand-mark">
    <svg
      class="brand-mark__icon"
      :width="size"
      :height="size"
      viewBox="0 0 40 40"
      aria-hidden="true"
      focusable="false"
    >
      <rect x="9" y="9" width="22" height="22" rx="5" transform="rotate(45 20 20)" fill="currentColor" />
      <text
        x="20"
        y="25"
        text-anchor="middle"
        font-family="Geist, system-ui, sans-serif"
        font-size="13"
        font-weight="700"
        fill="#ffffff"
      >Si</text>
    </svg>
    <span class="brand-mark__name">{{ t('app.name') }}</span>
  </span>
</template>

<style scoped>
.brand-mark {
  display: inline-flex;
  align-items: center;
  gap: 0.55rem;
  color: var(--color-accent);
}

.brand-mark__name {
  color: var(--color-text);
  font-family: var(--font-display);
  font-size: 1.3rem;
  font-weight: 700;
  letter-spacing: -0.02em;
}
</style>
