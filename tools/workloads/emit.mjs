// Écrit les fichiers générés à partir des descripteurs normalisés (voir generate.mjs).
// Le format est fixé ici : `--check` compare octet pour octet, donc aucun formateur n'intervient.

const GENERATED = 'GÉNÉRÉ par tools/workloads/generate.mjs depuis contracts/workloads/*.json. Ne pas modifier à la main.'

const quote = (text) => JSON.stringify(text)
const float = (value) => (Number.isInteger(value) ? value.toFixed(1) : String(value))
const constName = (id) => id.toUpperCase().replace(/[^A-Z0-9]+/g, '_')

function rustSpecs(specs) {
  const out = []
  for (const spec of specs) {
    const unit = spec.unit === 'bytes_input' ? 'Unit::BytesInput' : 'Unit::PixelsBand'
    const verification =
      spec.policy === 'recompute'
        ? ['    verification: Verification::Recompute,']
        : spec.policy === 'sampling_tolerance'
          ? [
              '    verification: Verification::SamplingTolerance {',
              `        min_cheat_fraction: ${float(spec.minCheatFraction)},`,
              `        target_detection: ${float(spec.targetDetection)},`,
              `        cos_min: ${float(spec.cosMin)},`,
              `        model_sha256: ${quote(spec.modelSha256)},`,
              `        ort_version: ${quote(spec.ortVersion)},`,
              '    },',
            ]
          : [
              '    verification: Verification::Sampling {',
              `        min_cheat_fraction: ${float(spec.minCheatFraction)},`,
              `        target_detection: ${float(spec.targetDetection)},`,
              '    },',
            ]
    const params = `&[${spec.params.map(quote).join(', ')}]`
    out.push(`/// \`${spec.id}\` : ${spec.description}`)
    out.push(`pub const ${constName(spec.id)}: Spec = Spec {`)
    out.push(`    id: ${quote(spec.id)},`)
    out.push(`    unit: ${unit},`)
    out.push(`    label_key: ${quote(spec.labelKey)},`)
    out.push(`    params: ${params},`)
    out.push(`    max_input_bytes: ${spec.maxInputBytes},`)
    out.push(`    max_side: ${spec.maxSide},`)
    out.push(`    block_rows: ${spec.blockRows},`)
    out.push(`    max_fragments: ${spec.maxFragments},`)
    out.push(`    target_size: ${spec.targetSize},`)
    out.push(`    split_strategy: ${quote(spec.splitStrategy)},`)
    out.push(`    reward_per_unit: ${spec.rewardPerUnit},`)
    out.push(`    verifier_reward_per_unit: ${spec.verifierRewardPerUnit},`)
    out.push(...verification)
    out.push('    limits: Limits {')
    out.push(`        wall_ms: ${spec.wallMs},`)
    out.push(`        memory_bytes: ${spec.memoryBytes},`)
    out.push(`        output_bytes: ${spec.outputBytes},`)
    out.push('    },')
    out.push(`    deposit_supported: ${spec.depositSupported},`)
    out.push('    resources: Resources {')
    out.push(`        compute: ${spec.resources.compute},`)
    out.push(`        memory: ${spec.resources.memory},`)
    out.push(`        network: ${spec.resources.network},`)
    out.push(`        storage: ${spec.resources.storage},`)
    out.push('    },')
    out.push('};', '')
  }
  return out
}

function rustAll(specs) {
  const out = []
  const names = specs.map((spec) => constName(spec.id))
  out.push('/// Tous les descripteurs, triés par identifiant.')
  // Format rustfmt (voir ci-dessous).
  const oneLine = `pub const ALL: &[Spec] = &[${names.join(', ')}];`
  // rustfmt : tableau sur une ligne si son contenu tient en 60 colonnes (array_width par défaut).
  if (names.join(', ').length <= 60) out.push(oneLine, '')
  else out.push('pub const ALL: &[Spec] = &[', ...names.map((name) => `    ${name},`), '];', '')
  return out
}

export function emitRust(specs) {
  const coreSpecs = specs.filter((spec) => !spec.id.startsWith('ia/'))
  const out = [
    `// ${GENERATED}`,
    '',
    '#[path = "generated_ai.rs"]',
    'mod generated_ai;',
    'pub use generated_ai::*;',
    '',
    'use crate::{Limits, Resources, Spec, Unit, Verification};',
    '',
    ...rustSpecs(coreSpecs),
    ...rustAll(specs),
  ]
  return out.join('\n')
}

