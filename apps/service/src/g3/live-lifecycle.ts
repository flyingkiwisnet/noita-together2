import {randomUUID} from 'node:crypto';
import type {Tx} from '../g1/database.js';
import {afterCommit} from '../g1/commit-hooks.js';
export interface LiveParticipant {sessionId:string;scopeDigest:string;authorityGeneration:number;until:number;playing:boolean}
export interface LiveRoomProjection {serviceEpoch:string;roomId:string;version:number;retired:boolean;participants:LiveParticipant[]}
// Preparation only: C2 must build every complete projection from canonical SQL in
// the committing lifecycle transaction, never from a pose/HTTP client payload.
export class CommittedRoomAuthority {
 private rooms=new Map<string,{nonce:string;version:number;retired:boolean;projection:LiveRoomProjection|null}>();
 constructor(readonly serviceEpoch:string,readonly now:()=>number=Date.now){}
 get size(){return this.rooms.size;}
 reserve(roomId:string,committedVersion:number){
  if(!roomId||!Number.isSafeInteger(committedVersion)||committedVersion<0)throw Error('INVALID_MESSAGE');
  if(!this.rooms.has(roomId)&&this.rooms.size>=128)throw Error('CAPACITY');
  const nonce=randomUUID();this.rooms.set(roomId,{nonce,version:committedVersion,retired:false,projection:null});return nonce;
 }
 private checked(p:LiveRoomProjection){
  if(p.serviceEpoch!==this.serviceEpoch||!p.roomId||!Number.isSafeInteger(p.version)||p.version<1||typeof p.retired!=='boolean'||p.participants.length>32||new Set(p.participants.map(v=>v.sessionId)).size!==p.participants.length)throw Error('STALE_EPOCH');
  for(const a of p.participants)if(!a.sessionId||!/^[a-f0-9]{64}$/.test(a.scopeDigest)||!Number.isInteger(a.authorityGeneration)||a.authorityGeneration<1||a.authorityGeneration>0xffffffff||!Number.isSafeInteger(a.until)||typeof a.playing!=='boolean')throw Error('INVALID_MESSAGE');
  return structuredClone(p);
 }
 apply(nonce:string,p:LiveRoomProjection){
  const copy=this.checked(p),r=this.rooms.get(copy.roomId);
  if(!r||r.nonce!==nonce||r.retired||copy.version<=r.version)return false;
  r.version=copy.version;r.retired=copy.retired;r.projection=copy.retired?null:copy;return true;
 }
 stage(tx:Tx,nonce:string,p:LiveRoomProjection){const copy=this.checked(p);afterCommit(tx,()=>{this.apply(nonce,copy);},()=>this.retire(p.roomId,nonce));}
 retire(roomId:string,nonce:string){const r=this.rooms.get(roomId);if(r?.nonce===nonce){r.retired=true;r.projection=null;}}
 release(roomId:string,nonce:string){if(this.rooms.get(roomId)?.nonce===nonce)this.rooms.delete(roomId);}
 authority(roomId:string,sessionId:string){
  const r=this.rooms.get(roomId);if(!r||r.retired||!r.projection)return;
  if(r.projection.participants.some(p=>p.until<=this.now())){this.retire(roomId,r.nonce);return;}
  const actor=r.projection.participants.find(p=>p.sessionId===sessionId&&p.playing);
  return actor?{...actor}:undefined;
 }
}
