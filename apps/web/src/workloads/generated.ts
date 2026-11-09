// GÉNÉRÉ par tools/workloads/generate.mjs depuis contracts/workloads/*.json. Ne pas modifier à la main.

/** Unité de travail : ce que le fragment rapporte (sil-chain-core::units). */
export type WorkloadUnit = 'bytes_input' | 'pixels_band'

/** Vérification d'un résultat (sil-workload-api::VerificationPolicy). */
export type WorkloadVerification =
  | { policy: 'recompute' }
  | { policy: 'sampling'; minCheatFraction: number; targetDetection: number }

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
