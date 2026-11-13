#!/usr/bin/env node
// Génère les descripteurs de workload pour Rust, Go et TypeScript à partir de
// contracts/workloads/*.json. Un workload se décrit une fois, dans ce dossier : les trois
// consommateurs (nœud, API Go, site) lisent les fichiers générés, jamais une copie à la main.
//
// Usage : node tools/workloads/generate.mjs          (écrit les fichiers générés)
//         node tools/workloads/generate.mjs --check  (échoue si un fichier généré est périmé)

import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { emitGo, emitRust, emitTs } from './emit.mjs'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')
const source = join(root, 'contracts', 'workloads')
const outputs = {
  rust: join(root, 'crates', 'sil-workload-spec', 'src', 'generated.rs'),
  go: join(root, 'services', 'api', 'internal', 'workloadspec', 'generated.go'),
  ts: join(root, 'apps', 'web', 'src', 'workloads', 'generated.ts'),
}

const SCHEMA = 'silicium.workload/v1'
const UNITS = ['bytes_input', 'pixels_band']
const POLICIES = ['recompute', 'sampling', 'sampling_tolerance']
const DEPOSITS = ['supported', 'unsupported']
// Paramètres que la spec du job sait porter (ledger `JobSpec.width` / `height`).
const PARAMS = ['width', 'height']

function fail(file, message) {
  throw new Error(`${file} : ${message}`)
}

function integer(value, file, name, min = 0) {
  if (!Number.isSafeInteger(value) || value < min) {
    fail(file, `${name} : entier ≥ ${min} attendu`)
  }
  return value
}

function fraction(value, file, name) {
  if (typeof value !== 'number' || !(value > 0 && value < 1)) {
    fail(file, `${name} : nombre strictement entre 0 et 1 attendu`)
  }
  return value
}

function sha256Hex(value, file) {
  if (typeof value !== 'string' || !/^[0-9a-f]{64}$/.test(value)) {
    fail(file, 'model_sha256 : 64 caractères hexadécimaux minuscules attendus')
  }
  return value
}

function nonEmpty(value, file, name) {
  if (typeof value !== 'string' || value.trim() === '') fail(file, `${name} : texte non vide attendu`)
  return value
}

// Vecteur de ressources sollicitées (ADR 0013 §3) : intensité relative, 0 à 100, jamais le nom du
// workload. Sert à apprendre la relation ressources → résultat sans nommer de type de tâche.
const RESOURCE_DIMENSIONS = ['compute', 'memory', 'network', 'storage']

function resources(raw, file) {
  const value = raw.resources
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    fail(file, 'resources : objet attendu')
  }
  const keys = Object.keys(value)
  if (keys.length !== RESOURCE_DIMENSIONS.length || !RESOURCE_DIMENSIONS.every((k) => keys.includes(k))) {
    fail(file, `resources : exactement ${RESOURCE_DIMENSIONS.join(', ')} attendus`)
  }
  const out = {}
  for (const dimension of RESOURCE_DIMENSIONS) {
    out[dimension] = integer(value[dimension], file, `resources.${dimension}`, 0)
    if (out[dimension] > 100) fail(file, `resources.${dimension} : au plus 100 attendu`)
  }
  if (RESOURCE_DIMENSIONS.every((dimension) => out[dimension] === 0)) {
    fail(file, 'resources : au moins une dimension doit être non nulle')
  }
  return out
}

