import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { setLocale } from '@/i18n'
import EmbeddingInput from './EmbeddingInput.vue'

describe('EmbeddingInput', () => {
  it('offers a file picker for several files of any type, and no text box', async () => {
    setLocale('fr')
    const wrapper = mount(EmbeddingInput, { props: { maxBytes: 1024 } })
    expect(wrapper.emitted('change')?.at(-1)).toEqual([null, 'empty'])
    expect(wrapper.find('textarea').exists()).toBe(false)
    const input = wrapper.get('input[type="file"]')
    expect(input.attributes('multiple')).toBeDefined()
    expect(input.attributes('accept')).toBeUndefined()
    wrapper.unmount()
  })
})
