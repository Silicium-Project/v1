# Configuration du nœud

La configuration vient, par ordre croissant de priorité, de : valeurs par défaut,
fichier TOML, variables d'environnement `SIL_*`. Le nœud refuse de démarrer si elle est
dangereuse ou incohérente, avec un message qui nomme le champ.

## Exemple de fichier

```toml
data_dir = "data"

[network]
listen = ["/ip4/0.0.0.0/udp/4001/quic-v1"]
bootstrap = ["/ip4/203.0.113.10/udp/4001/quic-v1/p2p/12D3KooW..."]
# Relais public : à activer seulement si ce nœud est joignable directement (port ouvert).
relay_server = false
enable_mdns = false

[control]
bind = "127.0.0.1"
port = 8750
# token_file = "secrets/control.token"

[trust]
# Requesters (clés du journal, 64 hex) dont les résultats alimentent la réputation locale.
requesters = []

[limits]
max_message_bytes = 65536
max_peers = 128

[log]
level = "info"
json = false
```

## Variables d'environnement

| Variable | Effet |
|---|---|
| `SIL_DATA_DIR` | dossier de données |
| `SIL_CONTROL_BIND`, `SIL_CONTROL_PORT` | adresse et port de l'API locale |
| `SIL_CONTROL_TOKEN_FILE` | fichier du jeton d'accès |
| `SIL_LOG_LEVEL` | `error`, `warn`, `info`, `debug`, `trace` |
| `SIL_LOG_JSON` | `true` pour des logs JSON |

## Règles de sécurité vérifiées au démarrage

- Une API qui n'écoute pas sur la boucle locale **exige** `token_file`.
- Chaque pair d'amorçage doit être épinglé (`/p2p/<id>`).
- Les clés inconnues du fichier TOML sont refusées (pas de faute de frappe silencieuse).
- Les limites ne peuvent pas être nulles.
- `[trust] requesters` : chaque entrée doit faire 64 caractères hexadécimaux minuscules.

Le dossier de données contient les clés : réservez-le à l'utilisateur qui lance le nœud.

## Modes de test (`[testnet]`, optionnel)

```toml
[testnet]
free_work = true      # le worker calcule sans vérifier d'escrow : le travail n'est pas payé
```

Tous les modes sont désactivés par défaut.

- **`free_work`** : sert à tester le réseau P2P (publication, réclamation, vérification) avant
  d'avoir une chaîne.
- **Refusés avec `[chain]`** : le nœud ne démarre pas. Dès qu'une chaîne existe, le travail doit être
  payé et un résultat doit être vérifié par un tiers.
- Un avertissement est écrit au démarrage tant qu'un mode est actif.
- Réseau de test seulement : ne jamais les activer sur un nœud qui doit gagner de l'argent.

## Chaîne Solana (`[chain]`, optionnel)

Sans cette section, le nœud ne règle rien. Avec, il faut le binaire `chain/node` (le binaire
`sil-node` la refuse).

| Champ | Rôle |
|---|---|
| `rpc_url` | URL RPC `http://` ou `https://` |
| `program_id` | identifiant base58 du programme d'escrow |
| `payer_keypair` | fichier JSON de la clé qui paie les frais de transaction (elle doit avoir du SOL) |

## Moteur de rendu (`[render]`, optionnel)

Sans cette section, le nœud ne sait pas faire de `rendu/raytracer`. Avec, le moteur désigné doit
exister au démarrage. Le moteur tourne avec les droits du nœud : n'utilisez que des programmes
de confiance.

Le moteur du workload `rendu/raytracer` est le binaire `sil-raytracer-block` (crate
`sil-raytracer`, moteur porté de la v0) :

```bash
cargo build --release -p sil-raytracer
```

```toml
[render]
engine = "target/release/sil-raytracer-block"
```

C'est un programme natif : il est lancé dans `sil-sandbox` (sous Windows, ajouter `.exe`).

| Champ | Rôle |
|---|---|
| `engine` | chemin du moteur de rendu. Un fichier `.wasm` (recommandé) est isolé par construction, sur tous les systèmes. Un programme natif est lancé dans `sil-sandbox` (protocole : `sil-wl-render::ExternalRenderer`) |
| `wall_ms` | durée maximale d'un bloc, en millisecondes (défaut : 600000, maximum : 3600000) |
| `isolation` | `strict` (défaut) : refuse de lancer si l'isolation complète n'est pas possible ; `best-effort` : lance avec ce qui existe (sous Windows : seulement l'arrêt de l'arbre de processus) ; `off` : aucune isolation, développement seulement |

Sous Linux, le moteur lancé en `strict` n'écrit que dans `data_dir/render-work`, n'a pas de réseau
IPv4/IPv6, et ne lit que son propre dossier et les bibliothèques système. Le lanceur
`sil-sandbox-launch` doit être à côté de `silicium-node` (ou désigné par `SIL_SANDBOX_LAUNCHER`).
Sous Windows, `strict` refuse : utiliser `best-effort` en connaissance de cause (voir
`docs/technical/security/sandbox.md`).
