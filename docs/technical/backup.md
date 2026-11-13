# Protocole de sauvegarde des données du nœud

Les données d'un nœud (identité, journal, règlements en attente, base de l'API) ne se perdent plus
lors d'une mise à jour. Ce document dit ce qui est sauvegardé, quand, et comment le restaurer.

## Règles

1. **Pas de déploiement sans sauvegarde réussie.** `infra/vps/deploy.sh` et `infra/vps/deploy.ps1`
   appellent `infra/vps/backup.sh` avant toute copie ; un échec annule le déploiement.
2. **Aucune donnée n'est déplacée, renommée ou supprimée par un script de déploiement.** Seuls les
   binaires, les unités systemd et le site sont remplacés.
3. **Un changement de format de journal n'est jamais automatique.** Il se fait par une procédure
   séparée, décidée explicitement, après une sauvegarde vérifiée.
4. **Le poste garde les sauvegardes**, dans `~/silicium-backups/` (VPS) et
   `%USERPROFILE%\silicium-backups\` (bureau). Elles ne sont jamais supprimées automatiquement.
5. **`node.seed` est l'identité du nœud** : le perdre crée un nouveau nœud, sans réputation.

## Ce qui est sauvegardé

| Machine | Données | Exclu (régénérable) |
|---|---|---|
| VPS | `/var/lib/silicium/node` (`node.seed`, `control.token`, `ledger`, `settlement`), `/var/lib/silicium/api` (`silicium.db`), `/etc/silicium` (`node.toml`, `api.env`) | `render-work`, `content`, `embed` |
| Bureau (Windows) | `%APPDATA%\org.silicium.desktop\node` (`node.seed`, `ledger`, `settlement`, `link.json`, `node.toml`) | `render-work`, `content`, `embed` |

Les sauvegardes VPS sont **en lecture seule** : rien n'est écrit sur le serveur. Chaque sauvegarde
contient un `SHA256SUMS` ; le script vérifie les sommes et échoue si une donnée attendue manque.

## Commandes

Sauvegarde du VPS seule (depuis la racine du dépôt, dans Git Bash) :

```bash
SIL_SSH_KEY=~/.ssh/silicium_deploy_dev2 infra/vps/backup.sh vps-ovh
```

Sauvegarde du bureau, application fermée de préférence (PowerShell) :

```powershell
infra\desktop\backup.ps1
```

## Restauration

VPS (arrêter le service avant) :

```bash
sudo systemctl stop silicium-node silicium-api
sudo tar -C /var/lib/silicium -xzf donnees.tgz
sudo tar -C /etc -xzf configuration.tgz
sudo systemctl start silicium-node silicium-api
```

Bureau : fermer l'application, puis recopier le contenu de `desktop-<date>\node` dans
`%APPDATA%\org.silicium.desktop\node` (en gardant le dossier actuel renommé, jamais écrasé).

## Limite connue

Un journal écrit avant la version 1.8 ne se relit pas avec la version 1.8 : le format des
événements a changé (claims ancrés, verdicts signés en binaire fixe). Restaurer un ancien journal
ne suffit donc pas à retrouver l'historique réseau avec la version actuelle. La sauvegarde protège
contre la perte, pas contre l'incompatibilité de format.
