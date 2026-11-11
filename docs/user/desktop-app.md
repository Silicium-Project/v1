# Application de bureau Silicium (worker)

L'application de bureau fait tourner **votre nœud** : elle calcule les tâches que le réseau vous
confie, et vous montre ce que vous avez fait. Vous n'y soumettez pas de calcul : pour cela,
utilisez le site web.

> **Installeur Windows** (`Silicium_0.1.0_x64-setup.exe`) : téléchargeable sur le site. Il installe le
> nœud, le moteur de rendu et le runtime ONNX (embedding). Vérifiez la somme SHA-256 publiée à côté
> de l'installeur. Un avertissement SmartScreen peut apparaître : le certificat est celui du projet
> étudiant. Sous Linux, les paquets `.deb` et AppImage contiennent la même chose.
> Pour compiler vous-même, voir `apps/desktop/README.md`. Le programme du nœud (`silicium-node`) doit
> être à côté de l'application, ou désigné par la variable `SIL_NODE_BIN`.

## 1. Lier la machine à votre compte

1. Sur le site, ouvrez **Machines**, cliquez sur **Générer un code de liaison**, puis sur
   **Lier cette machine**. Le code est valable 10 minutes et ne sert qu'une fois.
2. Votre navigateur demande d'ouvrir l'application de bureau : acceptez. L'application affiche le
   compte auquel appartient le code et la clé de cette machine.
3. Vérifiez le compte, donnez un nom (facultatif) et cliquez sur **Lier cette machine à ce compte**.

Si l'application ne s'ouvre pas, saisissez le code à la main dans **Lier cette machine**.
N'acceptez jamais un code que vous n'avez pas généré vous-même : il lierait cette machine à un
autre compte.

La clé qui signe la liaison est la clé du nœud : seules la clé publique et la signature quittent
la machine.

## 2. Activer ce PC

Ouvrez **Tableau de bord**, puis cliquez sur **Activer mon PC**. Le nœud démarre en arrière-plan. La
première fois, il crée son jeton et son identité ; les données apparaissent en quelques secondes.
Le journal technique est dans `node.log`, dans le dossier de données de l'application.

**Désactiver ce PC** arrête le nœud. Le nœud s'arrête aussi quand vous fermez l'application.

## 3. Lire le tableau de bord

Une fois le PC activé, le **Tableau de bord** affiche, mis à jour toutes les 5 secondes :

- **Réputation bayésienne** : 0,50 pour un nouveau nœud. C'est la moyenne a posteriori d'une loi
  Beta(1,1) : elle monte avec les tâches finalisées et baisse avec les échecs. Les succès et
  échecs pondérés qui la fondent sont indiqués à côté.
- **Puissance utilisée** : un nœud neuf fait 4 tâches à la fois (plafond borné par les cœurs − 1 ;
  une réputation nulle interdit toute tâche). Le nombre de tâches (calcul ou vérification) en cours, sur le nombre
  autorisé par votre réputation, et le nombre de cœurs de la machine.
- **Gains** : non mesurés. Les règlements passent par la chaîne de test (devnet Solana) : aucun
  montant réel n'est affiché.
- **Tâches en cours** : le calcul, le morceau, l'étape (calcul, résultat en attente de vérification,
  validé, litige) et depuis quand.
- **Historique des tâches** : l'issue de chaque tentative, les 50 plus récentes. « Finalisée » est
  la seule issue retenue par le réseau.

Plus bas, le détail par type d'action, les 50 actions les plus récentes et les règlements vus.

## Ce qui se passe si ça échoue

| Message | Que faire |
|---|---|
| Le programme du nœud est introuvable | installez `silicium-node` à côté de l'application, ou définissez `SIL_NODE_BIN` |
| Le nœud n'a pas pu démarrer | lisez `node.log` dans le dossier de données |
| Le nœud ne répond pas encore | patientez ; la page réessaie seule |
| Le nœud a refusé le jeton | redémarrez le nœud depuis le tableau de bord |
| Code de liaison inconnu, expiré ou déjà utilisé | générez un nouveau code sur le site |

## Perte ou revente de la machine

Révoquez la machine depuis **Machines** sur le site, et arrêtez le nœud avant de la prêter ou de
la vendre. **Limite actuelle** : le service ne vérifie pas encore les signatures des machines sur
les calculs ; la révocation marque la machine, elle ne la bloque pas encore.
