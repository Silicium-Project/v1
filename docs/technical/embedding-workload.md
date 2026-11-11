# Workload embedding et vérification tolérante

Statut (novembre 2026) : **implémenté et publié** (`embedding/e5-small`). Ce document garde la spec d'origine et les mesures ; la section « État réel » donne ce qui est en place, et ce qui diffère de la spec.

## État réel

| Élément | En place | Référence |
|---|---|---|
| Descripteur `embedding/e5-small`, politique `sampling_tolerance` | oui (`min_cheat_fraction` 0,1 ; `target_detection` 0,97 ; `cos_min` 0,999) | `contracts/workloads/embedding-e5-small.json` |
| Modèle fp32 épinglé par SHA-256 (`ca456c06…8665`) | oui, téléchargé au premier démarrage, hash vérifié | `crates/sil-model`, `crates/sil-wl-embed/src/lib.rs` |
| Tokenizer épinglé (`0b44a9d7…4c39`) | oui | `crates/sil-wl-embed/src/lib.rs` |
| ONNX Runtime 1.31.1 | oui, livré dans les paquets (installeur, deb, AppImage, archives) | `apps/desktop/scripts/prepare-runtime.mjs`, `tools/runtime/fetch_onnxruntime.py` |
| Vérification par tolérance (cosinus par vecteur, racine d'échantillon) | oui | `crates/sil-wl-embed/src/workload.rs` (`verify_tolerance`) |
| Mode `tolerance` du certificat et du ledger | oui | `crates/sil-ledger/src/tolerance.rs`, `crates/sil-attest/src/tolerance.rs` |
| Au moins 2 vérificateurs par fragment (§10.8, décision 3) | **non** : `services/api/internal/policy/policy.go` fixe `VerificationsRequired = 1` | à décider |
| Int8 | abandonné (§9) | — |

Écarts avec la spec d'origine :
- Un passage de plus de 512 tokens est **tronqué** (troncature du tokenizer, `MAX_TOKENS = 512`), pas découpé en chunks.
- Entrée : un ou plusieurs fichiers, de tous types. Le texte de chaque fichier est extrait en entier dans le navigateur (`documents.ts`) avant envoi ; le nœud reçoit toujours `passages.txt`. Les parseurs ne tournent donc pas sur les nœuds. Un fichier sans texte (image, binaire, PDF scanné) fait échouer l'envoi en le nommant.
- Résultat : `/v1/jobs/{id}/result` renvoie les vecteurs bruts (`application/octet-stream`, `.bin`) : une sortie hexadécimale ne passe pas pour plus de 2 passages par fragment (limite de l'API).
- Le préfixe `passage: ` est ajouté par le nœud à chaque texte. Le site n'envoie pas de préfixe.
- Le modèle (470 268 510 octets) n'est **pas** dans les paquets : le nœud le télécharge au premier démarrage et refuse tout fichier dont le hash diffère.

Tests : les cas de mode, de seuil et de rejeu sont couverts par `crates/sil-ledger/tests/tolerance.rs` et `crates/sil-attest`. La graine (§10.7, test 5) n'a pas de test dédié. Le test de référence (`crates/sil-wl-embed/tests/reference.rs`, `--ignored`) reproduit les vecteurs mesurés avec ONNX Runtime 1.31.1 à 1e-5 près.

## 1. Problème

Les workloads actuels sont vérifiés par égalité d'octets : le vérificateur refait le calcul et compare le hash. Un flottant ne se reproduit pas à l'identique d'une machine à l'autre (CPU, jeux d'instructions SIMD, nombre de threads, build). Un embedding produit des flottants, donc la vérification actuelle rejette des résultats honnêtes.

Objectif : accepter les workloads à résultat flottant en vérifiant par **tolérance**, sans ouvrir de fraude.

## 2. Politique actuelle (inchangée)

- `calcul/sha256`, `rendu/raytracer`, `image/batch` : résultat bit-exact, vérifié par recalcul et égalité de hash.
- Rien ne change pour ces workloads.

## 3. Nouvelle politique : `verify = tolerance`

Chaque workload déclare sa politique dans son descripteur (`contracts/workloads`). Une seule valeur par workload, pas de choix à l'exécution.

Le vérificateur :

1. Tire `k` éléments du shard avec une graine dérivée de `(job, fragment)`. Le tirage est imprévisible pour le worker avant son résultat.
2. Recalcule ces `k` éléments avec le **même modèle figé**.
3. Compare chaque vecteur reçu au vecteur recalculé avec la métrique du descripteur.
4. Approuve si **tous** les vecteurs tirés passent. Un seul échec rejette le fragment.

Métrique : similarité cosinus minimale `cos_min` (valeur fixée par modèle, ex. `0.999`). Garde-fous fixes, hors descripteur :

- dimension exacte, nombre d'éléments exact, pas de `NaN` ni `Inf` ;
- norme L2 de chaque vecteur dans une plage fixée ;
- refus immédiat d'un résultat de taille ou de format faux.

## 4. Risque principal : la tolérance est une fenêtre de fraude

Un worker peut renvoyer des vecteurs légèrement faux, dans la tolérance, sans recalculer. Mitigations, par ordre d'importance :

- **Seuil serré et propre à chaque modèle**, validé par mesure : écart entre machines honnêtes (Linux, Windows, CPU de génération différente) doit être nettement sous `1 - cos_min`.
- **Déterminisme maximal côté nœud** : même fichier modèle (hash figé dans le descripteur), threads fixés à 1 pour l'inférence, pas de téléchargement à la volée. Réduit l'écart, ne le supprime pas.
- **Contrôle croisé** : plusieurs vérificateurs tirent des éléments différents. Un fragment frauduleux finit par être pris, sur un échantillon ou un autre.
- **Mesure avant ouverture** : taux de faux rejets et taux de fausses acceptations, mesurés sur un corpus de test, consignés dans la spec du modèle. Pas de workload ouvert sans ces chiffres.

## 5. Modèle figé

- Le modèle n'est pas choisi par l'utilisateur : il est dans le descripteur, avec son SHA-256 et sa licence.
- Modèle retenu : **`intfloat/multilingual-e5-small`** (sources : fiche Hugging Face, fichiers ONNX).
  - Licence MIT (usage commercial libre).
  - 117,65 M paramètres, 12 couches, 384 dimensions.
  - Entrée max 512 tokens. Un passage plus long est tronqué (voir « État réel »).
  - 100 langues (base xlm-roberta). Score Mr. TyDi moyen : 64,4.
  - **Préfixe obligatoire** : chaque texte commence par `query: ` ou `passage: `, y compris hors anglais. Le workload utilise `passage: ` (figé dans `sil-wl-embed`).
  - Pooling : moyenne masquée (average pooling), puis normalisation L2.
- Variantes ONNX et taille à distribuer :
  - fp32 : `model.onnx`, 448 MB.
  - int8 publié : `model_qint8_avx512_vnni.onnx`, 113 MB. Cette variante est liée aux instructions AVX-512 VNNI : **non retenue** pour un modèle figé multi-machines.
  - Décision appliquée : fp32 (§9, troisième passage).
- Runtime d'inférence : ONNX Runtime 1.31.1 (crate `ort` 2.0.0-rc.2, `ort-sys` épinglé), threads fixés à 1, chargé dynamiquement depuis le dossier du binaire.
- Impact paquet nœud : la bibliothèque ONNX Runtime seulement (19 Mo Windows, 29 Mo Linux). Le modèle fp32 (448 MiB) n'est pas dans le paquet.

## 6. Format du résultat

- Vecteurs en `f32`, little-endian, dans l'ordre des éléments du shard, avec en-tête (modèle, dimension, nombre).
- Le hash du résultat engage les octets reçus. Il n'est **pas** comparé entre workers : la comparaison se fait par tolérance.
- Conséquence protocole : une approbation signifie « dans la tolérance du recalcul », plus « octets identiques ». Voir section 10 pour le détail. Correction après relecture du code : `certificate.rs:43` compare le hash du résultat de chaque approbation au résultat du certificat, c'est-à-dire **le résultat du worker**. Ce contrôle reste valable en mode tolérance. Le vrai blocage est ailleurs : `AttestationBody` n'a pas de champ de preuve pour une approbation (`evidence` est interdit sur un verdict positif, `attestation.rs` `check_shape`).

## 7. Questions ouvertes (à trancher avant implémentation)

1. Valeur de `cos_min` et méthode de mesure du seuil.
2. Nombre `k` d'éléments tirés par fragment, et son effet sur le coût.
3. Modèle retenu, après mesure de déterminisme et de licence.
4. Le certificat du ledger et le règlement on-chain acceptent-ils une approbation par tolérance sans changer de format ? Sinon, upgrade du programme devnet.
5. Taille maximale d'un modèle distribué aux nœuds.

## 8. Ordre d'implémentation (après validation de cette spec)

1. Mesure de déterminisme : `multilingual-e5-small`, fp32 et int8 générique, sur deux machines (Linux, Windows), mêmes textes avec préfixe. Mesurer le cosinus minimal entre machines et le cosinus minimal entre deux machines qui ne sont pas du même CPU. Rien d'autre avant ce chiffre.
2. Crate `sil-verify-tolerance` : métrique, garde-fous, tirage d'échantillon. Tests avec vecteurs fixes.
3. Descripteur et politique `tolerance` dans `contracts/workloads` et génération.
4. Workload embedding dans le nœud, puis verifier loop.
5. Certificat et règlement : décision de la question 4.
6. API, web, docs, puis upgrade devnet si nécessaire.

Ces étapes sont faites. Les questions 1 et 3 sont tranchées par les mesures de §9 (`cos_min` 0,999 ; fp32).

## 9. Résultat de l'étape 1 (mesure de déterminisme)

Protocole : `multilingual-e5-small`, 36 textes (fr, en, de, es, ja, zh, ar, sw, ru, code, chiffres, texte long, cas limites), préfixe `passage: `, un texte par passe, 1 thread, ONNX Runtime 1.23.2 identique des deux côtés, pooling moyenne masquée, L2. Scripts et sorties hors dépôt (scratchpad).

| Comparaison | Cosinus min | Diff max absolue |
|---|---|---|
| fp32, run à run (Windows) | 0,9999999 | 0 |
| fp32, Windows vs WSL Linux | 0,9999999 | 0 |
| int8 générique, Windows vs WSL Linux | 0,9999998 | 0 |
| fp32 vs int8 générique (Windows) | **0,9750** | 3,6e-2 |

### Second passage : CPU différents, même version ONNX Runtime

Hôtes : Windows sur AMD Ryzen 9 4900H (Zen 2) ; VPS sur Intel Haswell (virtualisé). Les deux ont AVX2 et **pas** AVX-512. Python 3.12 et ONNX Runtime 1.31.1 identiques des deux côtés (uv).

| Comparaison | Cosinus min | Diff max absolue |
|---|---|---|
| fp32, Windows vs VPS (CPU différents) | 0,9999998 | 0 |
| int8 générique, Windows vs VPS (CPU différents) | 0,9999998 | 0 |
| fp32, ORT 1.23.2 vs ORT 1.31.1 (même hôte) | 0,9999999 | 1,2e-7 |
| fp32 vs int8 générique (même hôte) | **0,9764** | 3,7e-2 |

Lecture :
- Entre deux CPU AVX2, à version ONNX Runtime fixée : **bit à bit identiques**, fp32 et int8.
- Changer de version ONNX Runtime change les flottants (1,2e-7). Donc la version est à figer dans le descripteur, comme le hash du modèle.
- Int8 générique par quantification dynamique par défaut : cosinus min 0,976, sous `cos_min = 0,999`. **Non viable en l'état.** Il est reproductible, mais trop imprécis.
- Non testé : une machine **AVX-512** (le noyau de calcul change). Le fichier int8 `avx512_vnni` de Hugging Face montre que ça compte. Sans cette mesure, on ne peut pas dire que le déterminisme tient sur tous les CPU.

### Troisième passage : int8 à meilleure précision (ONNX Runtime 1.31.1, Windows)

| Variante int8 | Cosinus min vs fp32 | Cosinus moyen |
|---|---|---|
| dynamique, par tenseur (générique) | 0,9764 | 0,9894 |
| dynamique, par canal | 0,9881 | 0,9958 |
| statique, par canal, calibration sur les 36 textes, format QOperator | 0,8092 | 0,8680 |

Lecture :
- Aucune variante int8 n'atteint `cos_min = 0,999`. La meilleure (dynamique par canal) donne 0,988.
- La statique QDQ par canal ne se charge pas sur ONNX Runtime 1.31.1 (attribut `axis` de `DequantizeLinear` refusé). En QOperator, elle est pire, probablement une calibration trop petite (36 textes) ou une quantification des activations trop grossière.
- **Conclusion : int8 abandonné pour l'instant. Le modèle distribué sera fp32** (448 MB, bit à bit identique entre AVX2 Zen 2 et Haswell à version ONNX Runtime fixée).
- Int8 reste ouvert si on veut le poids : calibration sur un corpus beaucoup plus grand, ou seuil `cos_min` plus bas (à justifier par la mesure de faux rejets).

Étape 1 : faite pour AVX2, fp32 retenu.

**AVX-512 : non bloquant.** Le mode `tolerance` absorbe les écarts de flottants : mesurés entre versions et CPU AVX2, ils sont de l'ordre de 1e-7, soit environ 10 000 fois sous `1 - cos_min`. Un noyau AVX-512 change l'ordre des additions, donc le même ordre de grandeur. C'est un raisonnement, pas une mesure.

Mitigation : chaque vérificateur journalise le cosinus minimal de chaque fragment (honnêtes et rejetés). Si un nœud honnête sort sous 0,999 de façon répétée, on revoit le seuil. Pas de machine AVX-512 nécessaire pour lancer la v1.

## 10. Certificat à deux modes (`exact` et `tolerance`)

Principe : deux modes, fixés **par workload** dans le descripteur. Le worker ne choisit pas son mode. Un vérificateur ne peut pas rétrograder un workload `tolerance` en `exact`, ni l'inverse.

### 10.1 Compatibilité

- **Mode `exact`** : format et domaine de signature actuels, inchangés. Les attestations existantes restent valides.
- **Mode `tolerance`** : nouveau domaine de signature, `sil/v1/attestation-tolerance`. Une attestation `exact` ne peut pas être rejouée comme `tolerance` (domaines distincts), et réciproquement.

### 10.2 Corps d'une attestation `tolerance`

Champs signés (encodage à largeur fixe, entiers pour tout ce qui doit être bit-exact) :

- `task`, `attempt`, `result` : identiques au mode `exact`. `result` est le hash du résultat du worker.
- `model_hash` : SHA-256 du fichier modèle, figé dans le descripteur.
- `ort_version` : version ONNX Runtime, figée (1,2e-7 de différence observée entre 1.23.2 et 1.31.1).
- `sample_seed` : graine du tirage (voir 10.3).
- `sample_root` : hash de la liste des indices tirés et des vecteurs que le vérificateur a recalculés.
- `min_cos_micro` : cosinus minimal observé, en millionièmes (entier signé, pas de flottant dans le message).
- `approved` : vrai si `min_cos_micro >= cos_min_micro` du descripteur.
- `evidence` : hash de la preuve de divergence, obligatoire si `approved` est faux, interdit sinon (même règle qu'aujourd'hui).

### 10.3 Tirage d'échantillon : réutiliser l'existant

Le code a déjà ce mécanisme (`sil-sampling`) : `SamplingSeed::draw` dérive la graine de la **signature du vérificateur** sur `(job, fragment, attempt, result)`. Le worker ne peut pas la prévoir sans la clé privée du vérificateur, donc pas de grinding. Pas besoin d'un identifiant d'offre. Chaque vérificateur tire son propre échantillon.

Taille de l'échantillon : `samples_needed(total, min_cheat_fraction, target)` (`sil-sampling/src/math.rs`). Elle dépend du nombre d'éléments du fragment, pas d'un `k` fixe. Pour 20 % de triche et 99 % de détection, environ 21 éléments.

### 10.4 Rejet et contrôle croisé

- Un rejet est signé avec `evidence` : hash des indices où le cosinus passe sous le seuil, et des deux vecteurs.
- Un vérificateur qui ne répond pas ne compte pas. Le fragment reste à approuver par les autres, comme aujourd'hui.

### 10.5 Certificat

- `Certificate` ajoute un champ `mode`, lu depuis le descripteur, pas depuis le certificat.
- `Certificate::verify` : si `mode = exact`, comportement actuel. Si `mode = tolerance`, chaque approbation est vérifiée avec son domaine, et la preuve (`sample_root`, `min_cos_micro`) est contrôlée : cosinus au-dessus du seuil du descripteur, modèle et version ONNX Runtime égaux à ceux du descripteur.
- Le contrôle `certificate.rs:43` (hash du résultat égal entre approbations et certificat) est conservé.

### 10.6 Règlement on-chain

- Le message de règlement (`claim.rs`) contient déjà le hash du résultat. Les approbateurs le signent comme aujourd'hui. **Pas de changement de format de règlement** si le message reste identique.
- Le programme devnet doit connaître le nouveau workload (identifiant et récompenses), comme pour les autres upgrades. Ce n'est pas un changement de protocole de règlement.
- À confirmer : le programme ne vérifie aucun champ d'attestation autre que la signature de règlement. Si c'est le cas, le mode `tolerance` ne le touche pas.

### 10.7 Tests à écrire avant le code

1. Attestation `tolerance` rejouée comme `exact` : refusée.
2. Attestation `exact` rejouée comme `tolerance` : refusée.
3. Worker qui change le mode dans le certificat : refusé (mode lu du descripteur).
4. Cosinus sous le seuil avec `approved = true` : refusé.
5. Graine calculée avant le VerifyOffer : refusée.
6. Honnête sur deux CPU (AVX2) : approuvé, cosinus 1.0 sur les mêmes vecteurs.

### 10.8 Décisions

1. **`cos_min` = 0,999** (`cos_min_micro = 999000`). Honnête sur AVX2 : cosinus 1,0. Triches mesurées : int8 générique 0,976, int8 par canal 0,988, toutes sous le seuil. À revalider après la mesure AVX-512.
2. **Taille d'échantillon : pilotée par le descripteur**, via `samples_needed`. Proposition : `min_cheat_fraction = 0,1` et `target_detection = 0,97`, soit environ 34 éléments par vérificateur (à confirmer avec la fonction). Pour `f = 0,2` et `0,99`, une vingtaine suffit. Coût à mesurer : passages par vérificateur et par fragment.
3. **Au moins 2 vérificateurs par fragment en mode `tolerance`** (`verifications_required >= 2`). Un vérificateur seul laisse passer des fraudes légères. **Non appliqué** : `policy.go` fixe `VerificationsRequired = 1` pour tous les workloads. Décision à prendre avant de l'ouvrir à des utilisateurs réels.
4. **Programme devnet : confirmé.** `chain/program/src/settle.rs` ne lit que les signatures Ed25519 précompilées sur le message de règlement, qui contient déjà le hash du résultat. Ni le corps d'attestation, ni le mode, ni la preuve de tolérance ne sont lus. Le mode `tolerance` ne touche pas au programme. L'upgrade devnet ne sert qu'à ajouter le workload (identifiant, récompenses).
