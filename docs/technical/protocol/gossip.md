# Réseau (`sil-net`)

libp2p : TCP et QUIC, Noise, Yamux. Le crate ne connaît ni le journal ni le stockage ;
le nœud lui passe des traits (`Providers` : validateur de messages, fournisseur
d'événements, fournisseur de contenu).

## Identité

`keypair_from_secret` construit l'identité libp2p depuis la clé **`Purpose::Network`** de
la graine du nœud. Elle est indépendante de la clé du journal (`Purpose::Ledger`).

## Topics (V0 : un seul topic global)

| Topic | Contenu | Taille max |
|---|---|---|
| `sil/v1/presence` | `Presence` (rôles, charge, workloads) | 512 octets |
| `sil/v1/announce` | nouveaux jobs (`JobPublished`) | 40 Kio |
| `sil/v1/jobs/0` … `jobs/15` | événements des jobs dont `job_id[0] % 16 = n` | 40 Kio |

Seize fragments fixes plutôt qu'un topic par job : le scoring gossipsub exige des topics
connus d'avance, et un topic par job laisserait n'importe qui en créer sans limite. Un nœud
s'abonne seulement aux fragments des jobs qu'il suit. Tout autre nom de topic est rejeté.

La **taille maximale est refusée avant l'envoi** (`NetError::TooLarge`) et à la réception.

## Présence

`Presence` fait une trentaine d'octets (V0 : un JSON complet avec télémétrie toutes les
5 s). `PresenceGate` ne laisse publier que si l'état a changé (au plus tous les
`min_gap_ms`) ou après `max_silence_ms` de silence. Les valeurs par défaut sont à choisir
par le nœud ; le crate ne publie pas la présence lui-même.

## Validation avant relais

Les messages sont retenus par gossipsub tant que le nœud n'a pas répondu. Pour chacun :

1. topic inconnu ou message trop grand → **Reject** ;
2. quota du pair dépassé → **Ignore** (aucune pénalité : c'est peut-être une rafale légitime) ;
3. sinon le `MessageValidator` de l'application tranche : `Accept` (livré et relayé),
   `Reject` (mal formé, signature fausse : non relayé), `Ignore` (job encore inconnu).

Seuls les messages acceptés remontent à l'application (`NetEvent::Gossip`).

## Réputation des pairs

Un message **rejeté** coûte cher à son émetteur (pénalité P4, poids −20, au carré du
nombre de rejets) : environ deux rejets font perdre le relais (`gossip_threshold` −10),
trois le font ignorer (`graylist_threshold` −80). La régularité de livraison dans le
maillage n'est pas notée (un pair honnête peut être silencieux). Plusieurs pairs derrière
une même IP au-delà de 8 sont pénalisés.

Conséquence pour le validateur : ne renvoyer `Reject` que pour ce qui est **objectivement
invalide**. Un événement valide dont le job n'est pas encore connu est un `Ignore`.

## Limites par pair

`sil-quota::KeyedLimiter`, mémoire bornée (`max_tracked_peers`) : une table pleine de pairs
épuisés refuse les nouveaux pairs au lieu de les évincer.

| Ressource | Rafale | Débit |
|---|---|---|
| messages gossip | 120 | 40/s |
| requêtes de rattrapage | 10 | 2/s |
| requêtes de contenu | 64 | 32/s |

Connexions : 256 au total, 2 par pair, 64 entrantes en attente
(`libp2p::connection_limits`). Un pair au-delà de sa limite reçoit `Busy`.

## Rattrapage du journal

`/sil/ledger/1`, requête-réponse **point à point** : `LedgerRequest { job, author,
after_sequence, max }` → au plus 64 événements encodés, en ordre de séquence. V0
diffusait la réponse à tout le réseau (amplification).

Second mode, `LedgerRequest::Cursor { job, cursor, max }` : les événements du journal du pair
**dans son ordre d'acceptation** à partir de la position `cursor`. Le demandeur avance son curseur
du nombre d'événements reçus. C'est le rattrapage d'un nœud arrivé tard ou ayant manqué des
messages (le mode par auteur ne sert qu'à combler un trou connu).

## Abonnements et présence

Un nœud s'abonne d'emblée à `presence` et `announce` seulement, puis au fragment de topic de chaque
job qu'il apprend. Les messages `presence` (rôles, charge, workloads, quelques dizaines d'octets)
sont émis sur changement ou après 30 s de silence ; les récepteurs gardent la dernière par pair.

## Contenu et fournisseurs

`/sil/content/1` : `Manifest { id }` et `Chunk { id, index }`. Un morceau de plus de
256 Kio n'est jamais envoyé. Kademlia sert uniquement de **registre de fournisseurs** :
`max_records = 0`, aucun enregistrement n'est accepté ni stocké (V0 y stockait des
événements qui n'étaient jamais relus).

## Identifiant de message

`SHA-256(émetteur ‖ séquence ‖ données)`. V0 utilisait `DefaultHasher` (SipHash à clé fixe),
dont on peut fabriquer des collisions pour faire supprimer le message d'un autre.

## Limites connues

- **Pas de traversée de NAT** : ni relais, ni DCUtR, ni AutoNAT. Un nœud derrière un
  routeur n'est joignable que s'il se connecte lui-même. À traiter avant la mise en service.
- **Pas de découverte mDNS** (la clé `network.enable_mdns` de `sil-config` n'a pas d'effet).
- Pas de transport WebSocket ni DNS : les adresses d'amorçage sont des IP.
- Les paramètres de scoring sont **validés** par gossipsub mais pas éprouvés contre un
  attaquant ; ils sont à calibrer par simulation (phase 9). La limite gossip par pair n'est
  testée qu'unitairement (les tests deux nœuds couvrent rattrapage et contenu).
- Les fournisseurs (`LedgerProvider`, `ContentProvider`) sont appelés dans la boucle réseau :
  ils doivent rester rapides (mémoire, un morceau sur disque).
- Si l'application lit trop lentement, les événements sont abandonnés (journalisé) : la
  boucle réseau n'attend jamais l'application.
