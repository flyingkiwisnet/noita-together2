// Discussion types only; not accepted schemas, authentication implementations or promises.
export interface ServiceIdentityProposal {
  serviceId: string; httpsOrigin: string; protocolMajor: number;
  contractRevision: string; supportedClientVersions: string[]; capabilities: string[];
}
export interface PairingAttemptProposal {
  attemptId: string; serviceId: string; deviceId: string;
  activationCode: string; pollingSecret: string; expiresAt: string;
  state: 'pending' | 'confirmed' | 'exchanged' | 'cancelled' | 'expired';
}
export interface PreparationProposal {
  preparationId: string; runId: string; runEpoch: string; configRevision: number;
  seed: number; rules: Record<string, boolean | number | string>;
  participants: {accountId:string; gameConnectionId:string; connectionEpoch:string; compatibilityVersion:string}[];
  state: 'preparing' | 'loading' | 'running' | 'cancelled' | 'ended';
}
export interface TransferProposal {
  operationId:string; accountId:string; roomId:string; runId:string; runEpoch:string;
  direction:'deposit' | 'withdrawal'; requestDigest:string; deliveryId:string | null;
  asset:{kind:'gold'; amount:number} | {kind:'catalogue-item'; catalogueVersion:string; itemId:string; representation:Record<string,unknown>};
  state:'requested' | 'reserved' | 'committed' | 'locally-applied' | 'completed' | 'cancelled' | 'uncertain';
  receipt:{gameInstanceId:string; receiptId:string; persistenceEvidenceVersion:string} | null;
}
