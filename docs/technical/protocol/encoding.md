# Encodage canonique (`sil-codec`)

Tout objet signé ou haché (événement, `JobSpec`, attestation, certificat) est encodé en
CBOR par `sil_codec::encode`. L'API `decode` accepte **uniquement** l'encodage que
`encode` produirait.

## Garanties

| Contrôle | Erreur |
|---|---|
| Taille > limite fournie (vérifiée avant tout parsing) | `TooLarge` |
| CBOR invalide ou imbrication > 32 niveaux | `Malformed` |
| Octets après la valeur | `TrailingBytes` |
| Entiers non minimaux, champs réordonnés, toute variante d'octets | `NotCanonical` |

Conséquence : une signature ne peut pas être rejouée sur une variante d'encodage du même
contenu (malléabilité), et deux pairs hachent toujours les mêmes octets.

## Règles pour les structures du protocole

- Structs à champs ordonnés (l'ordre de déclaration fait foi) ; pas de `HashMap` ni
  de flottants dans ce qui est signé ; `BTreeMap` si une table est nécessaire.
- Hash, clés et signatures : types `sil-types` (CBOR `bstr` de longueur fixe).
- Signer = `identity.sign(Domain::X, &encode(&valeur))` ; vérifier avec le même domaine.
- Toute évolution de format change un numéro de version dans la structure, jamais
  l'interprétation d'un champ existant.

## Limites

Chaque point d'entrée réseau passe sa propre `max_len` (voir `limits.max_message_bytes`).
