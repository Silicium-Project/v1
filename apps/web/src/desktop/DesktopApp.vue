<script setup lang="ts">
import { onMounted } from 'vue'
import { RouterLink, RouterView, useRouter } from 'vue-router'
import { t } from '@/i18n'
import { watchLinkRequests } from '@/desktop/pending-link'

// The desktop application shell: the node's dashboard and machine linking, nothing else. The
// public site (jobs, submissions, accounts) is not part of this application.
const router = useRouter()

onMounted(() => {
  // A link from the website opens the machine page with its confirmation.
  void watchLinkRequests(() => {
    void router.push('/machine')
  })
})

function focusMain(event: Event): void {
  event.preventDefault()
  const main = document.getElementById('main')
  main?.setAttribute('tabindex', '-1')
  main?.focus()
}
</script>

<template>
  <a class="skip-link" href="#main" @click="focusMain">{{ t('common.skipToContent') }}</a>
  <header class="desktop-header">
    <p class="desktop-brand">{{ t('desktop.appTitle') }}</p>
    <nav :aria-label="t('desktop.navLabel')">
      <ul class="desktop-nav">
        <li><RouterLink to="/">{{ t('desktop.navWorker') }}</RouterLink></li>
        <li><RouterLink to="/machine">{{ t('desktop.navMachine') }}</RouterLink></li>
      </ul>
    </nav>
  </header>
  <main id="main" class="main" tabindex="-1">
    <RouterView />
  </main>
</template>
