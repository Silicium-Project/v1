<script setup lang="ts">
import { useRouter } from 'vue-router'
import { t, locale, setLocale, type Locale } from '@/i18n'
import { hasSession, clearSession, currentToken } from '@/state/session'
import { logout } from '@/api/endpoints'
import BrandMark from '@/components/BrandMark.vue'

const router = useRouter()

const languages: ReadonlyArray<{ code: Locale; name: string }> = [
  { code: 'fr', name: 'Français' },
  { code: 'en', name: 'English' },
]

async function signOut(): Promise<void> {
  const token = currentToken()
  clearSession()
  if (token) {
    try {
      await logout(token)
    } catch {
      // The token expires on its own; the local session is already gone.
    }
  }
  await router.push({ name: 'login' })
}
</script>

<template>
  <header class="site-header">
    <div class="site-header__inner">
      <RouterLink to="/" class="brand brand-link"><BrandMark :size="30" /></RouterLink>

      <nav :aria-label="t('site.nav.label')" class="nav">
        <ul>
          <li><RouterLink to="/produit">{{ t('site.nav.product') }}</RouterLink></li>
          <li><RouterLink to="/types-de-calcul">{{ t('site.nav.workloads') }}</RouterLink></li>
          <li><RouterLink to="/blockchain">{{ t('site.nav.chain') }}</RouterLink></li>
          <li><RouterLink to="/installation">{{ t('site.nav.install') }}</RouterLink></li>
          <li><RouterLink to="/reseau">{{ t('site.nav.network') }}</RouterLink></li>
          <li><RouterLink to="/a-propos">{{ t('site.nav.about') }}</RouterLink></li>
        </ul>
      </nav>

      <nav v-if="hasSession()" :aria-label="t('nav.main')" class="nav">
        <ul>
          <li><RouterLink to="/soumettre">{{ t('nav.submit') }}</RouterLink></li>
          <li><RouterLink to="/calculs">{{ t('nav.jobs') }}</RouterLink></li>
          <li><RouterLink to="/machines">{{ t('nav.devices') }}</RouterLink></li>
          <li><RouterLink to="/compte">{{ t('nav.account') }}</RouterLink></li>
          <li><RouterLink to="/profil">{{ t('nav.profile') }}</RouterLink></li>
          <li><RouterLink to="/parametres">{{ t('nav.settings') }}</RouterLink></li>
        </ul>
      </nav>
      <nav v-else :aria-label="t('nav.main')" class="nav">
        <ul>
          <li><RouterLink to="/connexion">{{ t('nav.login') }}</RouterLink></li>
        </ul>
      </nav>

      <div class="header-tools">
        <div class="lang" role="group" :aria-label="t('common.language')">
          <button
            v-for="item in languages"
            :key="item.code"
            type="button"
            class="lang__btn"
            :lang="item.code"
            :aria-pressed="locale === item.code ? 'true' : 'false'"
            @click="setLocale(item.code)"
          >
            {{ item.name }}
          </button>
        </div>
        <button v-if="hasSession()" type="button" class="btn btn-quiet" @click="signOut">
          {{ t('common.logout') }}
        </button>
        <RouterLink v-else class="btn btn-primary header-cta" to="/soumettre">
          {{ t('site.home.submit') }}
        </RouterLink>
      </div>
    </div>
  </header>
</template>
