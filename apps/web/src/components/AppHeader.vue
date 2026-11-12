<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { t, locale, setLocale, type Locale } from '@/i18n'
import { hasSession, clearSession, currentToken } from '@/state/session'
import { logout } from '@/api/endpoints'
import BrandMark from '@/components/BrandMark.vue'

// En-tête standard : marque, navigation du site, langue, puis compte. Les liens du compte sont
// dans un menu « Mon espace » (pas une deuxième barre). Sur petit écran, un bouton ouvre le menu.
const router = useRouter()
const route = useRoute()

const languages: ReadonlyArray<{ code: Locale; name: string }> = [
  { code: 'fr', name: 'Français' },
  { code: 'en', name: 'English' },
]

const menuOpen = ref(false)
const accountOpen = ref(false)
const header = ref<HTMLElement | null>(null)

function closeMenus(): void {
  menuOpen.value = false
  accountOpen.value = false
}

// Fermer les menus au changement de page, à la touche Échap et au clic en dehors.
watch(
  () => route.fullPath,
  () => closeMenus(),
)

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') closeMenus()
}

function onDocumentClick(event: MouseEvent): void {
  if (header.value && event.target instanceof Node && !header.value.contains(event.target)) {
    closeMenus()
  }
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown)
  document.addEventListener('click', onDocumentClick)
})
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown)
  document.removeEventListener('click', onDocumentClick)
})

async function signOut(): Promise<void> {
  const token = currentToken()
  clearSession()
  closeMenus()
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
  <header ref="header" class="site-header">
    <div class="site-header__inner">
      <RouterLink to="/" class="brand brand-link">
        <BrandMark :size="30" />
      </RouterLink>

      <nav :aria-label="t('site.nav.label')" id="primary-nav-list" class="primary-nav" :class="{ 'primary-nav--open': menuOpen }">
        <ul>
          <li><RouterLink to="/produit">{{ t('site.nav.product') }}</RouterLink></li>
          <li><RouterLink to="/types-de-calcul">{{ t('site.nav.workloads') }}</RouterLink></li>
          <li><RouterLink to="/blockchain">{{ t('site.nav.chain') }}</RouterLink></li>
          <li><RouterLink to="/installation">{{ t('site.nav.install') }}</RouterLink></li>
          <li><RouterLink to="/reseau">{{ t('site.nav.network') }}</RouterLink></li>
          <li><RouterLink to="/a-propos">{{ t('site.nav.about') }}</RouterLink></li>
          <li v-if="!hasSession()" class="nav-cta-item">
            <RouterLink to="/soumettre" class="btn btn-primary">{{ t('site.home.submit') }}</RouterLink>
          </li>
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

        <div v-if="hasSession()" class="account-menu">
          <button
            type="button"
            class="btn btn-quiet account-menu__button"
            aria-controls="account-menu-list"
            :aria-expanded="accountOpen ? 'true' : 'false'"
            @click.stop="accountOpen = !accountOpen"
          >
            {{ t('common.account') }}
          </button>
          <ul v-show="accountOpen" id="account-menu-list" class="account-menu__list" :aria-label="t('common.account')">
            <li><RouterLink to="/soumettre">{{ t('nav.submit') }}</RouterLink></li>
            <li><RouterLink to="/calculs">{{ t('nav.jobs') }}</RouterLink></li>
            <li><RouterLink to="/machines">{{ t('nav.devices') }}</RouterLink></li>
            <li><RouterLink to="/compte">{{ t('nav.account') }}</RouterLink></li>
            <li><RouterLink to="/profil">{{ t('nav.profile') }}</RouterLink></li>
            <li><RouterLink to="/parametres">{{ t('nav.settings') }}</RouterLink></li>
            <li><button type="button" class="account-menu__logout" @click="signOut">{{ t('common.logout') }}</button></li>
          </ul>
        </div>
        <RouterLink v-else to="/connexion" class="btn btn-quiet header-login">{{ t('nav.login') }}</RouterLink>

        <button
          type="button"
          class="menu-toggle"
          aria-controls="primary-nav-list"
          :aria-expanded="menuOpen ? 'true' : 'false'"
          @click.stop="menuOpen = !menuOpen"
        >
          {{ menuOpen ? t('common.closeMenu') : t('common.menu') }}
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.site-header__inner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem 1.5rem;
  flex-wrap: nowrap;
}

.primary-nav {
  flex: 1 1 auto;
  min-width: 0;
}

.primary-nav ul {
  display: flex;
  justify-content: center;
  flex-wrap: nowrap;
  gap: 0.25rem 1rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

/* Le bouton d'appel n'est présent que dans le menu replié. */
.nav-cta-item {
  display: none;
}

.primary-nav a {
  white-space: nowrap;
  color: var(--color-text);
  text-decoration: none;
  padding: 0.35rem 0;
}

.primary-nav a.router-link-active {
  color: var(--color-accent);
  font-weight: 600;
  box-shadow: inset 0 -2px 0 var(--color-accent);
}

.header-tools {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex: 0 0 auto;
}

.account-menu {
  position: relative;
}

.account-menu__list {
  position: absolute;
  right: 0;
  top: calc(100% + 0.4rem);
  z-index: 20;
  min-width: 13rem;
  display: grid;
  gap: 0.1rem;
  margin: 0;
  padding: 0.4rem;
  list-style: none;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  box-shadow: var(--shadow-card);
}

.account-menu__list a,
.account-menu__logout {
  display: block;
  width: 100%;
  padding: 0.5rem 0.6rem;
  border: 0;
  border-radius: 0.5rem;
  background: none;
  color: var(--color-text);
  font: inherit;
  text-align: left;
  text-decoration: none;
  cursor: pointer;
}

.account-menu__list a:hover,
.account-menu__logout:hover {
  background: var(--color-accent-soft);
}

.header-login {
  white-space: nowrap;
}

.menu-toggle {
  display: none;
  min-height: var(--touch-target);
  padding: 0 0.8rem;
  border: 1px solid var(--color-input-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: var(--color-text);
  font: inherit;
  cursor: pointer;
}

/* Navigation en ligne seulement si elle tient : en dessous de 1280 px, elle se replie derrière Menu. */
@media (max-width: 1280px) {
  .site-header__inner {
    flex-wrap: wrap;
  }

  .menu-toggle {
    display: inline-flex;
    align-items: center;
  }

  .nav-cta-item {
    display: block;
    margin-top: 0.5rem;
  }

  .primary-nav {
    display: none;
    order: 3;
    flex-basis: 100%;
  }

  .primary-nav--open {
    display: block;
  }

  .primary-nav ul {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.2rem;
    padding: 0.5rem 0 0.25rem;
  }

  .primary-nav a {
    display: block;
    padding: 0.6rem 0;
  }

  .primary-nav a.btn {
    display: inline-flex;
    width: auto;
    margin-top: 0.4rem;
    padding: 0.6rem 1.1rem;
    color: var(--color-accent-text);
    box-shadow: none;
  }
}
</style>
