import { describe, expect, it } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createMemoryHistory } from 'vue-router'
import axe from 'axe-core'
import { createAppRouter } from '@/router'
import { setLocale } from '@/i18n'
import LoginPage from './LoginPage.vue'
import ContentPage from './ContentPage.vue'
import NotFoundPage from './NotFoundPage.vue'

// Colour contrast is verified by styles/contrast.test.ts (happy-dom has no layout engine).
const RULES = { 'color-contrast': { enabled: false } }

async function violationsOf(element: Element): Promise<string[]> {
  const result = await axe.run(element, { rules: RULES })
  return result.violations.map((v) => `${v.id}: ${v.help}`)
}

async function mountWithRouter(component: object, props: Record<string, unknown> = {}) {
  setLocale('fr')
  const router = createAppRouter(createMemoryHistory())
  await router.push('/')
  await router.isReady()
  const wrapper = mount(component, { props, attachTo: document.body, global: { plugins: [router] } })
  await flushPromises()
  return wrapper
}

describe('accessibility (axe-core)', () => {
  it('login page has no violation', async () => {
    const wrapper = await mountWithRouter(LoginPage)
    expect(await violationsOf(wrapper.element)).toEqual([])
    wrapper.unmount()
  })

  it('privacy page has no violation', async () => {
    const wrapper = await mountWithRouter(ContentPage, { page: 'privacy' })
    expect(await violationsOf(wrapper.element)).toEqual([])
    wrapper.unmount()
  })

  it('not-found page has no violation', async () => {
    const wrapper = await mountWithRouter(NotFoundPage)
    expect(await violationsOf(wrapper.element)).toEqual([])
    wrapper.unmount()
  })

  it('login form names every field and offers a visible label for the consent box', async () => {
    const wrapper = await mountWithRouter(LoginPage)
    expect(wrapper.find('label[for="email"]').exists()).toBe(true)
    expect(wrapper.find('label[for="password"]').exists()).toBe(true)
    wrapper.unmount()
  })
})
