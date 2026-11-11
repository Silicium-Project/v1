import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { setLocale } from '@/i18n'
import EmbeddingInput from './EmbeddingInput.vue'

describe('EmbeddingInput', () => {
  it('emits the passages file as the user types, and nothing when the text is empty', async () => {
    setLocale('fr')
    const wrapper = mount(EmbeddingInput, { props: { maxBytes: 1024 } })
    expect(wrapper.emitted('change')?.at(-1)).toEqual([null, 'empty'])
    await wrapper.get('textarea').setValue('Un texte\nUn autre')
    const last = wrapper.emitted('change')?.at(-1) as [File, string]
    expect(last[1]).toBe('')
    expect(last[0].name).toBe('passages.txt')
    await wrapper.get('textarea').setValue('   ')
    expect(wrapper.emitted('change')?.at(-1)).toEqual([null, 'empty'])
    expect(wrapper.text()).toContain('au moins un texte')
    wrapper.unmount()
  })
})