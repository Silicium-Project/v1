import '@fontsource/geist/400.css'
import '@fontsource/geist/600.css'
import '@fontsource/geist-mono/400.css'
import '@fontsource-variable/fraunces'
import '@/styles/tokens.css'
import '@/styles/base.css'
import './styles/shell.css'
import './styles/worker.css'
import { createApp } from 'vue'
import { initLocale } from '@/i18n'
import DesktopApp from './DesktopApp.vue'
import { router } from './router'

initLocale()
createApp(DesktopApp).use(router).mount('#app')
