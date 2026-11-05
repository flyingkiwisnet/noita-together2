import { BoundaryError, boundedId, exactKeys } from './index.js';
export const pairingRevision = 'g0-pairing-1' as const;
export type PairingState = 'pending' | 'approved' | 'exchanged' | 'cancelled' | 'expired';
export const pairingLimits = {attempts:64, lifetimeMs:60000, preparationMs:120000} as const;
export const proofKeys = ['contractRevision','serviceId','attemptId','deviceId','pollingSecret'] as const;
export function parsePairing(value: unknown, fields: readonly string[]): Record<string, unknown> {
  exactKeys(value,['contractRevision','serviceId',...fields]);
  if (value.contractRevision !== pairingRevision) throw new BoundaryError('INCOMPATIBLE_VERSION');
  if (!boundedId(value.serviceId)) throw new BoundaryError('INVALID_MESSAGE');
  for (const key of fields) {
    if (key === 'body') continue;
    if (key === 'expectedRevision' || key === 'configRevision') {
      if (!Number.isSafeInteger(value[key]) || (value[key] as number) < 1) throw new BoundaryError('INVALID_MESSAGE');
    } else if (key === 'runId' || key === 'runEpoch') {
      if (!(value[key] === null || boundedId(value[key]))) throw new BoundaryError('INVALID_MESSAGE');
    } else if (key === 'kind') {
      if (typeof value[key] !== 'string' || value[key].length>80 || !/^[a-z]+\.[a-zA-Z]+$/.test(value[key])) throw new BoundaryError('INVALID_MESSAGE');
    } else if (key === 'present') {
      if (typeof value[key] !== 'boolean') throw new BoundaryError('INVALID_MESSAGE');
    } else if (!boundedId(value[key])) throw new BoundaryError('INVALID_MESSAGE');
  }
  return value;
}
export interface GameBinding {
  serviceEpoch:string; roomId:string; runId:string; runEpoch:string; configRevision:number;
  desktopConnectionId:string; attachmentId:string; gameInstanceId:string; expiresAt:number;
}
