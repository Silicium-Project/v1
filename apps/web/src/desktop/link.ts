import { linkDevice, type Device } from '@/api/endpoints'
import { createMachineKey, machineKey, signLinkCode } from '@/platform/desktop'
import { saveLinkRecord } from '@/desktop/link-record'

/**
 * Links this machine to the account that issued `code`. The machine key is created when it is
 * missing. Only the public key and the signature of the code leave the machine: the API checks
 * the proof and consumes the code. The link is then remembered here, to show its date.
 */
export async function linkThisMachine(code: string, label: string): Promise<Device> {
  const normal = code.trim().toUpperCase()
  if ((await machineKey()) === null) await createMachineKey()
  const proof = await signLinkCode(normal)
  const device = await linkDevice(normal, proof.public_key, proof.signature, label.trim())
  saveLinkRecord(device, proof.public_key)
  return device
}
