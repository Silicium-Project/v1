# API de contrôle (`sil-control-api`)

API HTTP locale par laquelle le service Go (ou un outil) pilote un nœud.

## Ce qui change par rapport à V0

| V0 | V1 |
|---|---|
| Jeton comparé avec `!=` | Tous les jetons comparés en **temps constant** (`ct_eq`), sans s'arrêter au premier trouvé |
| Un seul jeton, tous droits | **Scopes** : `read` (lire l'état), `submit` (envoyer contenu et événements) |
| Sans jeton = API ouverte sur la boucle locale | **Jamais d'accès anonyme** hors `/health` et la description OpenAPI |
| `scene_path` absolu accepté : le nœud lisait n'importe quel fichier, le stockait par hash et le servait aux pairs (fuite de `identity.key`, wallet…) | **Aucun chemin accepté.** Le contenu est envoyé **en octets** (`PUT /v1/content`) |
| Aucune limite | Débit par jeton, taille des corps, freinage de l'essai de jetons |
| Description écrite à la main | **OpenAPI généré** de la table des routes (source unique du routeur) |

## Routes

| Méthode et chemin | Scope | Rôle |
|---|---|---|
| `GET /health` | publique | le nœud répond |
| `GET /v1/openapi.json` | publique | description générée |
| `PUT /v1/content` | `submit` | envoie un contenu (octets), retourne `content_id` |
| `POST /v1/events` | `submit` | envoie un événement signé (CBOR canonique) ; le nœud vérifie, stocke, relaie |
| `GET /v1/jobs/{id}` | `read` | état dérivé d'un job |
| `GET /v1/peers` | `read` | pairs connectés |
| `GET /v1/settlements` | `read` | état des règlements |
| `GET /v1/worker` | `read` | activité de ce nœud comme worker : événements par type, 50 récents, réputation et ses succès/échecs, capacité, tâches en cours, historique (50), règlements vus, `payments: false` |

Le nœud **ne signe rien pour le client** : le requester signe lui-même la publication du job
(`JobPublished`) et ses annulations ; l'API ne fait que les transmettre.

## Jetons

Fichier, une ligne par jeton : `scope1,scope2:secret`, `#` pour commenter. Secret de 32 à 128
caractères imprimables. Au premier démarrage, si le fichier n'existe pas, le nœud en crée un
(`read,submit`, 256 bits aléatoires, mode 0600 sous Unix) et ne le réécrit jamais.

```
# jeton de lecture seule pour la supervision
read:<secret de 32 caractères au moins>
# jeton du service Go
read,submit:<autre secret>
```

## Limites

- Débit par jeton : rafale de 60, 20 par seconde. Essais de jetons invalides : un seul seau global
  (rafale 20, 5 par seconde), puis `429`.
- Corps : contenu 64 Mio, événement 64 Kio (`413` au-delà, avant d'être lu en entier).
- Les appels du nœud sont exécutés hors de la boucle asynchrone (`spawn_blocking`).

## Limites connues

- **HTTP simple, sans TLS** : à n'exposer que sur la boucle locale ou derrière un proxy TLS. La
  validation de configuration (`sil-config`) exige un fichier de jetons si l'adresse n'est pas
  locale, mais ne force pas TLS.
- Le débit est limité **par jeton**, pas par adresse IP.
- Pas de révocation ni de rotation de jeton à chaud : il faut redémarrer.
- Le contenu est reçu en mémoire avant d'être stocké (jusqu'à 64 Mio par requête).
