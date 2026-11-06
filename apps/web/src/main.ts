import '@fontsource/geist/400.css'
import '@fontsource/geist/600.css'
import '@fontsource/geist-mono/400.css'
import './styles/tokens.css'
import './styles/base.css'
import { createApp } from 'vue'
import App from './App.vue'
import { initLocale } from './i18n'
import { router } from './router'

initLocale()
createApp(App).use(router).mount('#app')
