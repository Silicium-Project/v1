<script setup lang="ts">
import { t } from '@/i18n'
import BrandMark from '@/components/BrandMark.vue'
</script>

<template>
  <footer class="site-footer">
    <div class="site-footer__inner">
      <div class="site-footer__brand">
        <BrandMark :size="26" />
        <p class="muted">{{ t('app.tagline') }}</p>
      </div>
      <nav :aria-label="t('footer.navLabel')">
        <ul class="footer-links">
          <li><RouterLink to="/types-de-calcul">{{ t('footer.workloads') }}</RouterLink></li>
          <li><RouterLink to="/a-propos#faq">{{ t('footer.faq') }}</RouterLink></li>
          <li><RouterLink to="/contact">{{ t('footer.contact') }}</RouterLink></li>
          <li><RouterLink to="/confidentialite">{{ t('footer.privacy') }}</RouterLink></li>
          <li><RouterLink to="/mentions-legales">{{ t('footer.legal') }}</RouterLink></li>
          <li><RouterLink to="/accessibilite">{{ t('footer.accessibility') }}</RouterLink></li>
        </ul>
      </nav>
    </div>
  </footer>
</template>
