# Lancer un nœud

> **État** : le nœud démarre, rejoint le réseau et expose son API. Les règlements passent par la
> chaîne de test (devnet Solana) : aucun gain réel. Un nœud sans section `[chain]` ne règle rien et
> ne réclame aucun calcul rémunéré.

## Obtenir le nœud

Le plus simple : l'application de bureau (voir [desktop-app.md](desktop-app.md)), qui embarque le
nœud et le runtime. Sans installeur, deux archives sont publiées sur le site :

- `silicium-node-linux-amd64.tar.gz` : `silicium-node`, `sil-raytracer-block`, `sil-sandbox-launch`,
  `libonnxruntime.so`, un `node.toml` d'exemple et un README.
- `silicium-node-windows-x86_64.zip` : `silicium-node.exe`, `sil-raytracer-block.exe`,
  `onnxruntime.dll`, `node.toml` et README.

Vérifiez la somme SHA-256 (`.sha256` à côté de l'archive) avant de décompresser.

## Construire depuis les sources

```bash
cd chain/node && cargo build --release          # nœud avec chaîne (binaire de l'application)
cd ../.. && cargo build --release -p sil-raytracer
```

Le binaire `cargo build -p sil-node` existe, mais il refuse une section `[chain]`. Il sert aux tests
réseau sans chaîne.

Le runtime ONNX (nécessaire à l'embedding, voir ci-dessous) se télécharge depuis PyPI, avec hash
vérifié, dans le dossier du binaire :

```bash
python3 tools/runtime/fetch_onnxruntime.py linux target/release     # Linux
python tools/runtime/fetch_onnxruntime.py windows target/release    # Windows
```

## Démarrer

```bash
./target/release/silicium-node --config node.toml --roles worker,verifier
```

Rôles : `worker` (calcule), `verifier` (vérifie les calculs des autres). Combinables, séparés par des
virgules.

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

## Types de calcul proposés

Un nœud n'offre que les types de calcul qu'il peut exécuter :

| Type | Condition |
|---|---|
| `calcul/sha256` | toujours |
| `image/batch` | toujours (fourni par le nœud) |
| `rendu/raytracer` | moteur `sil-raytracer-block` désigné par `[render]` (voir [configuration.md](configuration.md)) |
| `embedding/e5-small` | ONNX Runtime 1.31.1 à côté du binaire, et modèle présent (voir ci-dessous) |

Le nœud écrit dans son journal ce qu'il n'offre pas, par exemple `embedding non offert par ce nœud`.

### Embedding (`embedding/e5-small`)

- **Runtime** : `libonnxruntime.so` (Linux) ou `onnxruntime.dll` (Windows), 1.31.1, à côté du binaire.
  Les paquets officiels l'incluent. Sous Linux, le paquet `.deb` le place dans `/usr/lib/Silicium/`,
  où le nœud le cherche aussi.
- **Modèle** : au premier démarrage, le nœud télécharge `multilingual-e5-small` (environ 470 Mo) et
  son tokenizer (17 Mo) depuis Hugging Face, dans `data/embed/`. Il vérifie leur SHA-256 et refuse
  tout fichier qui ne correspond pas. Le nœud doit donc pouvoir joindre `huggingface.co` au premier
  lancement.
- Après l'installation du runtime, redémarrez le nœud pour qu'il propose le workload.

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
| `pairs : []` | le nœud n'a pas pu joindre le bootstrap ; derrière un routeur, le nœud n'est pas joignable (le relais aide, voir `[network]`) |
| arrêt avec « adresse … n'épingle pas le pair » | une adresse de bootstrap doit finir par `/p2p/<id du pair>` |
| `embedding non offert par ce nœud` | runtime ONNX absent à côté du binaire, ou modèle absent / hash incorrect |
| `runtime absent : …` | la bibliothèque ONNX Runtime 1.31.1 n'est pas à l'emplacement affiché |
| `Loaded ONNX Runtime dylib … 1.31.1` suivi d'un `WARN` « may have compatibility issues » | avertissement de `ort` (attend 1.17.x) : sans gravité, 1.31.1 est la version mesurée et voulue |
| le téléchargement du modèle échoue | pas de sortie vers `huggingface.co` ; relancer après correction du réseau |
