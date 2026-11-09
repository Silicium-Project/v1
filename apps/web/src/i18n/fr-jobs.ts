// Annulation d'un calcul par son requester (voir JobDetailPage.vue). Fusionné dans `job` de fr.ts.
export const frJobCancel = {
  cancel: 'Annuler le calcul',
  cancelNote:
    'Le réseau arrête de travailler sur ce calcul. Le paiement n’est remboursé qu’à l’échéance du calcul plus 24 h (règle de l’escrow).',
  cancelAsk: 'Confirmer l’annulation avec votre wallet ?',
  cancelConfirm: 'Annuler avec le wallet',
  cancelKeep: 'Garder le calcul',
  cancelFailed: 'L’annulation n’a pas abouti. Vérifiez que le wallet connecté est celui du requester.',
}
