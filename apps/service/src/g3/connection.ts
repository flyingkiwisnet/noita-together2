import type {Actor} from '../g1/auth.js';
import type {Tx,Database} from '../g1/database.js';
import type {PrincipalAuth} from '../g2-next/auth.js';
import {fingerprint,demand} from '../g1/security.js';
import {readRoomAdmissions,type AdmittedWorld,type AdmissionCompatibility} from './admission.js';
import {validate} from '../../../../packages/contracts/src/g3-connection.js';

export const remaining=(until:number|undefined,now:number,max:number)=>Number.isFinite(until)?Math.max(0,Math.min(max,Math.floor(until!-now))):0;
export function serializeAdmission(w:AdmittedWorld,actor:Actor,requestId:string){
 const sample={originRequestId:requestId,channel:'http',credentialId:actor.credentialId??'',sampleKey:fingerprint([w.serviceId,w.serviceEpoch,w.scope, w.generation,w.immutable.authorizationId,requestId,actor.credentialId,'http'])};
 const {serviceId,serviceEpoch,participantId,generation,scope,owner,originalStart,immutable,currentReport,decision}=w;
 // JSON normalization removes optional undefined values before strict validation.
 const result=JSON.parse(JSON.stringify({serviceId,serviceEpoch,participantId,generation,scope,owner,originalStart,immutable,currentReport,decision,sample}));
 validate('ConnectionAdmission',result);return result;
}
export async function credentialValidity(tx:Tx,auth:PrincipalAuth,token:string,requestId:string){
 const actor=await auth.authenticate(tx,token),guest=(await tx.query('SELECT guest_expires_at FROM principal_profiles WHERE principal_id=$1',[actor.accountId])).rows[0];
 actor.accessUntil=Math.min(actor.accessUntil??0,guest?.guest_expires_at?+guest.guest_expires_at:Infinity);
 demand((actor.accessUntil??0)>auth.now(),'UNAUTHENTICATED');
 return {actor,value:{principalId:actor.accountId,sessionId:actor.sessionId,sessionEpoch:actor.sessionEpoch,credentialId:actor.credentialId!,accessRemainingMs:remaining(actor.accessUntil,auth.now(),900000),originRequestId:requestId,sampleKey:fingerprint([auth.db.serviceId,auth.db.epoch,actor.accountId,actor.sessionId,actor.sessionEpoch,actor.credentialId,requestId,'credential'])}};
}
export async function connectionReply(tx:Tx,db:Database,auth:PrincipalAuth,compat:AdmissionCompatibility,input:any,value:any,token?:string){
 if(value.credentials?.accessToken)token=value.credentials.accessToken;
 if(!token)return value;
 const sample=await credentialValidity(tx,auth,token,input.requestId??'');
 const actor=sample.actor;
 // Only same-current-owner responses publish admission; browsers cannot adopt it.
 const roomId=value.participant?.association?.roomId??value.room?.roomId??input.roomId;
 if(roomId&&['game','desktop'].includes(actor.kind)){
  const room=await readRoomAdmissions(tx,db,compat,auth.now(),roomId);
  const w=room.worlds.find(w=>w.scope.principalId===actor.accountId&&(actor.kind==='game'?w.owner.gameSessionId===actor.sessionId:w.owner.desktopId===actor.sessionId)&&(!input.expectedAttachmentId||w.owner.attachmentId===input.expectedAttachmentId));
  if(w){const admission=serializeAdmission(w,actor,input.requestId??'');admission.decision={...admission.decision,remainingMs:Math.min(admission.decision.remainingMs,remaining(actor.accessUntil,auth.now(),30000))};value={...value,canonicalAdmission:admission};}
 }
 return {...value,credentialValidity:sample.value};
}
