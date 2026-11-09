import type { Device } from '@/api/endpoints'
import { isDesktop, machineKey } from '@/platform/desktop'

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

const KEY = 'silicium.desktop.link'

export function readLinkRecord(): LinkRecord | null {
  try {
    const raw = globalThis.localStorage.getItem(KEY)
    if (raw === null) return null
    const value = JSON.parse(raw) as Partial<LinkRecord>
    if (typeof value.linked_at !== 'string' || typeof value.public_key !== 'string') return null
    return value as LinkRecord
  } catch {
    return null
  }
}

export function saveLinkRecord(device: Device, publicKey: string): void {
  const record: LinkRecord = {
    device_id: device.id,
    label: device.label,
    linked_at: device.linked_at,
    public_key: publicKey,
  }
  try {
    globalThis.localStorage.setItem(KEY, JSON.stringify(record))
  } catch {
    // Stockage indisponible : la liaison reste faite côté API, seul l'affichage est absent.
  }
}

/**
 * La liaison mémorisée vaut pour la clé de machine courante. Une réinstallation crée une nouvelle
 * clé : l'ancienne liaison ne doit plus s'afficher comme celle de cette machine.
 */
export async function currentLinkRecord(): Promise<LinkRecord | null> {
  if (!isDesktop()) return null
  const current = await machineKey()
  const record = readLinkRecord()
  return record !== null && current !== null && record.public_key === current ? record : null
}
