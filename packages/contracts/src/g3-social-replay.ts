import {createHash} from 'node:crypto';
import type {Scope} from '../g3-social/g3-social.js';
export const SOCIAL_REPLAY_POLICY='sequence-bound-transient-v1';
// Canonical UTF-8 compact JSON array; no object ordering or Unicode normalization.
// UUIDv8 is reserved for transient publications. POIs use other UUID versions.
export function transientEventId(scope:Scope,gameSessionId:string,kind:'hud'|'emote'|'cosmetic',sequence:number){
 const source=JSON.stringify([SOCIAL_REPLAY_POLICY,scope.roomId,scope.runId,scope.runEpoch,scope.configRevision,scope.attachmentId,scope.worldToken,gameSessionId,kind,sequence]);
 const bytes=createHash('sha256').update(source,'utf8').digest().subarray(0,16);
 bytes[6]=(bytes[6]!&15)|128;bytes[8]=(bytes[8]!&63)|128;
 const h=bytes.toString('hex');return `${h.slice(0,8)}-${h.slice(8,12)}-${h.slice(12,16)}-${h.slice(16,20)}-${h.slice(20)}`;
}
