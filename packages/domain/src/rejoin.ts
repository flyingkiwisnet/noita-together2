// Durable eligibility is independent of the short transport/native lease.
export const REJOIN_LEASE_MS=30000;
export const REJOIN_RETENTION='Until explicit End/cancel, participant removal/offline, or retained run deletion; no automatic all-offline or human rejoin timeout.';
export function recoveryDisposition(terminal:boolean,healthyOwner:boolean,protectedReport:boolean,contentMatches:boolean){
 if(terminal)return 'ineligible';
 if(healthyOwner)return 'owner-active';
 if(!protectedReport)return 'inspect';
 return contentMatches?'release':'review';
}
