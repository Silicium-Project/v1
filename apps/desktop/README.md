# apps/desktop

Application de bureau Silicium (Tauri 2). Elle affiche l'interface `apps/web` et détient la
clé de signature de la machine. La clé ne quitte jamais la machine : l'interface ne reçoit que
la clé publique et la signature d'un code de liaison.

## Compiler

Prérequis : Rust stable, Node 24, et les dépendances système Tauri pour votre système
(WebView2 sous Windows, `libwebkit2gtk-4.1-dev` et voisins sous Linux).

```bash
npm --prefix ../web ci
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Lancer en développement (le serveur Vite de `apps/web` démarre seul) :

```bash
cargo tauri dev --manifest-path src-tauri/Cargo.toml
```

Le crate n'appartient pas au workspace du nœud (`exclude` dans le `Cargo.toml` racine) : il a
son propre fichier de verrouillage et ses dépendances graphiques ne pèsent pas sur les tests
du réseau.

## Commandes exposées à l'interface

| Commande | Effet | Retour |
|---|---|---|
| `machine_key` | lit la clé publique de la machine | clé publique hex, ou `null` |
| `create_machine_key` | crée la clé (ne remplace jamais une clé existante) | clé publique hex |
| `sign_link_code` | signe un code de liaison (domaine `sil/v1/device-link`) | `{ public_key, signature }` |

Erreurs : `no_key`, `bad_code`, `storage`. Aucun chemin de fichier ni secret n'est renvoyé.

Documentation : `../../docs/technical/web-and-desktop.md`, `../../docs/user/desktop-app.md`.
