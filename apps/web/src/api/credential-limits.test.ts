import { describe, expect, it } from 'vitest'
import { credentialProblem, PASSWORD_MAX, PASSWORD_MIN, type CredentialsDraft } from './credential-limits'

const GOOD = 'x'.repeat(PASSWORD_MIN)
const draft = (patch: Partial<CredentialsDraft> = {}): CredentialsDraft => ({
  email: 'a@example.com',
  currentPassword: '',
  newPassword: '',
  confirmPassword: '',
  ...patch,
})

describe('credentialProblem', () => {
  it('asks for an e-mail address', () => {
    expect(credentialProblem(draft({ email: '  ', newPassword: GOOD, confirmPassword: GOOD }), false)).toBe(
      'account.emailMissing',
    )
  })

  it('refuses an e-mail longer than 254 characters', () => {
    const email = `${'a'.repeat(250)}@b.co`
    expect(credentialProblem(draft({ email, newPassword: GOOD, confirmPassword: GOOD }), false)).toBe(
      'account.emailTooLong',
    )
  })

  it('a wallet account must set a password of 12 to 128 characters, confirmed', () => {
    expect(credentialProblem(draft(), false)).toBe('account.passwordTooShort')
    expect(credentialProblem(draft({ newPassword: 'x'.repeat(PASSWORD_MIN - 1), confirmPassword: 'x'.repeat(PASSWORD_MIN - 1) }), false)).toBe(
      'account.passwordTooShort',
    )
    const long = 'x'.repeat(PASSWORD_MAX + 1)
    expect(credentialProblem(draft({ newPassword: long, confirmPassword: long }), false)).toBe('account.passwordTooLong')
    expect(credentialProblem(draft({ newPassword: GOOD, confirmPassword: GOOD + 'y' }), false)).toBe(
      'account.passwordMismatch',
    )
    expect(credentialProblem(draft({ newPassword: GOOD, confirmPassword: GOOD }), false)).toBe('')
  })

  it('counts characters, not bytes, for the limits', () => {
    const accented = 'é'.repeat(PASSWORD_MIN)
    expect(credentialProblem(draft({ newPassword: accented, confirmPassword: accented }), false)).toBe('')
  })

  it('an account with a password must give its current password', () => {
    expect(credentialProblem(draft(), true)).toBe('account.currentMissing')
    expect(credentialProblem(draft({ currentPassword: 'old' }), true)).toBe('')
  })

  it('keeps the password when only the e-mail changes', () => {
    expect(credentialProblem(draft({ currentPassword: 'old', newPassword: '' }), true)).toBe('')
  })

  it('checks a new password when one is typed on an account that has one', () => {
    expect(credentialProblem(draft({ currentPassword: 'old', newPassword: 'short' }), true)).toBe(
      'account.passwordTooShort',
    )
  })
})
