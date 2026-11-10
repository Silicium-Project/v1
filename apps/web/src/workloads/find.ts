import { WORKLOADS, type WorkloadSpec } from './generated'

/** The descriptor of a workload identifier, if the network runs it. */
export function findWorkload(id: string): WorkloadSpec | undefined {
  return WORKLOADS.find((spec) => spec.id === id)
}

/** True for a whole number from 1 to the workload's largest side (pixels of an image parameter). */
export function isImageSide(value: number, spec: WorkloadSpec): boolean {
  return Number.isInteger(value) && value >= 1 && value <= spec.maxSide
}

