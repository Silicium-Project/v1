# sil-devnet-job — test d'un job complet sur Solana devnet

Outil de test, sans wallet. Il fait ce que le site fait, avec une clé de test :

1. connexion à l'API par signature Ed25519 (même chemin que Phantom) ;
3. envoi de l'entrée (`PUT /v1/content`) ;
4. ouverture de l'escrow on-chain par le requester (`open`), avec confirmation ;
5. soumission de l'événement `JobPublished` signé (`POST /v1/jobs`) ;
6. attente de `finalized` côté réseau, puis du règlement on-chain de chaque fragment ;
7. affichage du total payé et de qui a réalisé et vérifié chaque fragment.

Il ne règle rien lui-même : le règlement est fait par les nœuds, avec leur clé payeur (ADR 0007).

## Prérequis

- Le programme est déployé sur devnet (`5nvVoget71j65HtMZmfVJjnwSePDtrG5hzAPpjMbrMkx`).
- L'API est joignable : tunnel SSH vers le VPS, par exemple
  `ssh -N -L 8080:127.0.0.1:8080 ubuntu@<IP du VPS>`.
- Le VPS et au moins un autre nœud tournent (calcul, vérification).
- Le requester de test a du SOL devnet (le faucet limite les demandes ; un transfert depuis un
  autre compte de test suffit).

## Lancer

```bash
cd chain/devnet-job
cargo run --release
```

Options (toutes facultatives) :

| Option | Défaut | Rôle |
|---|---|---|
| `--api` | `http://127.0.0.1:8080` | API publique (tunnel) |
| `--rpc` | `https://api.devnet.solana.com` | RPC Solana |
| `--program` | identifiant devnet | programme d'escrow |
| `--seed` | `~/sil-devnet/requester.seed` | graine du requester de test (32 octets, 0600) |
| `--bytes` | `2048` | taille de l'entrée (1 à 1048576) |
| `--fragments` | `2` | fragments (1 à 16) |
| `--timeout` | `900` | délai de chaque étape, en secondes |

La graine est créée au premier lancement si elle n'existe pas. Elle ne quitte pas la machine et
n'apparaît dans aucun journal.

## Limites

- Le requester est une identité de test. Sur le site, c'est le wallet de l'utilisateur qui signe.
- Le dépôt se fait par l'outil, pas par l'interface : le bouton « déposer » du site reste à faire.
- Un seul job par lancement. Aucun nettoyage : l'escrow reste on-chain après le test.
