// GÉNÉRÉ par tools/workloads/generate.mjs depuis contracts/workloads/*.json. Ne pas modifier à la main.

/** Unité de travail : ce que le fragment rapporte (sil-chain-core::units). */
export type WorkloadUnit = 'bytes_input' | 'pixels_band'

/** Vérification d'un résultat (sil-workload-api::VerificationPolicy). */
export type WorkloadVerification =
  | { policy: 'recompute' }
  | { policy: 'sampling'; minCheatFraction: number; targetDetection: number }
  | { policy: 'sampling_tolerance'; minCheatFraction: number; targetDetection: number; cosMin: number; modelSha256: string; ortVersion: string }

export interface WorkloadSpec {
  id: string
  unit: WorkloadUnit
  description: string
  labelKey: string
  params: readonly ('width' | 'height')[]
  maxInputBytes: number
  maxSide: number
  blockRows: number
  maxFragments: number
  targetSize: number
  rewardPerUnit: number
  verifierRewardPerUnit: number
  verification: WorkloadVerification
  limits: { wallMs: number; memoryBytes: number; outputBytes: number }
  depositSupported: boolean
}

/** Tous les workloads du réseau, triés par identifiant. */
export const WORKLOADS: readonly WorkloadSpec[] = [
  {
    id: "calcul/sha256",
    unit: "bytes_input",
    description: "Une unité = un octet de l'entrée. Chaque fragment est une plage d'octets dont le SHA-256 est publié ; un vérificateur refait le hash de la plage.",
    labelKey: "submit.workloadSha256",
    params: [],
    maxInputBytes: 8388608,
    maxSide: 0,
    blockRows: 0,
    maxFragments: 16,
    targetSize: 262144,
    rewardPerUnit: 100,
    verifierRewardPerUnit: 10,
    verification: { policy: 'recompute' },
    limits: { wallMs: 120000, memoryBytes: 67108864, outputBytes: 32 },
    depositSupported: true,
  },
  {
    id: "embedding/e5-small",
    unit: "bytes_input",
    description: "Une unité = un octet de l'entrée. L'entrée est un lot de textes (un texte par ligne, découpé en chunks de 512 tokens au plus). Chaque chunk reçoit son vecteur à 384 dimensions (multilingual-e5-small, fp32, préfixe « passage: »). Vérification par tolérance : un vérificateur recalcule un échantillon de chunks et compare chaque vecteur par cosinus.",
    labelKey: "submit.workloadEmbedding",
    params: [],
    maxInputBytes: 8388608,
    maxSide: 0,
    blockRows: 0,
    maxFragments: 16,
    targetSize: 262144,
    rewardPerUnit: 100,
    verifierRewardPerUnit: 10,
    verification: { policy: 'sampling_tolerance', minCheatFraction: 0.1, targetDetection: 0.97, cosMin: 0.999, modelSha256: "ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665", ortVersion: "1.31.1" },
    limits: { wallMs: 300000, memoryBytes: 1073741824, outputBytes: 67108864 },
    depositSupported: true,
  },
  {
    id: "image/batch",
    unit: "bytes_input",
    description: "Une unité = un octet de l'entrée. L'entrée est un lot d'images (conteneur SILBATCH) : chaque image est redimensionnée et réencodée (PNG ou JPEG). Chaque fragment est une plage d'images, dont le résultat est publié ; un vérificateur refait le traitement de la plage.",
    labelKey: "submit.workloadImage",
    params: [],
    maxInputBytes: 8388608,
    maxSide: 0,
    blockRows: 0,
    maxFragments: 16,
    targetSize: 262144,
    rewardPerUnit: 100,
    verifierRewardPerUnit: 10,
    verification: { policy: 'sampling', minCheatFraction: 0.2, targetDetection: 0.99 },
    limits: { wallMs: 120000, memoryBytes: 536870912, outputBytes: 67108864 },
    depositSupported: true,
  },
  {
    id: "rendu/raytracer",
    unit: "pixels_band",
    description: "Une unité = un pixel de la bande du fragment. L'entrée est une scène .cfg, rendue par le moteur raytracing déterministe (sil-raytracer) ; un vérificateur recalcule un échantillon de blocs de 8 lignes.",
    labelKey: "submit.workloadRender",
    params: ["width", "height"],
    maxInputBytes: 8388608,
    maxSide: 4096,
    blockRows: 8,
    maxFragments: 16,
    targetSize: 32,
    rewardPerUnit: 1,
    verifierRewardPerUnit: 1,
    verification: { policy: 'sampling', minCheatFraction: 0.2, targetDetection: 0.99 },
    limits: { wallMs: 600000, memoryBytes: 1073741824, outputBytes: 268435456 },
    depositSupported: true,
  },
]
