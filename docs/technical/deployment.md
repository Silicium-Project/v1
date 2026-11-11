# Déploiement du VPS (testnet)

Ce document décrit ce qui tourne sur le VPS du testnet, comment il est installé et ce qui reste
ouvert. Les fichiers cités sont dans `infra/`.

## Ce qui tourne sur le VPS

| Composant | Rôle | Écoute | Utilisateur |
|---|---|---|---|
| `silicium-node` (`--roles worker,verifier`) | point d'amorçage P2P, worker et vérificateur | `4001/tcp` et `4001/udp` (public), `127.0.0.1:8750` (API locale) | `silicium` |
| `silicium-api` | comptes, soumission de jobs, site web (fichiers statiques) | `127.0.0.1:8080` | `silicium` |
| Caddy | HTTPS public, certificat Let's Encrypt renouvelé automatiquement | `80/tcp`, `443/tcp` | `caddy` |

Ce qui **ne** tourne **pas** sur le VPS :

- **Les workers et vérificateurs** : ce sont les postes des utilisateurs, via l'application de bureau.
  Un calcul ne doit pas dépendre du VPS.
- **La clé de paiement Solana** : retirée du VPS le 8 nov. (voir « Points ouverts »). Le nœud du VPS
  relaie les règlements et vérifie, il ne paie pas lui-même.
- **La compilation** : les binaires sont compilés sur le poste de développement, puis copiés.
  Un build sur le VPS ajouterait des outils et des sources à un serveur exposé. Les artefacts
  signés et les canaux de mise à jour sont prévus en phase 9.

Pas de comité (ADR 0007) : le testnet ne demande aucune configuration de comité. Un seul
nœud reste un point unique de défaillance, à corriger avant tout déploiement réel.

## Sécurité du serveur

Appliqué par `infra/vps/base.sh` :

- **Pare-feu UFW** : entrée refusée par défaut ; ouverts : `22/tcp` (SSH), `80/tcp` et `443/tcp`
  (Caddy), `4001/tcp` et `4001/udp` (P2P). Les ports `8080` et `8750` ne sont accessibles que
  depuis `127.0.0.1`.
- **SSH par clé seulement** : `PasswordAuthentication no`, `PermitRootLogin no`. Vérifier une
  connexion par clé dans une **deuxième** session avant de fermer la première.
- **Mises à jour** : `unattended-upgrades` installe les correctifs de sécurité. Un redémarrage
  reste manuel : `/var/run/reboot-required` indique quand il est nécessaire.
- **Utilisateurs** : `silicium` (système, sans shell de connexion) exécute le nœud et l'API ;
  `caddy` exécute le proxy. Aucun service ne tourne en `root`.
- **Units systemd durcies** (`infra/systemd/`) : `NoNewPrivileges`, `ProtectSystem=strict`,
  écriture seulement dans `/var/lib/silicium/<service>`, familles d'adresses limitées à
  IPv4, IPv6 et sockets Unix. Le nœud a aussi besoin de `AF_NETLINK` (découverte des interfaces) :
  sans lui, l'écoute échoue avec `écoute impossible`.

## Arborescence sur le VPS

```
/opt/silicium/bin/          silicium-node, silicium-api (binaires Linux x86-64)
/opt/silicium/web/          site web construit (apps/web/dist)
/etc/silicium/node.toml     configuration du nœud (0640 root:silicium)
/etc/silicium/api.env       variables de l'API (0640 root:silicium)
/var/lib/silicium/node/     node.seed (identité du nœud), control.token, journal
/var/lib/silicium/api/      silicium.db (SQLite : comptes, sessions, liaisons, machines, index des jobs)
/etc/caddy/Caddyfile        HTTPS public
```

`node.seed` est l'identité du nœud : **le sauvegarder hors du VPS**. Le perdre crée un nouveau
nœud, avec une nouvelle identité et sans la réputation de l'ancien.

## Installation pas à pas

Toutes les commandes sont lancées depuis la racine du dépôt, sur le poste de développement, sauf
mention contraire. `HOTE` désigne `ubuntu@<IP du VPS>`.

1. **Socle du serveur** (une fois) :

   ```bash
   ssh HOTE 'bash -s' < infra/vps/base.sh
   ```

