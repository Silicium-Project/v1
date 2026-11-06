<script setup lang="ts">
import { t } from '@/i18n'
import { hasSession } from '@/state/session'
</script>

<template>
  <section class="stack">
    <h1 tabindex="-1">{{ t('notFound.title') }}</h1>
    <p>{{ t('notFound.text') }}</p>
    <p>
      <RouterLink :to="hasSession() ? '/calculs' : '/connexion'">{{ t('notFound.home') }}</RouterLink>
    </p>
  </section>
</template>
