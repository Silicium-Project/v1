<script setup lang="ts">
import { t } from '@/i18n'
</script>

<template>
  <footer class="site-footer">
    <div class="site-footer__inner">
      <p class="muted">{{ t('app.name') }} — {{ t('app.tagline') }}</p>
      <nav :aria-label="t('footer.navLabel')">
        <ul class="footer-links">
          <li><RouterLink to="/confidentialite">{{ t('footer.privacy') }}</RouterLink></li>
          <li><RouterLink to="/mentions-legales">{{ t('footer.legal') }}</RouterLink></li>
          <li><RouterLink to="/accessibilite">{{ t('footer.accessibility') }}</RouterLink></li>
        </ul>
      </nav>
    </div>
  </footer>
</template>