export function emitRustAI(specs) {
  const aiSpecs = specs.filter((spec) => spec.id.startsWith('ia/'))
  const out = [
    `// ${GENERATED}`,
    '',
    'use crate::{Limits, Resources, Spec, Unit, Verification};',
    '',
    ...rustSpecs(aiSpecs),
  ]
  return out.join('\n')
}

export function emitGo(specs) {
  const rows = [
    ['ID', (s) => quote(s.id)],
    ['Unit', (s) => (s.unit === 'bytes_input' ? 'UnitBytesInput' : 'UnitPixelsBand')],
    ['Description', (s) => quote(s.description)],
    ['LabelKey', (s) => quote(s.labelKey)],
    ['Params', (s) => `[]string{${s.params.map(quote).join(', ')}}`],
    ['MaxInputBytes', (s) => String(s.maxInputBytes)],
    ['MaxSide', (s) => String(s.maxSide)],
    ['BlockRows', (s) => String(s.blockRows)],
    ['MaxFragments', (s) => String(s.maxFragments)],
    ['TargetSize', (s) => String(s.targetSize)],
    ['SplitStrategy', (s) => quote(s.splitStrategy)],
    ['RewardPerUnit', (s) => String(s.rewardPerUnit)],
    ['VerifierRewardPerUnit', (s) => String(s.verifierRewardPerUnit)],
    ['Policy', (s) => quote(s.policy)],
    ['MinCheatFraction', (s) => float(s.minCheatFraction)],
    ['TargetDetection', (s) => float(s.targetDetection)],
    ['CosMin', (s) => float(s.cosMin)],
    ['ModelSha256', (s) => quote(s.modelSha256)],
    ['OrtVersion', (s) => quote(s.ortVersion)],
    ['WallMs', (s) => String(s.wallMs)],
    ['MemoryBytes', (s) => String(s.memoryBytes)],
    ['OutputBytes', (s) => String(s.outputBytes)],
    ['DepositSupported', (s) => String(s.depositSupported)],
    ['Closed', (s) => String(s.closed)],
    ['InputForm', (s) => quote(s.inputForm)],
    ['InputOptions', (s) => String(s.inputOptions)],
    ['Result', (s) => quote(s.result)],
    ['Resources', (s) =>
      `Resources{Compute: ${s.resources.compute}, Memory: ${s.resources.memory}, Network: ${s.resources.network}, Storage: ${s.resources.storage}}`],
  ]
  // gofmt aligne les valeurs d'un même littéral : la colonne suit la clé la plus longue.
  const column = Math.max(...rows.map(([key]) => key.length + 1)) + 1
  const out = []
  out.push(`// Code generated by tools/workloads/generate.mjs from contracts/workloads/*.json. DO NOT EDIT.`, '')
  out.push('package workloadspec', '')
  out.push('// Workloads lists every descriptor of the network, sorted by ID.')
  out.push('var Workloads = []Spec{')
  for (const spec of specs) {
    out.push('\t{')
    for (const [key, value] of rows) {
      out.push(`\t\t${`${key}:`.padEnd(column)}${value(spec)},`)
    }
    out.push('\t},')
  }
  out.push('}', '')
  return out.join('\n')
}

