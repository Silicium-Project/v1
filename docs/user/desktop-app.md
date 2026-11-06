# Application de bureau Silicium (worker)

L'application de bureau fait tourner **votre nœud** : elle calcule les tâches que le réseau vous
confie, et vous montre ce que vous avez fait. Vous n'y soumettez pas de calcul : pour cela,
utilisez le site web.

> Aucun installeur n'est encore publié. Pour l'instant, l'application se compile depuis les
> sources (`apps/desktop/README.md`). Le programme du nœud (`silicium-node`) doit être à côté de
> l'application, ou désigné par la variable `SIL_NODE_BIN`.

## 1. Lier la machine à votre compte

1. Sur le site, ouvrez **Machines** et cliquez sur **Générer un code de liaison**. Le code est
   valable 10 minutes et ne sert qu'une fois.
2. Dans l'application de bureau, section **Lier cette machine** : saisissez le code, donnez un nom
   (facultatif) et cliquez sur **Lier cette machine**.

La clé qui signe la liaison est la clé du nœud : seules la clé publique et la signature quittent
la machine.

## 2. Démarrer le nœud

Ouvrez **Travail**, puis **Démarrer le nœud**. La première fois, le nœud crée son jeton et son
identité ; les données apparaissent en quelques secondes. Le journal technique est dans
`node.log`, dans le dossier de données de l'application.

Le nœud s'arrête quand vous fermez l'application.

## 3. Lire vos données

La page **Travail** affiche :

- vos rôles (calcul, vérification, coordination) ;
- le nombre de calculs auxquels votre nœud a participé ;
- le détail par type d'action (fragment réclamé, résultat envoyé, vérification…) ;
- les 50 actions les plus récentes ;
- votre réputation (0,50 pour un nouveau nœud, elle monte avec les résultats acceptés) ;
- les règlements vus.

**Gains** : aucun paiement n'est actif pour l'instant. Aucun montant n'est affiché tant que les
règlements sur Solana ne sont pas en service.

## Ce qui se passe si ça échoue

| Message | Que faire |
|---|---|
| Le programme du nœud est introuvable | installez `silicium-node` à côté de l'application, ou définissez `SIL_NODE_BIN` |
| Le nœud n'a pas pu démarrer | lisez `node.log` dans le dossier de données |
| Le nœud ne répond pas encore | patientez ; la page réessaie seule |
| Le nœud a refusé le jeton | redémarrez le nœud depuis la page Travail |
| Code de liaison inconnu, expiré ou déjà utilisé | générez un nouveau code sur le site |

## Perte ou revente de la machine

Révoquez la machine depuis **Machines** sur le site, et arrêtez le nœud avant de la prêter ou de
la vendre. **Limite actuelle** : le service ne vérifie pas encore les signatures des machines sur
les calculs ; la révocation marque la machine, elle ne la bloque pas encore.
