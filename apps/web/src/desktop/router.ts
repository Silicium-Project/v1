import { createRouter, createWebHashHistory } from 'vue-router'
import MachinePage from './pages/MachinePage.vue'
import WorkerPage from './pages/WorkerPage.vue'

// Hash history: the desktop window loads its bundle from a fixed local address, which has no
// route to fall back to on reload.
export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'worker', component: WorkerPage },
    { path: '/machine', name: 'machine', component: MachinePage },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
})