export function emitTs(specs) {
  const out = []
  out.push(`// ${GENERATED}`, '')
  out.push('/** Unité de travail : ce que le fragment rapporte (sil-chain-core::units). */')
  out.push("export type WorkloadUnit = 'bytes_input' | 'pixels_band'", '')
  out.push('/** Vérification d\'un résultat (sil-workload-api::VerificationPolicy). */')
  out.push('export type WorkloadVerification =')
  out.push("  | { policy: 'recompute' }")
  out.push("  | { policy: 'sampling'; minCheatFraction: number; targetDetection: number }")
  out.push(
    "  | { policy: 'sampling_tolerance'; minCheatFraction: number; targetDetection: number; cosMin: number; modelSha256: string; ortVersion: string }",
    '',
  )
  out.push('/** Formulaire de soumission du workload (bloc `ui.input` du descripteur). */')
  out.push("export type WorkloadInput = 'file' | 'scene' | 'images' | 'documents' | 'texts' | 'chat'", '')
  out.push('/** Forme du résultat : transport par l\'API et affichage par le site (bloc `ui.result`). */')
  out.push("export type WorkloadResult = 'text' | 'image' | 'files' | 'documents' | 'embedding' | 'classification' | 'chat'", '')
  out.push('export interface WorkloadSpec {')
  out.push('  id: string')
  out.push('  unit: WorkloadUnit')
  out.push('  description: string')
  out.push('  labelKey: string')
  out.push("  params: readonly ('width' | 'height')[]")
  out.push('  maxInputBytes: number')
  out.push('  maxSide: number')
  out.push('  blockRows: number')
  out.push('  maxFragments: number')
  out.push('  targetSize: number')
  out.push("  splitStrategy: 'input_size' | 'workers'")
  out.push('  rewardPerUnit: number')
  out.push('  verifierRewardPerUnit: number')
  out.push('  verification: WorkloadVerification')
  out.push('  limits: { wallMs: number; memoryBytes: number; outputBytes: number }')
  out.push('  depositSupported: boolean')
  out.push("  /** Descripteur gardé pour les jobs déjà publiés : l'API et le site n'ouvrent plus de nouveau job. */")
  out.push('  closed: boolean')
  out.push('  inputForm: WorkloadInput')
  out.push("  /** Le formulaire d'images montre ses options (format, taille, rotation…). Sans objet hors des images. */")
  out.push('  inputOptions: boolean')
  out.push('  result: WorkloadResult')
  out.push('  /** Intensité relative sollicitée (0 à 100), jamais le nom du workload (ADR 0013 §3). */')
  out.push('  resources: { compute: number; memory: number; network: number; storage: number }')
  out.push('}', '')
  out.push('/** Tous les workloads du réseau, triés par identifiant. */')
  out.push('export const WORKLOADS: readonly WorkloadSpec[] = [')
  for (const spec of specs) {
    const verification =
      spec.policy === 'recompute'
        ? "{ policy: 'recompute' }"
        : spec.policy === 'sampling_tolerance'
          ? `{ policy: 'sampling_tolerance', minCheatFraction: ${float(spec.minCheatFraction)}, targetDetection: ${float(spec.targetDetection)}, cosMin: ${float(spec.cosMin)}, modelSha256: ${quote(spec.modelSha256)}, ortVersion: ${quote(spec.ortVersion)} }`
          : `{ policy: 'sampling', minCheatFraction: ${float(spec.minCheatFraction)}, targetDetection: ${float(spec.targetDetection)} }`
    out.push('  {')
    out.push(`    id: ${quote(spec.id)},`)
    out.push(`    unit: ${quote(spec.unit)},`)
    out.push(`    description: ${quote(spec.description)},`)
    out.push(`    labelKey: ${quote(spec.labelKey)},`)
    out.push(`    params: [${spec.params.map(quote).join(', ')}],`)
    out.push(`    maxInputBytes: ${spec.maxInputBytes},`)
    out.push(`    maxSide: ${spec.maxSide},`)
    out.push(`    blockRows: ${spec.blockRows},`)
    out.push(`    maxFragments: ${spec.maxFragments},`)
    out.push(`    targetSize: ${spec.targetSize},`)
    out.push(`    splitStrategy: ${quote(spec.splitStrategy)},`)
    out.push(`    rewardPerUnit: ${spec.rewardPerUnit},`)
    out.push(`    verifierRewardPerUnit: ${spec.verifierRewardPerUnit},`)
    out.push(`    verification: ${verification},`)
    out.push(
      `    limits: { wallMs: ${spec.wallMs}, memoryBytes: ${spec.memoryBytes}, outputBytes: ${spec.outputBytes} },`,
    )
    out.push(`    depositSupported: ${spec.depositSupported},`)
    out.push(`    closed: ${spec.closed},`)
    out.push(`    inputForm: ${quote(spec.inputForm)},`)
    out.push(`    inputOptions: ${spec.inputOptions},`)
    out.push(`    result: ${quote(spec.result)},`)
    out.push(
      `    resources: { compute: ${spec.resources.compute}, memory: ${spec.resources.memory}, network: ${spec.resources.network}, storage: ${spec.resources.storage} },`,
    )
    out.push('  },')
  }
  out.push(']', '')
  return out.join('\n')
}
