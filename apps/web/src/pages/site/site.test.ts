import { describe, expect, it } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createMemoryHistory } from 'vue-router'
import axe from 'axe-core'
import { createAppRouter } from '@/router'
import { setLocale, type Locale } from '@/i18n'
import HomePage from './HomePage.vue'
import ProductPage from './ProductPage.vue'
import ChainPage from './ChainPage.vue'
import InstallPage from './InstallPage.vue'
import AboutPage from './AboutPage.vue'

// Contrast is verified by styles/contrast.test.ts (happy-dom has no layout engine).
const RULES = { 'color-contrast': { enabled: false } }

const PAGES = [
  ['home', HomePage],
  ['product', ProductPage],
  ['chain', ChainPage],
  ['install', InstallPage],
  ['about', AboutPage],
] as const

async function mountPage(component: object, locale: Locale) {
  setLocale(locale)
  const router = createAppRouter(createMemoryHistory())
  await router.push('/')
  await router.isReady()
  const wrapper = mount(component, { attachTo: document.body, global: { plugins: [router] } })
  await flushPromises()
  return wrapper
}

describe('public pages', () => {
  it.each(
    PAGES.flatMap(([name]) => (['fr', 'en'] as const).map((locale) => [name, locale] as const)),
  )('%s page in %s has one heading and no axe violation', async (name, locale) => {
    const component = PAGES.find(([page]) => page === name)![1]
    const wrapper = await mountPage(component, locale)
    expect(wrapper.findAll('h1')).toHaveLength(1)
    const result = await axe.run(wrapper.element, { rules: RULES })
    expect(result.violations.map((v) => `${v.id}: ${v.help}`)).toEqual([])
    wrapper.unmount()
  })

  it('home page links to the product, the chain and the install page', async () => {
    const wrapper = await mountPage(HomePage, 'fr')
    const hrefs = wrapper.findAll('a').map((a) => a.attributes('href'))
    expect(hrefs).toEqual(expect.arrayContaining(['/produit', '/blockchain', '/installation']))
    wrapper.unmount()
  })

  it('install page shows the build command from the user docs', async () => {
    const wrapper = await mountPage(InstallPage, 'en')
    expect(wrapper.text()).toContain('cargo build --release -p sil-node')
    wrapper.unmount()
  })
})
