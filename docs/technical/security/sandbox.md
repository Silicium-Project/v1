# Exécution de programmes externes (`sil-sandbox`)

## Ce qui est garanti dans tous les cas (testé)

| Garantie | Test |
|---|---|
| Environnement **vide**, seule la liste blanche est transmise (rien du nœud n'est hérité) | `the_node_environment_is_not_inherited_only_the_allowlist` |
| Le programme ne trouve pas d'autres programmes sans `PATH` explicite | `without_an_explicit_path_…` |
| Répertoire de travail imposé et obligatoire | `the_working_directory_is_the_one_given` |
| **Délai** : le programme et ses enfants sont tués au-delà de `wall` (groupe de processus) | `a_slow_process_is_killed_at_the_deadline`, `a_timeout_kills_the_program_and_its_children` |
| **Sortie plafonnée** : dépassement = processus tué | `runaway_output_kills_the_process` |
| Entrée standard transmise ; programme absent = erreur typée | `stdin_is_forwarded`, `a_missing_program_…` |
| `Confined::resolve` : chemins relatifs sûrs seulement | `paths_are_confined_to_the_root` |
| Liens symboliques et sorties de racine refusés | `symbolic_links_inside_the_root_are_refused` |

Le code est entièrement sûr (`unsafe_code = forbid` pour tout l'espace de travail).

## Isolation noyau sous Linux (appliquée, testée)

Quand une `Isolation` est donnée, le programme est lancé par `sil-sandbox-launch`. Ce lanceur
applique la politique au processus courant, puis remplace ce processus par le programme (`exec`).
Tout ce qui est appliqué est hérité par le programme.

| Protection | Mécanisme | Test |
|---|---|---|
| Pas de réseau : création de sockets IPv4/IPv6/netlink/paquets refusée | seccomp, `socket` renvoie `EPERM` | `isolated_probe_cannot_open_a_network_socket` |
| Écriture seulement dans le dossier de travail | Landlock (droits de fichier, de dossier) | `isolated_probe_cannot_write_outside_the_workdir`, `isolated_probe_can_write_inside_the_workdir` |
| Lecture seulement dans les chemins listés (programme, bibliothèques) | Landlock | `isolated_probe_cannot_read_files_outside_the_allowed_paths` |
| Mémoire virtuelle bornée | `RLIMIT_AS` | `isolated_probe_cannot_allocate_beyond_the_memory_limit` |
| Taille de fichier et temps CPU bornés | `RLIMIT_FSIZE`, `RLIMIT_CPU` | (appliqués, pas de test dédié) |
| Arbre tué au délai | groupe de processus, `killpg` | `an_isolated_timeout_kills_the_program_and_its_children` |

Si Landlock ou seccomp ne sont pas disponibles sur le noyau, le lancement **échoue** : jamais
d'exécution partiellement isolée. Noyau requis : Linux 5.13 ou plus récent (Landlock ABI V1).

## Moteur WebAssembly (`sil-render-wasm`, toutes plateformes)

Un moteur `.wasm` (WASI preview 1, commande `_start`) est exécuté par wasmtime. L'isolation est
**par construction** : même comportement sur Windows, Linux et macOS, sans réglage ni droits
système.

| Garantie | Test |
|---|---|
| Rendu exact (mêmes octets que la référence) | `the_wasm_engine_renders_exactly_the_reference_pixels` |
| Délai par bloc : interruption epoch, trap `Interrupt` | `a_slow_wasm_engine_is_interrupted_at_the_deadline` |
| Sortie trop grande : refusée par le tube, code 12 remonté en `OverLimit` | `a_wasm_engine_with_runaway_output_is_cut_off` |
| Sortie trop courte : bloc refusé | `a_wasm_engine_that_returns_too_few_bytes_is_refused` |
| Pas de réseau (WASI p1 n'a pas de sockets) | `a_wasm_engine_has_no_network` |
| Pas de fichiers (aucun dossier pré-ouvert) | `a_wasm_engine_cannot_write_to_the_filesystem` |
| Mémoire bornée par instance (256 Mio dans le nœud) | `a_wasm_engine_cannot_allocate_beyond_its_memory_limit` |

Pas d'environnement, pas d'arguments hérités : seuls les quatre paramètres du protocole sont passés.
Le module est compilé une fois ; chaque bloc reçoit une instance neuve.

Construire le moteur de référence : `cargo build -p sil-wl-render --bin sil-render-reference --target wasm32-wasip1`
(cible `wasm32-wasip1` installée : `rustup target add wasm32-wasip1`).

Limite : un moteur WebAssembly ne peut pas accéder au GPU ni aux bibliothèques natives. Un moteur
existant en natif doit être porté en WASM pour être utilisé ici.

## Isolation sous Windows (partielle, moteurs natifs seulement)

Un Job Object contient le programme et ses enfants : sa fermeture tue tout l'arbre au délai.
Testé par `best_effort_timeout_kills_the_job_tree`.

**Non appliqué sous Windows** : limite de mémoire, limite du nombre de processus, réseau,
système de fichiers. `win32job` n'expose pas les limites dures ; les poser demande du code
`unsafe` (interdit dans ce projet) ou AppContainer (non fait). Pour cette raison, `strict`
refuse de lancer sous Windows (`strict_isolation_is_refused_on_windows`) : il faut `best-effort`
(ou `off`) en connaissance de cause.

## Moteur de rendu (`rendu/blocs`)

Le rendu est le seul workload qui exécute du code externe. Le moteur est un programme que
l'opérateur installe et désigne dans `[render] engine` (voir `docs/user/configuration.md`). Le nœud
le lance pour chaque bloc via `sil-wl-render::ExternalRenderer`, donc dans `sil-sandbox`, avec
l'isolation `[render] isolation` (défaut : `strict`).

Protocole : arguments `<largeur> <hauteur> <y0> <y1>`, scène sur l'entrée standard, exactement
`largeur × (y1 - y0) × 3` octets RGB24 sur la sortie standard.

| Garantie (en plus de celles ci-dessus) | Test |
|---|---|
| Le rendu d'un bloc est exactement celui du moteur de référence | `the_engine_renders_exactly_the_reference_pixels` |
| Sortie trop grande : moteur tué | `an_engine_with_runaway_output_is_cut_off` |
| Sortie trop courte : bloc refusé (non réglé) | `an_engine_that_returns_too_few_bytes_is_refused` |
| Délai par bloc (`wall_ms`) : moteur tué | `a_slow_engine_is_killed_at_the_deadline` |
| Moteur absent : erreur, pas de panique | `a_missing_engine_is_an_error_not_a_panic` |
| Le moteur n'écrit que dans son dossier de travail, jamais dans `data_dir` (clés) | `isolated_probe_cannot_write_outside_the_workdir` |

Le moteur de référence (`sil-render-reference`) et les modes de panne (`slow`, `flood`, `short`)
ne servent qu'aux tests. Le workload sha256 reste dans le processus du nœud : c'est du code du projet.

## Ce qui reste non couvert

- **Windows** : mémoire, nombre de processus, réseau, fichiers (voir ci-dessus).
- **Linux** : un programme peut toujours consommer le CPU jusqu'à `cpu_seconds`, et utiliser la
  mémoire jusqu'à `memory_bytes`, ce qui est voulu. Le réseau reste fermé uniquement pour les
  sockets IPv4/IPv6/netlink/paquets : les sockets UNIX restent possibles.
- `Confined` juge l'état du disque à l'instant de l'appel : il ne protège pas contre un tiers qui
  modifie le dossier entre la résolution et l'ouverture (course TOCTOU).
- Pas d'audit de la configuration Landlock/seccomp par un tiers. Les règles ont été écrites et
  testées ici, sur WSL2 (noyau 6.6) et Windows 11.
- macOS : aucune isolation (pas de backend).

**Conséquence** : un moteur installé par l'opérateur reste de confiance ; un moteur non fiable
demande un backend complet (Windows : AppContainer ou équivalent ; macOS : à écrire).

## Notes de test

Le test des liens symboliques est **ignoré** quand le système refuse d'en créer (droits Windows) ;
il tourne sous Linux/macOS et sous Windows en mode développeur.

Les tests d'isolation Linux tournent sous WSL2 (`cargo test -p sil-sandbox`) ; ceux de Windows
sous Windows. Les sondes sont `sil-sandbox-probe` (`crates/sil-sandbox/src/bin/`).
