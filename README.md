# Silicium V1

Réseau de calcul P2P décentralisé et distribué, réécrit de zéro.
Dossier local pour l'instant (pas encore de dépôt Git).

- Plan complet : `C:\Users\Thom\.claude\plans\parallel-inventing-rivest.md`
- Architecture : [docs/technical/architecture.md](docs/technical/architecture.md)
- Démarrer en tant qu'utilisateur : [docs/user/quickstart.md](docs/user/quickstart.md)
- Règles de contribution : [docs/contributing/file-size-rule.md](docs/contributing/file-size-rule.md)

## Vérifier le dépôt

```bash
python tools/check_size.py .   # aucun fichier > 300 lignes
cargo test --workspace          # une fois des crates ajoutés
```

## État

Phases 0 et 1 faites (squelette, CI, ADR, socle : `sil-types`, `sil-crypto`, `sil-codec`, `sil-config`, `sil-telemetry`).
Phase 2 faite : journal (`sil-attest`, `sil-quota`, `sil-ledger`, `sil-ledger-store`), voir `docs/technical/protocol/ledger.md`.
Phase 3 faite : réseau et contenu (`sil-content`, `sil-models`, `sil-net`, `sil-transfer`), voir `docs/technical/protocol/gossip.md` et `content.md`. Pas encore de traversée de NAT.
Phase 4 faite (partielle) : calcul et vérification (`sil-sampling`, `sil-workload-api`, `sil-wl-sha256`, `sil-wl-render`, `sil-sandbox`, `sil-reputation`, `sil-coordinator`), voir `verification.md`, `coordination.md`, `security/sandbox.md`. Workloads embeddings et LLM non portés ; pas d'isolation noyau.
Phase 5 faite (partielle) : règles d'escrow (`sil-chain-core`) et règlement idempotent contre pannes (`sil-settlement`), voir `docs/technical/protocol/settlement.md`. **Programme Solana et adaptateur non écrits** (chaîne d'outils absente).
Phase 6 faite : API de contrôle (`sil-control-api`) et nœud complet (`sil-node`, binaire `silicium-node`), testés de bout en bout sur de vrais nœuds libp2p et la vraie API HTTP (job complet avec pannes de chaîne, worker menteur, perte d'un membre du comité, calcul long avec renouvellement de bail, nœud arrivé tard, présence). Limites levées en fin de phase : renouvellement du bail, rattrapage par curseur, chef de tour, présence, abonnements ciblés, réputation alimentée. Voir `docs/technical/node.md`, `docs/technical/security/control-api.md`, `docs/user/run-a-node.md`. **Toujours aucune connexion à une vraie chaîne.**

Phase 8 (révisée) : site = soumission de calculs par wallet Solana (étapes 1 à 4 : octets signés, login wallet, page Soumettre, comité configuré) ; application de bureau = worker (lance le nœud, données worker via `GET /v1/worker`). Non vérifié : wallet réel, desktop lancé à l'écran. Première version : interface web (`apps/web`, Vue 3, fr/en, WCAG 2.2 AA visé, déclaration d'accessibilité, pages RGPD : export et suppression du compte) et application de bureau (`apps/desktop`, Tauri 2, signature de la liaison de machine par `sil-crypto`). Limites : accessibilité partiellement vérifiée (voir `docs/technical/web-and-desktop.md`), desktop non lancé à l'écran, signature dans le même processus que l'interface, révocation de machine non appliquée par l'API, mentions légales à compléter. Voir `docs/technical/web-and-desktop.md`, `docs/user/desktop-app.md`.

Phase 7 faite : service API Go (`services/api`, bibliothèque standard uniquement) : comptes, sessions, liaison de machine par preuve Ed25519 (vecteur de test partagé avec Rust, `contracts/device-link.md`), envoi du contenu et des événements signés par le client, index des jobs, statut honnête (`unknown` quand le nœud est injoignable, `finalized` ne dit pas que le paiement a eu lieu). Vérifié aussi contre le vrai nœud Rust. Voir `docs/technical/api-service.md`, `docs/user/use-the-api.md`. Limites : pas de TLS, magasin JSON mono-instance, pas de vérification d'e-mail ni de 2FA.
