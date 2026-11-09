import '@fontsource/geist/400.css'
import '@fontsource/geist/600.css'
import '@fontsource/geist-mono/400.css'
import '@fontsource-variable/fraunces'
import './styles/tokens.css'
import './styles/base.css'
import './styles/site.css'
import './styles/site-content.css'
import './styles/scene.css'
import './styles/account.css'
import './styles/jobs.css'
import './styles/network.css'
import { createApp } from 'vue'
import App from './App.vue'
import { initLocale } from './i18n'
import { router } from './router'

initLocale()
createApp(App).use(router).mount('#app')
