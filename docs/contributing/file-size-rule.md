# Règle de taille des fichiers

Aucun fichier de code, de test, de configuration ou de documentation ne dépasse
**300 lignes**. `python tools/check_size.py .` l'applique ; la CI échoue sinon.

Si un fichier grossit : découper par responsabilité (nouveau module ou crate), pas
raccourcir les lignes. Exclus : `Cargo.lock`, `package-lock.json`, `go.sum`, dossiers
générés (`target`, `node_modules`, `dist`).
