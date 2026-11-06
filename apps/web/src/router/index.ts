import { nextTick } from 'vue'
import { createRouter, createWebHistory, type RouteRecordRaw, type RouterHistory } from 'vue-router'
import { t } from '@/i18n'
import { clearSession, hasSession } from '@/state/session'
import { safeNext } from './safe-next'

// Pages load on demand: the first screen only downloads what it shows.
export const routes: RouteRecordRaw[] = [
  { path: '/', redirect: '/calculs' },
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
    path: '/travail',
    name: 'worker',
    component: () => import('@/pages/WorkerPage.vue'),
    meta: { auth: true, title: 'worker.title' },
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
