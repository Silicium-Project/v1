import type { Device } from '@/api/endpoints'
import { isDesktop, machineKey, saveLink, savedLink } from '@/platform/desktop'

/**
 * Liaison faite depuis cette application : quand, à quel libellé, avec quelle clé de machine. L'API
 * tient la référence du compte ; cet enregistrement sert seulement à afficher l'état sur la machine.
 */
export interface LinkRecord {
  device_id: string
  label: string
  linked_at: string
  public_key: string
}

/**
 * Ancien emplacement : le stockage du navigateur intégré. Il pouvait être perdu, la liaison
 * n'apparaissait plus sur la machine alors qu'elle existait côté compte. Repris une seule fois.
 */
const LEGACY_KEY = 'silicium.desktop.link'

function readLegacyRecord(): LinkRecord | null {
  try {
    const raw = globalThis.localStorage.getItem(LEGACY_KEY)
    if (raw === null) return null
    const value = JSON.parse(raw) as Partial<LinkRecord>
    if (typeof value.linked_at !== 'string' || typeof value.public_key !== 'string') return null
    return value as LinkRecord
  } catch {
    return null
  }
}

/**
 * La liaison de cette machine. Elle est gardée par l'application de bureau, sur le disque, à côté
 * de la clé : elle vaut seulement pour la clé de machine courante. Une réinstallation crée une
 * nouvelle clé : l'ancienne liaison ne doit plus s'afficher comme celle de cette machine.
 */
export async function currentLinkRecord(): Promise<LinkRecord | null> {
  if (!isDesktop()) return null
  try {
    const saved = await savedLink()
    if (saved !== null) return saved
  } catch {
    return null
  }
  const legacy = readLegacyRecord()
  const current = await machineKey()
  if (legacy === null || current === null || legacy.public_key !== current) return null
  try {
    await saveLink(legacy)
  } catch {
    // Pas de reprise possible : la liaison reste affichée pour cette visite.
  }
  return legacy
}

/** Garde la liaison faite à l'instant. Un échec ne défait pas la liaison côté API. */
export async function saveLinkRecord(device: Device, publicKey: string): Promise<void> {
  try {
    await saveLink({
      device_id: device.id,
      label: device.label,
      linked_at: device.linked_at,
      public_key: publicKey,
    })
  } catch {
    // La liaison est faite côté API : seul l'affichage sur cette machine est absent.
  }
}