2. **Binaire de l'API** (Go, `CGO_ENABLED=0` : le driver SQLite `modernc.org/sqlite` est en Go pur) :

   ```bash
   cd services/api
   GOOS=linux GOARCH=amd64 CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" \
     -o ../../dist/linux-amd64/silicium-api ./cmd/silicium-api
   cd ../..
   ```

3. **Binaire du nœud et runtime** (Rust, compilé sous Linux, par exemple dans WSL) :

   ```bash
   cd chain/node && cargo build --release && cd ../..
   mkdir -p dist/linux-amd64
   cp chain/node/target/release/silicium-node dist/linux-amd64/
   cargo build --release -p sil-raytracer --bin sil-raytracer-block
   cp target/release/sil-raytracer-block dist/linux-amd64/
   cargo build --release -p sil-sandbox --bin sil-sandbox-launch
   cp target/release/sil-sandbox-launch dist/linux-amd64/
   python3 tools/runtime/fetch_onnxruntime.py linux dist/linux-amd64   # ONNX Runtime 1.31.1, hash vérifié
   ```

   Le nœud du VPS est `chain/node` (même binaire que l'application). `sil-node` seul refuse la
   section `[chain]`.

4. **Site web** :

   ```bash
   cd apps/web && npm ci && npm run build && cd ../..
   tar -C apps/web/dist -cf - . | ssh HOTE 'sudo tar -C /opt/silicium/web -xf -'
   ```

5. **Binaires et unités** :

   ```bash
   infra/vps/deploy.sh HOTE
   ```

6. **Configuration** : copier `infra/config/node.toml` et `infra/config/api.env.example`
   (renommé en `api.env`) vers `/etc/silicium/`, puis fixer les droits :

   ```bash
   scp infra/config/node.toml HOTE:/tmp/node.toml
   scp infra/config/api.env.example HOTE:/tmp/api.env
   ssh HOTE 'sudo install -m 0640 -o root -g silicium /tmp/node.toml /etc/silicium/node.toml &&
     sudo install -m 0640 -o root -g silicium /tmp/api.env /etc/silicium/api.env &&
     rm -f /tmp/node.toml /tmp/api.env'
   ```

7. **Démarrer le nœud**, puis noter son identifiant de pair dans le journal :

   ```bash
   ssh HOTE 'sudo systemctl enable --now silicium-node && sudo journalctl -u silicium-node -n 40 --no-pager'
   ```

8. **Clé publique du nœud** (information, aucune clé de comité à configurer, ADR 0007) :

   ```bash
   ssh HOTE 'sudo -u silicium /opt/silicium/bin/silicium-node --config /etc/silicium/node.toml --public-key'
   ```

   Le résultat est 64 caractères hexadécimaux minuscules.

9. **Démarrer l'API** et vérifier qu'elle répond en local :

   ```bash
   ssh HOTE 'sudo systemctl enable --now silicium-api && curl -s http://127.0.0.1:8080/health'
   ```

10. **HTTPS** (après l'enregistrement DNS du domaine vers l'IP du VPS) :

   ```bash
   ssh HOTE 'sudo tee /etc/caddy/Caddyfile >/dev/null' < infra/caddy/Caddyfile
   ssh HOTE 'sudo mkdir -p /etc/systemd/system/caddy.service.d &&
     printf "[Service]\nEnvironment=SIL_DOMAIN=%s\n" "DOMAINE" |
     sudo tee /etc/systemd/system/caddy.service.d/silicium.conf >/dev/null &&
     sudo systemctl daemon-reload && sudo systemctl enable --now caddy'
   ```

   Remplacer `DOMAINE` par le nom de domaine. Le certificat est émis au premier accès HTTPS.

## Vérifications

- `ssh HOTE 'sudo ufw status verbose'` : seuls 22, 80, 443, 4001 (tcp et udp) sont ouverts.
- `curl -I https://DOMAINE/health` : `200`, avec `Strict-Transport-Security`.
- `ssh HOTE 'sudo ss -tlnp'` : `8080` et `8750` liés à `127.0.0.1` uniquement.
- `ssh HOTE 'sudo systemctl is-active silicium-node silicium-api caddy'` : trois `active`.

## Mise à jour d'un déploiement

Deux scripts, même effet : copier `silicium-api`, `silicium-node`, `sil-sandbox-launch`,
`sil-raytracer-block`, `libonnxruntime.so` et les deux unités systemd dans `/opt/silicium/bin/` (écriture
atomique), puis relancer les services.

- **Windows** (poste de développement) : `powershell -ExecutionPolicy Bypass -File infra\vps\deploy.ps1 -Target ubuntu@213.32.68.67`.
  Utilise le OpenSSH de Windows et `~\.ssh\id_ed25519`. `-DryRun` vérifie la connexion et prépare les
  fichiers sans rien copier.
- **Linux / WSL** : `SIL_SSH_KEY=… SIL_SSH_KNOWN_HOSTS=… bash infra/vps/deploy.sh ubuntu@213.32.68.67`.
  Sous WSL, `~/.ssh` n'est pas celui de Windows : sans `SIL_SSH_KEY`, le script échoue sur la clé absente.

Les deux passent en `BatchMode` et `StrictHostKeyChecking=yes` : aucune question interactive, et un hôte
inconnu fait échouer le déploiement. Vérifier ensuite :

```bash
ssh HOTE 'sudo sha256sum /opt/silicium/bin/silicium-node /opt/silicium/bin/libonnxruntime.so'
ssh HOTE 'sudo journalctl -u silicium-node -n 20 --no-pager | grep -i "onnx"'
```

Attendu : `Loaded ONNX Runtime dylib with version '1.31.1'`.

Le pipeline `.github/workflows/deliver.yml` (build puis déploiement par environnement) n'est pas
opérationnel : les environnements GitHub `development` et `production` et leurs secrets ne sont pas
configurés. Le déploiement se fait à la main avec les commandes ci-dessus.

## Base de l'API (SQLite)

Les données de l'API sont dans `/var/lib/silicium/api/silicium.db`. Au premier démarrage du nouveau
binaire, un `store.json` présent dans le même dossier est importé une fois, puis renommé en
`store.json.migrated` (jamais supprimé). Ordre à respecter, car le nouveau binaire ouvre
`SIL_API_STORE` comme une base :

```bash
ssh HOTE 'sudo cp -a /var/lib/silicium/api/store.json /var/lib/silicium/api/store.json.bak'   # sauvegarde
ssh HOTE 'sudo sed -i "s#^SIL_API_STORE=.*#SIL_API_STORE=/var/lib/silicium/api/silicium.db#" /etc/silicium/api.env'
powershell -ExecutionPolicy Bypass -File infra\vps\deploy.ps1 -Target ubuntu@213.32.68.67      # redémarre l'API
ssh HOTE 'sudo ls -l /var/lib/silicium/api/ && sudo journalctl -u silicium-api -n 20 --no-pager'
```

Vérifier : `silicium.db` existe, `store.json.migrated` est là, et l'API répond sur `/health`. Un
`store.json` présent alors que la base contient déjà des données fait refuser le démarrage
(« import refusé ») : ne pas le supprimer à la main, le sortir du dossier après vérification.

## Points ouverts

- **Clé payeur retirée du VPS** (8 nov.) : section `[chain]` supprimée de `/etc/silicium/node.toml`
  (sauvegarde `node.toml.before-chain-removal`), fichier `payer.json` détruit (`shred -u`). Le nœud
  du VPS ne réclame plus de fragment (« aucune chaîne configurée ») : il relaie et vérifie.
- **Relais** : `relay_server = true` sur le VPS (`[network]`). Les nœuds derrière NAT réservent un
  circuit auprès de lui (voir `node.md`, section NAT).
- **Domaine** : HTTPS et le certificat exigent un nom de domaine. Sans lui, le site reste inaccessible
  depuis Internet (l'API n'a pas de TLS). Le nom `vps-910c1dbc.vps.ovh.net` résout sur l'IP du VPS,
  mais il appartient à OVH : aucun certificat public ne peut y être émis, donc un site de dev sur ce
  nom demande un certificat interne ou un autre port (à vérifier avant de configurer Caddy).
- **Pas de sauvegarde automatique** de `silicium.db` ni de `node.seed` pour l'instant.
- **Artefacts non signés** : le déploiement copie des binaires sans vérifier leur origine. À remplacer
  par des artefacts signés (phase 9).
