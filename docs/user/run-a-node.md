# Lancer un nœud

> **État** : le nœud démarre, rejoint le réseau et expose son API, mais il n'a **pas encore de
> connexion à la chaîne Solana**. Un nœud lancé ainsi **ne réclame aucun calcul** et ne règle
> rien : il sert à tester le réseau et l'API, pas à gagner de l'argent.

## Construire et démarrer

```bash
cargo build --release -p sil-node
./target/release/silicium-node --config node.toml --roles worker,verifier
```

Rôles : `worker` (calcule),
`verifier` (vérifie les calculs des autres). Combinables, séparés par des virgules.

## Configuration minimale (`node.toml`)

```toml
data_dir = "data"

[network]
listen = ["/ip4/0.0.0.0/tcp/4001"]
bootstrap = ["/ip4/203.0.113.10/tcp/4001/p2p/12D3KooW..."]

[control]
bind = "127.0.0.1"
port = 8750
```

Toutes les clés sont décrites dans [configuration.md](configuration.md). Au démarrage :

- la clé du nœud est créée dans `data/node.seed` (droits 0600 sous Unix, **à sauvegarder** : elle
  porte votre identité) ;
- un jeton d'accès est créé dans `data/control.token` s'il n'existe pas.

## Afficher la clé publique du nœud

```bash
./target/release/silicium-node --config node.toml --public-key
```

Affiche la clé publique du nœud (64 caractères hexadécimaux) puis s'arrête. Elle identifie le nœud ;
aucune graine n'est créée ni affichée : sans `node.seed`, la commande échoue.

## Appeler l'API locale

Le jeton est la partie après `read,submit:` dans `data/control.token`.

```bash
curl http://127.0.0.1:8750/health
curl -H "Authorization: Bearer <jeton>" http://127.0.0.1:8750/v1/peers
```

La liste des routes et des droits (`read`, `submit`) : `GET /v1/openapi.json`, ou
[control-api.md](../technical/security/control-api.md).

## Ce qui peut mal tourner

| Symptôme | Cause probable |
|---|---|
| `401` sur toutes les routes | jeton absent ou faux (en-tête `Authorization: Bearer …`) |
| `429` | trop de requêtes avec ce jeton, ou trop d'essais de jetons invalides |
| aucun calcul réclamé | normal sans chaîne configurée ; sinon pas d'escrow valide pour le job |
| `pairs : []` | le nœud n'a pas pu joindre le bootstrap ; derrière un routeur, le nœud n'est pas joignable (pas de traversée de NAT) |
| arrêt avec « adresse … n'épingle pas le pair » | une adresse de bootstrap doit finir par `/p2p/<id du pair>` |
