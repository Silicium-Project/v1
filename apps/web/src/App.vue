<script setup lang="ts">
import { t } from '@/i18n'
import AppHeader from '@/components/AppHeader.vue'
import AppFooter from '@/components/AppFooter.vue'

function focusMain(event: Event): void {
  event.preventDefault()
  const main = document.getElementById('main')
  main?.setAttribute('tabindex', '-1')
  main?.focus()
}
</script>

<template>
  <a class="skip-link" href="#main" @click="focusMain">{{ t('common.skipToContent') }}</a>
  <AppHeader />
  <main id="main" class="main" tabindex="-1">
    <RouterView />
  </main>
  <AppFooter />
</template>
