# Site web et application de bureau

Deux clients, deux rôles :

- **Site web** (`apps/web`) : les utilisateurs soumettent les calculs qu'ils veulent (requesters)
  et suivent leur état. Le site ne détient aucune clé : chaque calcul est signé par le wallet
  Solana de l'utilisateur.
- **Application de bureau** (`apps/desktop`) : pour les workers. Elle lance le nœud, garde la clé
  du nœud, et montre au worker ce qu'il a fait (événements, réputation, règlements vus).

```
navigateur ── wallet (signe) ──┐
   │ HTTP (bearer)            ▼
   └──────────────▶ services/api (même origine, CSP stricte, CORS liste blanche)
bureau (Tauri) ── HTTP ─▶ services/api          (liaison de machine, navigation)
   ├─ signer.rs   : clé du nœud, preuve de liaison
   ├─ node_process: lance silicium-node --roles worker
   └─ control_client : GET /v1/worker sur 127.0.0.1:8750 (jeton de lecture du nœud)
```

## Soumettre un calcul (site)

1. Connexion : bouton « Se connecter avec mon wallet ». Le site demande un challenge, le wallet
   signe le message (`Silicium login / Address / Nonce`), l'API vérifie la signature Ed25519 et
   ouvre une session. Le premier login crée le compte (pas d'e-mail, pas de mot de passe).
2. Page **Soumettre** : fichier (8 Mo max), type de calcul (`calcul/sha256`, `rendu/blocs`),
   nombre de fragments (1 à 16), nom facultatif.
3. Le site envoie le fichier (`PUT /v1/content`), construit la spec du job et l'événement
   `JobPublished` (encodeur CBOR de `apps/web/src/signing/`, octets identiques au Rust), puis
   fait signer **deux fois** par le wallet, en texte ASCII lisible : `Silicium JobSpec v1` (une ligne
   par champ de la spec), puis `Silicium Event v1` (l'auteur, le job, la séquence, le hash précédent,
   l'heure et la signature de la spec). Il publie ensuite via `POST /v1/jobs`.

   Pourquoi du texte : Phantom refuse de signer des octets binaires qui ressemblent à une transaction
   Solana (`You cannot sign solana transactions using sign message`). Le texte est affiché tel quel
   dans le wallet, donc l'utilisateur voit ce qu'il signe. Le format est fixé dans
   `crates/sil-ledger/src/wallet_text.rs` ; le site le reproduit à l'identique (`apps/web/src/signing/job.ts`).
   Les événements signés par les nœuds gardent le cadrage binaire (`sil/v1/event`).
4. Le comité vient de la configuration de l'API (`SIL_API_COMMITTEE`), pas du réseau : un nœud
   ne peut pas s'auto-désigner coordinateur.
5. Aucun paiement : la page l'indique tant que l'adaptateur de chaîne (phase 5) n'existe pas.

Les vecteurs partagés (`contracts/vectors/job-published.json`) figent les octets à signer. Un
test Rust et un test TypeScript les comparent ; un test Node vérifie les signatures avec une
clé Ed25519 indépendante.

## Travail (application de bureau)

- **Clé** : la clé du nœud (`node.seed` dans `<app>/node/`). La liaison de machine signe avec cette
  même clé, donc les résultats et le compte sont liés à la même identité.
- **Lancer / arrêter** : le desktop écrit `node.toml` (dossier de données, écoute `/ip4/0.0.0.0/tcp/4001`,
  API de contrôle sur `127.0.0.1:8750`), lance `silicium-node --config … --roles worker`, journal dans
  `node.log`. À la fermeture de l'application, le nœud est arrêté.
- **Données** : `GET /v1/worker` sur l'API de contrôle du nœud (jeton de lecture lu dans
  `control.token`). Le desktop interroge le nœud lui-même ; le navigateur n'a pas de jeton et ne
  contacte jamais le nœud.
- **Contenu du rapport** : rôles, calculs touchés, compteurs d'événements par type, 50 événements
  récents, réputation (0,5 pour un inconnu), règlements vus. `payments: false` : aucun gain n'est
  affiché comme acquis.
- Commandes Tauri : `machine_key`, `create_machine_key`, `sign_link_code`, `node_status`,
  `node_start`, `node_stop`, `worker_report`. Erreurs : codes stables, sans chemin ni secret.

Le binaire est cherché via `SIL_NODE_BIN`, sinon `silicium-node(.exe)` à côté de l'application.

## Réseau et sécurité

- **API** : CORS en liste blanche (`SIL_API_CORS_ORIGINS`) pour l'origine de la webview
  (`http://tauri.localhost` sous Windows, `tauri://localhost` ailleurs). Pas de credentials.
- **Wallet** : challenge à usage unique, lié à l'adresse et au but (`login` ou `delete`), valable
  5 minutes, stocké en mémoire (un redémarrage invalide les challenges). Un échec consomme le
  nonce. La signature ne se déplace ni vers une autre adresse ni vers une autre action.
- **Suppression** : un compte e-mail confirme par mot de passe ; un compte wallet confirme par une
  nouvelle signature d'un challenge `delete` (une signature de login ne suffit pas).
- **CSP** des pages : `default-src 'self'`, pas de script ou style en ligne, pas de cadre.
- **Session** : jeton bearer dans `sessionStorage` (fin avec l'onglet). Compromis assumé : un script
  injecté pourrait le lire ; la CSP est la défense principale.
- **Desktop** : une fenêtre, capacités minimales (`core:default`), aucun plugin shell ni réseau.
  Le CSP autorise `connect-src 'self' http://127.0.0.1:8080` seulement.

## Accessibilité et conformité

Objectif : WCAG 2.2 niveau AA et EN 301 549 (Acte européen d'accessibilité, directive (UE)
2019/882). Contrastes calculés et testés (`palette.ts`), axe-core sur la connexion, la vie privée
et la page introuvable, champs étiquetés, erreurs liées au champ, statut annoncé
(`role="alert"` / `role="status"`), focus sur le titre à chaque page.

**Non vérifié** : lecteur d'écran réel, navigation clavier complète, zoom 400 % et 320 px, pages
Calculs / Soumettre / Machines / Compte / Travail sous axe, audit tiers. La déclaration publiée
(`/accessibilite`) le dit.

RGPD : information à l'inscription (`/confidentialite`), export (`GET /v1/account/export`),
suppression (`DELETE /v1/account`), aucun cookie ni outil de mesure. Les mentions légales,
l'éditeur, l'hébergeur et le contact d'accessibilité sont des **emplacements à compléter**.

## Limites connues

- **Deux popups wallet** par calcul (spec, puis événement). Changer ça demande un changement de
  protocole ; décision reportée à l'escrow Solana.
- **Format de signature du wallet** : le site suppose que `signMessage` signe les octets bruts
  sans préfixe. **Non vérifié avec un vrai wallet** : un test manuel avec Phantom est requis.
- **Pas de paiement** : `reward_per_unit` et `verifier_reward_per_unit` valent 0 et la page le dit. Aucun gain n'est calculé.
- **Comité figé** : `SIL_API_COMMITTEE`, pas de sélection automatique.
- **Révocation** : l'API ne vérifie pas encore les signatures de machine ; une machine révoquée
  peut encore signer.
- **Desktop** : le nœud est arrêté par `kill` (pas d'arrêt propre). Non lancé à l'écran dans ces
  vérifications ; seul le binaire nœud a été démarré avec la même config que le desktop.
- **Pas de paquet d'installation** (`bundle.active` à `false`), icône provisoire.
- **Signature dans le même processus** que l'interface pour la clé de nœud, pas de processus
  signataire séparé (`docs/technical/security/keys.md`). À faire en phase 9.
