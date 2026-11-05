import {readRoomAdmissions} from './admission.js';
import {PolicyError} from '../g1/security.js';
import type {Database,Tx} from '../g1/database.js';
import type {CandidateRooms} from '../g2-next/rooms.js';
import {afterCommit} from '../g1/commit-hooks.js';
import {liveScopeDigest} from '../../../../packages/domain/src/g3-live.js';
import type {Scope} from '../../../../packages/contracts/g3-live/g3-live.js';

interface DiagnosticWorld {sessionId:string;protected:boolean;authorized:boolean;terminal:boolean;reason:string}
export interface LiveMember {scope:Scope;digest:string;generation:number;until:number;parentId:string;familyId:string;lastPose:number}
export interface ProjectedRoom {id:string;nonce:object;version:number;members:Map<string,LiveMember>;poseFences:Map<string,LiveMember>;terminal:boolean;normalEnd:boolean}
// Every service transaction observes only the bounded watched set. This covers
// indirect family/account/session changes as well as explicit lobby transitions.
// No pose/snapshot code invokes this adapter or the database.
export class LiveProjection {
 readonly rooms=new Map<string,ProjectedRoom>();private version=0;private generation=0;
 observeWorlds?:((room:ProjectedRoom,worlds:DiagnosticWorld[])=>void)|undefined;
 onChange:(room:ProjectedRoom)=>void=()=>{};
 constructor(readonly db:Database,readonly authority:CandidateRooms,readonly now:()=>number){
  db.transactionProjection={prepare:tx=>this.prepare(tx),uncertain:()=>this.invalidateAll()};
 }
 watch(id:string){let r=this.rooms.get(id);if(!r){if(this.rooms.size>=128)throw new PolicyError('CAPACITY');r={id,nonce:{},version:0,members:new Map(),poseFences:new Map(),terminal:false,normalEnd:false};this.rooms.set(id,r);}return r;}
 invalidateAll(){for(const r of this.rooms.values()){r.members.clear();r.terminal=true;try{this.onChange(r);}catch{}}this.rooms.clear();}
 private async read(tx:Tx,id:string):Promise<{terminal:boolean;normalEnd?:boolean;members:Omit<LiveMember,'generation'|'lastPose'>[];ownerSessionIds?:string[];diagnosticWorlds?:DiagnosticWorld[]}>{
  const admission=await readRoomAdmissions(tx,this.db,this.authority.compatibility,this.now(),id);
  const members=admission.worlds.filter(w=>w.decision.playing&&['core','race'].includes(w.mode)&&w.capabilities.includes('core.ghost')).map(w=>({scope:w.scope,digest:liveScopeDigest(w.scope),until:w.until,parentId:w.owner.desktopId,familyId:w.owner.familyId}));
  return {terminal:admission.terminal,normalEnd:admission.normalEnd,members,ownerSessionIds:admission.ownerSessionIds,...(this.observeWorlds?{diagnosticWorlds:admission.worlds.map(w=>({sessionId:w.scope.sessionId,protected:w.currentReport.protected,authorized:w.decision.authorized,terminal:w.decision.terminal,reason:w.decision.reason}))}:{})};
 }

 async authorize(tx:Tx,scope:Scope){const r=await this.read(tx,scope.roomId!);if(r.terminal||!r.members.some(m=>m.digest===liveScopeDigest(scope)))throw new PolicyError('FORBIDDEN');}
 async prepare(tx:Tx){
  const version=++this.version;if(!Number.isSafeInteger(version))throw Error('CAPACITY');
  const prepared:{room:ProjectedRoom;result:{terminal:boolean;normalEnd?:boolean;members:Omit<LiveMember,'generation'|'lastPose'>[];ownerSessionIds?:string[];diagnosticWorlds?:DiagnosticWorld[]}}[]=[];
  // A failed read is not evidence of terminal authority. Reject preparation so
  // the owning transaction rolls back; only a successful canonical read may
  // publish removal. The prior committed lease is neither renewed nor extended.
  for(const room of [...this.rooms.values()])prepared.push({room,result:await this.read(tx,room.id)});
  if(!prepared.length)return;
  // One bounded callback for the full watched set leaves room for token effects;
  // 128 rooms cannot exhaust the 128-effect transaction hook limit.
  afterCommit(tx,()=>{for(const {room,result}of prepared){
   if(this.rooms.get(room.id)!==room||room.terminal||version<=room.version)continue;
   try{
   room.version=version;const next=new Map<string,LiveMember>();
   const owners=new Set(result.ownerSessionIds??result.members.map(m=>m.scope.sessionId!));
   for(const id of room.poseFences.keys())if(!owners.has(id))room.poseFences.delete(id);
   for(const m of result.members){const old=room.members.get(m.scope.sessionId!);const same=old?.digest===m.digest&&old.until>this.now();const generation=same?old.generation:++this.generation;if(generation>0xffffffff)throw Error('CAPACITY');const fence=room.poseFences.get(m.scope.sessionId!),member={...m,generation,lastPose:fence?.digest===m.digest?fence.lastPose:0};next.set(m.scope.sessionId!,member);room.poseFences.set(m.scope.sessionId!,member);if(room.poseFences.size>32)throw Error('CAPACITY');}
   room.members=next;room.terminal=result.terminal;room.normalEnd=!!result.normalEnd;try{this.observeWorlds?.(room,result.diagnosticWorlds??[]);}catch{/* Diagnostic observer cannot affect projection. */}this.onChange(room);if(room.terminal)this.rooms.delete(room.id);
   }catch{room.members.clear();room.terminal=true;try{this.onChange(room);}catch{}this.rooms.delete(room.id);}
  }},()=>this.invalidateAll());
 }

 member(roomId:string,sessionId:string){const r=this.rooms.get(roomId),m=r?.members.get(sessionId);return !r?.terminal&&m&&m.until>this.now()?m:undefined;}
}
