import { nextTick } from 'vue'
import { createRouter, createWebHistory, type RouteRecordRaw, type RouterHistory } from 'vue-router'
import { t } from '@/i18n'
import { clearSession, hasSession } from '@/state/session'
import { safeNext } from './safe-next'

// Pages load on demand: the first screen only downloads what it shows.
export const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'home',
    component: () => import('@/pages/site/HomePage.vue'),
    meta: { title: 'site.titles.home' },
  },
  {
    path: '/produit',
    name: 'product',
    component: () => import('@/pages/site/ProductPage.vue'),
    meta: { title: 'site.titles.product' },
  },
  {
    path: '/blockchain',
    name: 'chain',
    component: () => import('@/pages/site/ChainPage.vue'),
    meta: { title: 'site.titles.chain' },
  },
  {
    path: '/installation',
    name: 'install',
    component: () => import('@/pages/site/InstallPage.vue'),
    meta: { title: 'site.titles.install' },
  },
  {
    path: '/a-propos',
    name: 'about',
    component: () => import('@/pages/site/AboutPage.vue'),
    meta: { title: 'site.titles.about' },
  },
  {
    path: '/reseau',
    name: 'network',
    component: () => import('@/pages/site/NetworkPage.vue'),
    meta: { title: 'site.titles.network' },
  },
  {
    path: '/connexion',
    name: 'login',
    component: () => import('@/pages/LoginPage.vue'),
    meta: { title: 'auth.loginTitle' },
  },
  {
    path: '/soumettre',
    name: 'submit',
    component: () => import('@/pages/SubmitJobPage.vue'),
    meta: { auth: true, title: 'submit.title' },
  },
  {
    path: '/calculs',
    name: 'jobs',
    component: () => import('@/pages/JobsPage.vue'),
    meta: { auth: true, title: 'jobs.title' },
  },
  {
    path: '/calculs/:id',
    name: 'job',
    component: () => import('@/pages/JobDetailPage.vue'),
    meta: { auth: true, title: 'job.title' },
  },
  {
    path: '/machines',
    name: 'devices',
    component: () => import('@/pages/DevicesPage.vue'),
    meta: { auth: true, title: 'devices.title' },
  },
  {
    path: '/compte',
    name: 'account',
    component: () => import('@/pages/AccountPage.vue'),
    meta: { auth: true, title: 'account.title' },
  },
  {
    path: '/profil',
    name: 'profile',
    component: () => import('@/pages/ProfilePage.vue'),
    meta: { auth: true, title: 'profile.title' },
  },
  {
    path: '/parametres',
    name: 'settings',
    component: () => import('@/pages/SettingsPage.vue'),
    meta: { auth: true, title: 'settings.title' },
  },
  {
    path: '/confidentialite',
    name: 'privacy',
    component: () => import('@/pages/ContentPage.vue'),
    props: { page: 'privacy' },
    meta: { title: 'privacy.title' },
  },
  {
    path: '/mentions-legales',
    name: 'legal',
    component: () => import('@/pages/ContentPage.vue'),
    props: { page: 'legal' },
    meta: { title: 'legal.title' },
  },
  {
    path: '/accessibilite',
    name: 'accessibility',
    component: () => import('@/pages/ContentPage.vue'),
    props: { page: 'accessibility' },
    meta: { title: 'accessibility.title' },
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'notFound',
    component: () => import('@/pages/NotFoundPage.vue'),
    meta: { title: 'notFound.title' },
  },
]

export function createAppRouter(history: RouterHistory = createWebHistory()) {
  const router = createRouter({
    history,
    routes,
    scrollBehavior: (_to, _from, saved) => saved ?? { top: 0 },
  })

  router.beforeEach((to) => {
    if (to.meta.auth && !hasSession()) {
      clearSession() // an expired token is dropped here, outside any render
      return { name: 'login', query: { next: to.fullPath } }
    }
    if (to.name === 'login' && hasSession()) {
      return safeNext(to.query.next)
    }
    return true
  })

  router.afterEach((to) => {
    const title = to.meta.title
    document.title = title ? `${t(title)} — ${t('app.name')}` : t('app.name')
    // Move focus to the page heading so screen readers announce the new page.
    void nextTick(() => {
      const heading = document.querySelector<HTMLElement>('#main h1')
      const target = heading ?? document.getElementById('main')
      target?.setAttribute('tabindex', '-1')
      target?.focus()
    })
  })

  return router
}

export const router = createAppRouter()
