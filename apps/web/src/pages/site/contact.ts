/**
 * Adresse qui reçoit les messages de la page Contact. Le site n'a pas de serveur d'envoi : le
 * formulaire ouvre la messagerie du visiteur, avec le message prêt à envoyer.
 */
export const CONTACT_EMAIL = 'contact@siliciumlabs.org'

/** Lien `mailto:` pour un sujet et un corps de message. Le corps est encodé, rien n'est envoyé ici. */
export function mailtoLink(subject: string, body: string): string {
  const query = `subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`
  return `mailto:${CONTACT_EMAIL}?${query}`
}