/** Vérifie un descripteur et le ramène à une forme plate, commune aux trois langages. */
export function normalize(raw, file) {
  if (raw.schema !== SCHEMA) fail(file, `schéma inconnu : ${raw.schema}`)
  if (!/^[a-z0-9][a-z0-9/._-]*$/.test(raw.id ?? '')) fail(file, 'identifiant invalide')
  if (typeof raw.description !== 'string' || raw.description.trim() === '') {
    fail(file, 'description manquante')
  }
  if (!UNITS.includes(raw.unit)) fail(file, `unité inconnue : ${raw.unit}`)
  if (typeof raw.label_key !== 'string' || raw.label_key === '') fail(file, 'label_key manquant')
  if (!DEPOSITS.includes(raw.deposit)) fail(file, `deposit inconnu : ${raw.deposit}`)

  const params = raw.input?.params ?? []
  if (!Array.isArray(params) || params.some((name) => !PARAMS.includes(name))) {
    fail(file, `paramètres inconnus (seulement ${PARAMS.join(', ')})`)
  }
  const isImage = raw.unit === 'pixels_band'
  if (isImage && JSON.stringify(params) !== JSON.stringify(PARAMS)) {
    fail(file, 'une bande de pixels exige les paramètres width puis height')
  }
  if (!isImage && params.length !== 0) fail(file, 'paramètres interdits pour cette unité')

  const blockRows = isImage ? integer(raw.geometry?.block_rows, file, 'geometry.block_rows', 1) : 0
  const maxSide = isImage ? integer(raw.input?.max_side, file, 'input.max_side', 1) : 0
  if (!isImage && raw.geometry !== undefined) fail(file, 'geometry interdit pour cette unité')

  const policy = raw.verification?.policy
  if (!POLICIES.includes(policy)) fail(file, `politique de vérification inconnue : ${policy}`)
  const tolerance = policy === 'sampling_tolerance'
  const sampling = policy === 'sampling' || tolerance
  if (!sampling && Object.keys(raw.verification ?? {}).length !== 1) {
    fail(file, 'politique recompute sans paramètres')
  }

  return {
    id: raw.id,
    unit: raw.unit,
    description: raw.description,
    labelKey: raw.label_key,
    params: [...params],
    maxInputBytes: integer(raw.input?.max_input_bytes, file, 'input.max_input_bytes', 1),
    maxSide,
    blockRows,
    maxFragments: integer(raw.split?.max_fragments, file, 'split.max_fragments', 1),
    targetSize: integer(raw.split?.target_size, file, 'split.target_size', 1),
    rewardPerUnit: integer(raw.terms?.reward_per_unit, file, 'terms.reward_per_unit'),
    verifierRewardPerUnit: integer(
      raw.terms?.verifier_reward_per_unit,
      file,
      'terms.verifier_reward_per_unit',
    ),
    policy,
    minCheatFraction: sampling ? fraction(raw.verification.min_cheat_fraction, file, 'min_cheat_fraction') : 0,
    targetDetection: sampling ? fraction(raw.verification.target_detection, file, 'target_detection') : 0,
    cosMin: tolerance ? fraction(raw.verification.cos_min, file, 'cos_min') : 0,
    modelSha256: tolerance ? sha256Hex(raw.verification.model_sha256, file) : '',
    ortVersion: tolerance ? nonEmpty(raw.verification.ort_version, file, 'ort_version') : '',
    wallMs: integer(raw.limits?.wall_ms, file, 'limits.wall_ms', 1),
    memoryBytes: integer(raw.limits?.memory_bytes, file, 'limits.memory_bytes', 1),
    outputBytes: integer(raw.limits?.output_bytes, file, 'limits.output_bytes', 1),
    depositSupported: raw.deposit === 'supported',
    resources: resources(raw, file),
  }
}

/** Lit tous les descripteurs, les vérifie, refuse les identifiants en double, trie par id. */
export function loadSpecs(dir = source) {
  const files = readdirSync(dir).filter((name) => name.endsWith('.json')).sort()
  if (files.length === 0) throw new Error(`aucun descripteur dans ${dir}`)
  const specs = files.map((name) => {
    const raw = JSON.parse(readFileSync(join(dir, name), 'utf8'))
    return normalize(raw, name)
  })
  const ids = new Set()
  for (const spec of specs) {
    if (ids.has(spec.id)) throw new Error(`identifiant en double : ${spec.id}`)
    ids.add(spec.id)
  }
  return specs.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
}

function main() {
  const check = process.argv.includes('--check')
  const specs = loadSpecs()
  const generated = {
    rust: emitRust(specs),
    go: emitGo(specs),
    ts: emitTs(specs),
  }
  let stale = false
  for (const [language, path] of Object.entries(outputs)) {
    const text = generated[language]
    if (check) {
      let current = ''
      try {
        current = readFileSync(path, 'utf8')
      } catch {
        // Absent : périmé.
      }
      if (current !== text) {
        stale = true
        console.error(`périmé : ${path}`)
      }
    } else {
      writeFileSync(path, text)
      console.log(`écrit : ${path}`)
    }
  }
  if (check && stale) {
    console.error('lancer : node tools/workloads/generate.mjs')
    process.exit(1)
  }
  if (check) console.log(`${specs.length} descripteur(s), fichiers générés à jour`)
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  main()
}
