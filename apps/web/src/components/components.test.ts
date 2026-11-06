import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import FormField from './FormField.vue'
import Notice from './Notice.vue'
import StatusBadge from './StatusBadge.vue'

describe('StatusBadge', () => {
  it('writes the state in text and hides the icon from assistive tech', () => {
    const wrapper = mount(StatusBadge, { props: { state: 'finalized' } })
    expect(wrapper.text()).toBe('Terminé')
    expect(wrapper.find('svg').attributes('aria-hidden')).toBe('true')
    expect(wrapper.classes()).toContain('badge--success')
  })

  it('shows an unknown state as unknown, never as a guess', () => {
    const wrapper = mount(StatusBadge, { props: { state: 'paid_out' } })
    expect(wrapper.text()).toBe('État inconnu')
    expect(wrapper.classes()).toContain('badge--warning')
  })
})

describe('Notice', () => {
  it('announces errors at once with role alert', () => {
    const wrapper = mount(Notice, { props: { kind: 'error', message: 'Échec' } })
    expect(wrapper.attributes('role')).toBe('alert')
  })

  it('announces other notices politely with role status', () => {
    const wrapper = mount(Notice, { props: { kind: 'success', message: 'OK' } })
    expect(wrapper.attributes('role')).toBe('status')
  })

  it('renders nothing without a message', () => {
    expect(mount(Notice, { props: { kind: 'error', message: '' } }).html()).toBe('<!--v-if-->')
  })
})

describe('FormField', () => {
  it('links the label to the input', () => {
    const wrapper = mount(FormField, { props: { id: 'email', label: 'Adresse', modelValue: '' } })
    expect(wrapper.find('label').attributes('for')).toBe('email')
    expect(wrapper.find('input').attributes('id')).toBe('email')
  })

  it('marks an invalid input and points it to its error', () => {
    const wrapper = mount(FormField, {
      props: { id: 'email', label: 'Adresse', modelValue: 'x', error: 'Adresse invalide', hint: 'Exemple' },
    })
    const input = wrapper.find('input')
    expect(input.attributes('aria-invalid')).toBe('true')
    expect(input.attributes('aria-describedby')).toBe('email-hint email-error')
    expect(wrapper.find('#email-error').text()).toBe('Adresse invalide')
  })

  it('emits the typed value', async () => {
    const wrapper = mount(FormField, { props: { id: 'x', label: 'X', modelValue: '' } })
    await wrapper.find('input').setValue('bonjour')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['bonjour'])
  })
})
