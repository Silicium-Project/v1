# Clés et signatures (`sil-crypto`)

## Une graine, des clés indépendantes

Le nœud stocke une graine de 32 octets (`MasterSeed`). Chaque usage en dérive sa clé par
HKDF-SHA256 avec une étiquette propre : `Purpose::Network` (identité des pairs),
`Purpose::Ledger` (événements). Compromettre ou faire tourner une clé ne révèle pas l'autre.
V0 utilisait un hash de la clé réseau pour le journal. Le wallet de paiement reste séparé
(jamais dérivé de cette graine).

## Domaines de signature

Le message réellement signé est `tag || 0x00 || message`. Les tags sont dans `Domain`
(`sil/v1/event`, `sil/v1/job-spec`, …; les spécifications et les événements `JobPublished` signés par un wallet sont des textes, voir `web-and-desktop.md`). Une signature d'un domaine n'est jamais valide
dans un autre. `sha256_domain` applique la même séparation aux identifiants
(`job_id = sha256_domain(JobSpec, octets du JobSpec)`).
La vérification utilise `verify_strict` (clés faibles et signatures malléables refusées).

## Fichier clé

`keyfile::load_or_create` crée le fichier :
- en `create_new` + mode **0600** (Unix), écrit puis `fsync` ;
- publié par `hard_link` : il n'écrase jamais un fichier existant, même en cas de course ;
- au chargement sous Unix, refuse un fichier lisible par le groupe ou les autres.

**Windows** : le mode 0600 n'existe pas, le fichier hérite des ACL du dossier de données.
Ce dossier doit être réservé à l'utilisateur du nœud ; le contrôle automatique des ACL
reste à faire (limite connue, test Unix non exécuté sur Windows).

## Secrets en mémoire et logs

`MasterSeed` est effacée à la destruction (`zeroize`) et son `Debug` est masqué.
Les jetons se comparent avec `ct_eq` (temps constant).

## Limites connues

- Pas encore de rotation ni de révocation signée (prévu avec le journal, phase 2).
- Pas de keystore OS/HSM (option prévue en phase 8, desktop).
