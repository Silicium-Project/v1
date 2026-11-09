# ADR 0008 : pénalité signée par le requester

**Statut :** accepté (demande du 8 novembre 2026, « corrige les limites »).

## Contexte

ADR 0007 a retiré le comité et, avec lui, les pénalités : plus d'autorité pour sanctionner un
vérificateur qui approuve un résultat faux. Sans sanction, un vérificateur ne perd jamais sa
réputation, et le gate de réputation (ADR 0006) ne bloque que les clés non enregistrées. La
protection Sybil est plus faible qu'en V0.

## Décision

Le **requester** du job peut pénaliser un approbateur d'un fragment réglé :

- Instruction `Penalize(PenaltyArgs { fragment, verifier })`. Le requester signe la transaction : pas
  de précompilé Ed25519, pas de comité.
- Règles : la transaction est signée par `terms.requester` ; le vérificateur a approuvé ce fragment
  (`SettledFragment.approvers`) ; une seule pénalité par (fragment, vérificateur) ; le vérificateur a
  un compte de réputation.
- Effet : score − 1 000, plancher 0. Trois pénalités sur trois fragments sortent un vérificateur de
  l'éligibilité (5 000 → 2 000).

## Pourquoi le requester

Il est la seule partie lésée par un résultat faux : il paie le job et reçoit le résultat. Il a donc
intérêt à sanctionner. Il ne gagne rien à sanctionner un vérificateur honnête.

## Limites

- Le requester peut sanctionner à tort. Un vérificateur honnête peut donc perdre des points. Une
  contestation par preuve de divergence (recalcul sur chaîne) n'existe pas encore.
- Un requester complice d'un worker peut laisser passer un résultat faux, sans pénalité.
- La pénalité est manuelle : le requester doit observer la faute (recalcul hors chaîne) et envoyer
  la transaction.

## Conséquences

- Adaptateur : `penalize_ix` (signée par le requester seul).
- Programme : instruction `Penalize`, compte de réputation écrit (enregistrement de sa pénalité
  dans le compte d'escrow : une entrée par approbateur et fragment).
- Le comité reste absent : seul le requester autorise une pénalité.
