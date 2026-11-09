// Limites des identifiants d'un compte. Les mêmes valeurs sont imposées par l'API
// (services/api/internal/auth) : ici elles évitent un aller-retour inutile et guident la saisie.

export const EMAIL_MAX = 254
export const PASSWORD_MIN = 12
export const PASSWORD_MAX = 128

export interface CredentialsDraft {
  email: string
  /** Mot de passe actuel : requis quand le compte en a déjà un. */
  currentPassword: string
  /** Nouveau mot de passe ; vide : on garde l'actuel (si le compte en a un). */
  newPassword: string
  confirmPassword: string
}

/** Clé i18n du premier problème de saisie, ou '' si la saisie peut être envoyée. */
export function credentialProblem(draft: CredentialsDraft, hasPassword: boolean): string {
  const email = draft.email.trim()
  if (email === '') return 'account.emailMissing'
  if (email.length > EMAIL_MAX) return 'account.emailTooLong'
  if (hasPassword && draft.currentPassword === '') return 'account.currentMissing'
  const wantsNewPassword = draft.newPassword !== '' || !hasPassword
  if (!wantsNewPassword) return ''
  const length = [...draft.newPassword].length
  if (length < PASSWORD_MIN) return 'account.passwordTooShort'
  if (length > PASSWORD_MAX) return 'account.passwordTooLong'
  if (draft.newPassword !== draft.confirmPassword) return 'account.passwordMismatch'
  return ''
}
