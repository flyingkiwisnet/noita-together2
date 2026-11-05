import {prepareC2Events,c2Sequence,revokeC2Readiness} from './c2.js';
import {prepareC1Events,c1Sequence} from './c1.js';
import {diagnosticTag, type ServerDebugDiagnostics} from './debug-diagnostics.js';
import {remaining} from './connection.js';
import websocket from '@fastify/websocket';
import type {TLSSocket} from 'node:tls';
import type {FastifyInstance} from 'fastify';
import type {WebSocket,RawData} from 'ws';
import type {Database,Tx} from '../g1/database.js';
import type {Auth} from '../g1/auth.js';
import type {CandidateRooms} from '../g2-next/rooms.js';
import type {TrafficBudgets} from '../g2-next/traffic.js';
import {demand,PolicyError} from '../g1/security.js';
import {afterCommit} from '../g1/commit-hooks.js';
import {WorkGate} from '../g1/work.js';
import {LIVE_POLICY as P,validateLiveFrame,acceptLiveFrame,liveScopeDigest} from '../../../../packages/domain/src/g3-live.js';
import {encode,decode,schema} from '../../../../packages/contracts/src/g3-live.js';
import type {Scope,Pose,ServerFrame} from '../../../../packages/contracts/g3-live/g3-live.js';
import {LiveProjection,type ProjectedRoom} from './live-projection.js';
import {bankRevision,prepareBankEvents,type BankChange} from './bank-events.js';

type Binding=Parameters<typeof acceptLiveFrame>[1]&{scope:Scope;tokenId:string;familyId:string};
interface Connection {socket:WebSocket;binding?:Binding;initial?:any;retired:boolean;busy:boolean;deadline:number;serverSequence:number;control:ServerFrame[];snapshot?:ServerFrame|undefined;bank?:Partial<ServerFrame>|undefined;bankDue?:number|undefined;bankRevision?:string;bankTurn?:boolean;c1?:Partial<ServerFrame>|undefined;c1Sequence?:string;c2?:Partial<ServerFrame>|undefined;c2Sequence?:string;optionalTurn?:number;c2Revoked?:boolean;sending:boolean;stalledAt:number;closeTimer?:NodeJS.Timeout;controller?:AbortController}
interface Sample {generation:number;scope:Scope;poses:Pose[];poseTimes:number[];acceptedAt:number}
const codeOf=(e:unknown)=>e instanceof PolicyError?e.code:e instanceof Error&&['CAPACITY','STALE_EPOCH','EXPIRED','FORBIDDEN'].includes(e.message)?e.message:'INVALID_MESSAGE';

export class LiveService {
 private latestBank=new Map<string,string>();
 readonly projection:LiveProjection;private connections=new Set<Connection>();private sessions=new Map<string,Connection>();private samples=new Map<string,Sample>();private admission=new WorkGate(16);private timer:NodeJS.Timeout;private closing=false;private diagnosticWorlds=new WeakMap<ProjectedRoom,Map<string,{protected:boolean;authorized:boolean;terminal:boolean;reason:string}>>();
 readonly metrics={receivedPoses:0,snapshotsSent:0,coalescedSnapshots:0,retired:0,maxConnections:0,maxQueuedBytes:0,maxControlFrames:0,maxSamples:0};
 constructor(readonly db:Database,readonly auth:Auth,rooms:CandidateRooms,readonly traffic:TrafficBudgets,readonly debug?:ServerDebugDiagnostics){
  this.debug?.bindDatabase(db);this.projection=new LiveProjection(db,rooms,auth.now);this.projection.onChange=r=>this.changed(r);if(this.debug?.enabled)this.projection.observeWorlds=(room,worlds)=>this.diagnosticWorlds.set(room,new Map(worlds.map(w=>[w.sessionId,w])));
  db.transactionProjection={prepare:async tx=>{await this.projection.prepare(tx);await this.tokens(tx);await prepareBankEvents(tx,changes=>this.bankChanged(changes));await prepareC2Events(tx,(room,sequence)=>{for(const c of this.connections)if(c.binding?.scope.roomId===room&&this.valid(c))this.queueC2(c,sequence);});await prepareC1Events(tx,(room,sequence)=>{for(const c of this.connections)if(c.binding?.scope.roomId===room&&this.valid(c))this.queueC1(c,sequence);});},uncertain:()=>this.projection.invalidateAll()};
  this.timer=setInterval(()=>this.tick(),100);this.timer.unref();
 }
 get stats(){return {...this.metrics,connections:this.connections.size,rooms:this.projection.rooms.size,samples:this.samples.size,admission:this.admission.stats};}
 private queueC2(c:Connection,sequence:string){if(c.retired||!c.binding||BigInt(sequence)<=BigInt(c.c2Sequence??'-1'))return;c.c2Sequence=sequence;c.c2={c2Changed:{sequence}};queueMicrotask(()=>this.drain(c));}
 private queueC1(c:Connection,sequence:string){if(c.retired||!c.binding||BigInt(sequence)<=BigInt(c.c1Sequence??'-1'))return;c.c1Sequence=sequence;c.c1={c1Changed:{sequence}};queueMicrotask(()=>this.drain(c));}
 private queueBank(c:Connection,revision:string){if(c.retired||!c.binding||BigInt(revision)<=BigInt(c.bankRevision??'-1'))return;c.bankRevision=revision;c.bank={bankChanged:{revision}};c.bankDue??=Date.now()+P.bankCoalesceMs;}
 private bankChanged(changes:BankChange[]){for(const change of changes){const interested=[...this.connections].filter(c=>(c.binding??c.initial)?.scope?.runId===change.runId&&!c.retired);if(!interested.length)continue;const old=this.latestBank.get(change.runId);if(old&&BigInt(old)>=BigInt(change.revision))continue;if(!old&&this.latestBank.size>=128)this.latestBank.delete(this.latestBank.keys().next().value!);this.latestBank.set(change.runId,change.revision);for(const c of interested)if(c.binding&&this.valid(c))this.queueBank(c,change.revision);}}
 private async tokens(tx:Tx){
  const ids=[...this.sessions.values()].filter(c=>!c.retired&&c.binding).map(c=>c.binding!.tokenId);if(!ids.length)return;
  const deadlines=new Map<string,number>((await tx.query('SELECT t.id,LEAST(t.expires_at,f.expires_at,q.guest_expires_at) AS expires_at FROM access_tokens t JOIN sessions s ON s.id=t.session_id JOIN session_families f ON f.id=s.family_id JOIN principal_profiles q ON q.principal_id=f.account_id WHERE t.id=ANY($1::uuid[]) AND t.revoked_at IS NULL AND s.revoked_at IS NULL AND f.revoked_at IS NULL',[ids])).rows.map(r=>[r.id,+r.expires_at]));const valid=new Set([...deadlines].filter(([,until])=>until>this.auth.now()).map(([id])=>id));
  afterCommit(tx,()=>{for(const c of this.sessions.values())if(c.binding&&deadlines.has(c.binding.tokenId))c.binding.accessUntil=Math.min(c.binding.accessUntil,deadlines.get(c.binding.tokenId)!);},()=>this.projection.invalidateAll());
  const invalid=[...this.sessions.values()].filter(c=>c.binding&&!valid.has(c.binding.tokenId));
  if(invalid.length)afterCommit(tx,()=>{for(const c of invalid)this.retire(c,'FORBIDDEN');},()=>this.projection.invalidateAll());
 }
 private changed(room:ProjectedRoom){
  for(const c of this.connections){const b=c.binding;if(!b||b.scope.roomId!==room.id)continue;const m=room.members.get(b.scope.sessionId!);
   if(room.terminal||!m||m.generation!==b.authorityGeneration||m.digest!==liveScopeDigest(b.scope)||m.until<=this.auth.now())this.retire(c,'RETIRED',room.normalEnd);else if(b.authorityUntil!==m.until){b.authorityUntil=m.until;this.send(c,{welcome:{authRevision:b.authRevision,accessExpiresAt:new Date(b.accessUntil).toISOString(),authorityUntil:new Date(m.until).toISOString(),poseHz:P.poseHz,snapshotHz:P.snapshotHz}});}
  }
  for(const [id,s]of this.samples)if(s.scope.roomId===room.id){const m=room.members.get(id);if(room.terminal||!m||m.generation!==s.generation)this.removeSample(id,s,this.closing?'shutdown':room.terminal?'terminal':!m?'member_removed':'generation_changed');}
 }
 private tags(c:Connection){const b=c.binding??c.initial;return {connectionTag:diagnosticTag(b?.connectionId),sessionTag:diagnosticTag(b?.scope?.sessionId),roomTag:diagnosticTag(b?.scope?.roomId)};}
 private removeSample(id:string,s:Sample,reason:string){
  if(this.debug?.enabled){const room=this.projection.rooms.get(s.scope.roomId!),member=room?.members.get(id),now=this.auth.now(),world=room?this.diagnosticWorlds.get(room)?.get(id):undefined;
   this.debug.operationEvent('sample_removed',{sessionTag:diagnosticTag(id),roomTag:diagnosticTag(s.scope.roomId),reason,sampleAgeMs:Math.max(0,now-s.acceptedAt),sampleGeneration:s.generation,memberGeneration:member?.generation,memberRemainingMs:member?Math.max(0,member.until-now):0,lastAcceptedPoseSequence:s.poses.at(-1)?.sequence,eligible:!!this.projection.member(s.scope.roomId!,id),terminal:room?.terminal??true,normalEnd:room?.normalEnd??false,worldPresent:!!world,...(world?{protected:world.protected,authorized:world.authorized,canonicalTerminal:world.terminal,admissionReason:world.reason||'other'}:{})});
  }
  this.samples.delete(id);
 }
 private valid(c:Connection){const b=c.binding;if(!b)return false;const m=this.projection.member(b.scope.roomId!,b.scope.sessionId!);return this.db.ready&&b.serviceEpoch===this.db.epoch&&b.serviceId===this.db.serviceId&&m?.generation===b.authorityGeneration&&m.digest===liveScopeDigest(b.scope)&&this.auth.now()<b.accessUntil&&this.auth.now()<m.until;}
 private frame(c:Connection,payload:Partial<ServerFrame>):ServerFrame{
  const b=c.binding??c.initial;if(!b||c.serverSequence>=0xffffffff)throw Error('STALE_EPOCH');
  // Backpressure can outlive a credential rotation. Bind welcome payload and
  // envelope to the same current credential/revision/deadlines at drain time.
  if(payload.welcome&&c.binding)payload={welcome:{...payload.welcome,authRevision:b.authRevision,accessExpiresAt:new Date(b.accessUntil).toISOString(),authorityUntil:new Date(b.authorityUntil).toISOString()}};
  if(payload.snapshot){const now=this.auth.now();payload={snapshot:{...payload.snapshot,acceptedAt:new Date(now).toISOString(),peers:(payload.snapshot.peers??[]).flatMap(peer=>{const sample=this.samples.get(peer.sessionId!);if(!sample||now-sample.acceptedAt>=P.poseTtlMs)return [];const poses=sample.poses.map((pose,i)=>({pose,age:Math.max(0,Math.floor(now-sample.poseTimes[i]!))})).filter(v=>v.age<P.poseTtlMs);if(!poses.length)return [];return [{...peer,poses:poses.map(v=>v.pose),peerAgeMsAtSend:Math.max(0,Math.floor(now-sample.acceptedAt)),poseAgesMsAtSend:poses.map(v=>v.age)}];})}};}
  if(payload.snapshot&&this.debug?.enabled){const room=this.projection.rooms.get(b.scope.roomId!);this.debug?.snapshot(c,(payload.snapshot.peers??[]).map(p=>p.sessionId!),[...(room?.members.values()??[])].filter(m=>m.scope.sessionId!==b.scope.sessionId&&!!this.projection.member(b.scope.roomId!,m.scope.sessionId!)).map(m=>m.scope.sessionId!),{...this.tags(c),terminal:room?.terminal??true,normalEnd:room?.normalEnd??false,authorityRemainingMs:remaining(b.authorityUntil,this.auth.now(),30000),accessRemainingMs:remaining(b.accessUntil,this.auth.now(),900000),busy:c.busy});}
  return {major:1,revision:schema.revision,serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,connectionId:b.connectionId,clientGeneration:b.clientGeneration,authorityGeneration:b.authorityGeneration||1,sequence:++c.serverSequence,scope:b.scope,ackClientSequence:b.lastSequence??b.sequence??0,authorityRemainingMs:remaining(b.authorityUntil,this.auth.now(),30000),accessRemainingMs:remaining(b.accessUntil,this.auth.now(),900000),credentialId:b.tokenId??'',...payload};
 }
 private send(c:Connection,payload:Partial<ServerFrame>,snapshot=false){
  if(c.retired||c.socket.readyState!==1)return;
  // Assign wire sequence at drain time: coalesced snapshots cannot overtake control.
  if(snapshot){if(c.snapshot)this.metrics.coalescedSnapshots++;c.snapshot=payload;}else{if(c.control.length>=P.controlQueueFrames){this.retire(c,'CAPACITY');return;}c.control.push(payload);}
  this.metrics.maxControlFrames=Math.max(this.metrics.maxControlFrames,c.control.length);queueMicrotask(()=>this.drain(c));
 }
 private drain(c:Connection){
  if(c.sending||c.socket.readyState!==1)return;
  const candidates=[c.bank&&Date.now()>=(c.bankDue??0)&&this.valid(c)?c.bank:undefined,c.c1&&this.valid(c)?c.c1:undefined,c.c2&&this.valid(c)?c.c2:undefined,c.snapshot];let payload=c.control.shift();if(!payload){for(let offset=0;offset<4;offset++){const i=((c.optionalTurn??0)+offset)%4;if(candidates[i]){payload=candidates[i]!;c.optionalTurn=(i+1)%4;break;}}}if(!payload)return;
  if(payload===c.bank){c.bank=undefined;c.bankDue=undefined;}else if(payload===c.c1)c.c1=undefined;else if(payload===c.c2)c.c2=undefined;else if(payload===c.snapshot)c.snapshot=undefined;
  let bytes:Buffer;try{bytes=Buffer.from(encode('ServerFrame',this.frame(c,payload)));}catch{this.retire(c,'INVALID_MESSAGE');return;}
  const queued=c.socket.bufferedAmount+bytes.length;this.metrics.maxQueuedBytes=Math.max(this.metrics.maxQueuedBytes,queued);
  if(queued>P.outboundBytes){if(payload.bankChanged){c.bank=payload;c.bankDue=Date.now()+P.bankCoalesceMs;return;}if(payload.c2Changed){c.c2=payload;return;}if(payload.c1Changed){c.c1=payload;return;}this.retire(c,'CAPACITY');return;}
  c.sending=true;c.stalledAt=Date.now();c.socket.send(bytes,{binary:true,compress:false},e=>{c.sending=false;c.stalledAt=0;if(e){this.cleanup(c);return;}if(payload.snapshot)this.metrics.snapshotsSent++;this.drain(c);});
 }
 private retire(c:Connection,code:string,normalEnd=false){
  if(c.retired)return;this.debug?.enabled&&this.debug.operationEvent('retirement',{...this.tags(c),code,normalEnd,busy:c.busy});c.retired=true;this.metrics.retired++;this.revokeReadiness(c);c.control=[];c.snapshot=undefined;c.bank=undefined;c.c1=undefined;c.c2=undefined;
  if(c.binding&&this.sessions.get(c.binding.scope.sessionId!)===c)this.sessions.delete(c.binding.scope.sessionId!);
  // Never wait for a slow data consumer before retiring gameplay authority.
  queueMicrotask(()=>{if(!this.connections.has(c))return;c.controller?.abort();
  try{if(c.socket.readyState===1){if(!c.sending&&c.socket.bufferedAmount<P.outboundBytes&&c.initial)c.socket.send(Buffer.from(encode('ServerFrame',this.frame(c,{retire:{code,retryAfterMs:code==='CAPACITY'?250:0}}))));c.socket.close(code==='CAPACITY'?1013:normalEnd?1000:1008,code);}}catch{c.socket.terminate();}
  c.closeTimer=setTimeout(()=>{c.socket.terminate();this.cleanup(c);},P.closeDrainMs);c.closeTimer.unref();});
 }
 private revokeReadiness(c:Connection){if(c.binding&&!c.c2Revoked){c.c2Revoked=true;const session=c.binding.scope.sessionId!,room=c.binding.scope.roomId!;void this.db.transaction(tx=>revokeC2Readiness(tx,session,room)).catch(()=>{});}}
 private cleanup(c:Connection){this.revokeReadiness(c);c.retired=true;c.controller?.abort();if(c.closeTimer)clearTimeout(c.closeTimer);this.connections.delete(c);if(c.binding&&this.sessions.get(c.binding.scope.sessionId!)===c)this.sessions.delete(c.binding.scope.sessionId!);}
 private async authenticate(c:Connection,f:any,renew=false){
  return this.debug?.enabled?this.debug.transaction(this.tags(c),()=>this.authenticateObserved(c,f,renew)):this.authenticateObserved(c,f,renew);
 }
 private async authenticateObserved(c:Connection,f:any,renew=false){
  let stage='admission';this.debug?.operationEvent(renew?'renew_begin':'authenticate_begin',this.tags(c));
  let release=()=>{};c.busy=true;c.deadline=Date.now()+P.authDeadlineMs;
  const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),P.authDeadlineMs);timer.unref();c.controller=controller;
  try{
   release=this.admission.enter();demand(f.serviceId===this.db.serviceId&&f.serviceEpoch===this.db.epoch,'STALE_EPOCH');
   const token=renew?f.reauthenticate.accessToken:f.authenticate.accessToken;let room:ProjectedRoom|undefined;
   stage='db';const verified=await this.db.transaction(async tx=>{const actor=await this.auth.authenticate(tx,token);demand(actor.kind==='game'&&actor.accountId===f.scope.principalId&&actor.sessionId===f.scope.sessionId&&actor.sessionEpoch===f.scope.sessionEpoch,'FORBIDDEN');
    await this.projection.authorize(tx,f.scope);room=this.projection.watch(f.scope.roomId);
    const guest=(await tx.query('SELECT guest_expires_at FROM principal_profiles WHERE principal_id=$1',[actor.accountId])).rows[0];return {actor,tokenId:actor.credentialId!,accessUntil:Math.min(actor.accessUntil??0,guest?.guest_expires_at?+guest.guest_expires_at:Infinity),...(!renew?{bankRevision:await bankRevision(tx,f.scope.runId),c1Sequence:await c1Sequence(tx,f.scope.roomId),c2Sequence:await c2Sequence(tx,f.scope.roomId)}:{})};
   },true,controller.signal,'control');
   stage='post_commit';demand(!c.retired&&Date.now()<c.deadline&&this.db.ready&&room&&this.projection.rooms.get(room.id)===room,'STALE_EPOCH');const m=this.projection.member(f.scope.roomId,f.scope.sessionId);
   demand(m&&m.digest===liveScopeDigest(f.scope)&&m.parentId===verified.actor.parentId&&m.familyId===verified.actor.familyId,'FORBIDDEN');
   if(renew){stage='renew_bounds';const b=c.binding!;
    // Rounded/capped expiries can be equal, including a valid credential retry.
    // The next auth revision acknowledges verified bounds; it grants no time.
    demand(this.valid(c)&&m.generation===b.authorityGeneration&&b.familyId===verified.actor.familyId&&verified.accessUntil>=b.accessUntil,'STALE_EPOCH');
    b.accessUntil=verified.accessUntil;b.tokenId=verified.tokenId;b.authRevision=f.reauthenticate.authRevision;
   }
   else{this.traffic.authenticated({sessionId:f.scope.sessionId,principalId:f.scope.principalId},'health');demand(!this.sessions.has(f.scope.sessionId),'CAPACITY');c.binding={serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,connectionId:f.connectionId,clientGeneration:f.clientGeneration,authorityGeneration:m.generation,scope:structuredClone(f.scope),lastSequence:1,lastPoseSequence:m.lastPose,authRevision:1,accessUntil:verified.accessUntil,authorityUntil:m.until,tokenId:verified.tokenId,familyId:verified.actor.familyId};this.sessions.set(f.scope.sessionId,c);}
   this.send(c,{welcome:{authRevision:c.binding!.authRevision,accessExpiresAt:new Date(c.binding!.accessUntil).toISOString(),authorityUntil:new Date(m.until).toISOString(),poseHz:P.poseHz,snapshotHz:P.snapshotHz}});
   if(!renew){const latest=this.latestBank.get(f.scope.runId),initial=verified.bankRevision??'0';this.queueBank(c,latest&&BigInt(latest)>BigInt(initial)?latest:initial);this.queueC1(c,verified.c1Sequence??'0');this.queueC2(c,verified.c2Sequence??'0');}
   this.debug?.operationEvent(renew?'renew_result':'authenticate_result',{...this.tags(c),outcome:'accepted',stage:'complete',retired:c.retired});
  }catch(e){this.debug?.operationEvent(renew?'renew_result':'authenticate_result',{...this.tags(c),outcome:c.retired?'retired-other':'refused',stage,code:codeOf(e),retired:c.retired});this.retire(c,codeOf(e));}finally{clearTimeout(timer);release();c.busy=false;if(c.controller===controller)delete c.controller;}
 }
 connect(socket:WebSocket){
  if(this.connections.size>=P.connections){socket.close(1013,'CAPACITY');return;}
  const c:Connection={socket,retired:false,busy:false,deadline:Date.now()+P.authDeadlineMs,serverSequence:0,control:[],sending:false,stalledAt:0};this.connections.add(c);this.metrics.maxConnections=Math.max(this.metrics.maxConnections,this.connections.size);
  // ws handles protocol Ping/Pong independently of application frames.

  socket.on('close',()=>this.cleanup(c));socket.on('error',()=>this.cleanup(c));socket.on('message',(data:RawData,binary:boolean)=>{
   if(c.retired)return;
   try{demand(binary,'INVALID_MESSAGE');const bytes=Buffer.isBuffer(data)?data:data instanceof ArrayBuffer?Buffer.from(data):Buffer.concat(data);demand(bytes.length<=P.frameBytes,'INVALID_MESSAGE');
    const f=validateLiveFrame('ClientFrame',decode('ClientFrame',bytes));
    if(!c.binding){demand(!c.busy&&f.authenticate&&Date.now()<c.deadline,'UNAUTHENTICATED');c.initial=f;void this.authenticate(c,f);return;}
    demand(this.valid(c),'EXPIRED');acceptLiveFrame(f,c.binding,this.auth.now());
    this.traffic.authenticated({sessionId:c.binding.scope.sessionId!,principalId:c.binding.scope.principalId!},f.pose?'pose':'health');
    c.binding.lastSequence=f.sequence;
    if(f.pose){this.traffic.authorizedRoom(c.binding.scope.roomId!);const m=this.projection.member(c.binding.scope.roomId!,c.binding.scope.sessionId!)!;demand(f.pose.sequence>m.lastPose,'STALE_EPOCH');m.lastPose=f.pose.sequence;c.binding.lastPoseSequence=f.pose.sequence;
     const old=this.samples.get(m.scope.sessionId!),now=this.auth.now(),retained=old?.generation===m.generation?old.poses.map((pose,i)=>({pose,at:old.poseTimes[i]!})).filter(p=>now-p.at<P.poseTtlMs):[],history=[...retained,{pose:f.pose,at:now}].slice(-4);this.samples.set(m.scope.sessionId!,{scope:m.scope,generation:m.generation,poses:history.map(p=>p.pose),poseTimes:history.map(p=>p.at),acceptedAt:now});this.debug?.enabled&&this.debug.record('pose_accepted',{...this.tags(c),poseSequence:f.pose.sequence,sampleGeneration:m.generation,memberGeneration:m.generation,eligible:true},false);this.metrics.receivedPoses++;this.metrics.maxSamples=Math.max(this.metrics.maxSamples,this.samples.size);
    }else{this.traffic.take('live-control:'+c.binding.scope.sessionId!,P.controlFramesPerSecond);if(f.reauthenticate){demand(!c.busy,'CAPACITY');void this.authenticate(c,f,true);}else this.send(c,{pong:{}});}
   }catch(e){this.retire(c,codeOf(e));}
  });
 }
 private tick(){
  const now=this.auth.now();for(const [id,s]of this.samples)if(now-s.acceptedAt>=P.poseTtlMs||!this.projection.member(s.scope.roomId!,id))this.removeSample(id,s,now-s.acceptedAt>=P.poseTtlMs?'pose_ttl':'member_unavailable');
  // Group once per tick, bounded by 128 rooms/4096 participants.
  const groups=new Map<string,Sample[]>();for(const s of this.samples.values()){const g=groups.get(s.scope.roomId!)??[];g.push(s);groups.set(s.scope.roomId!,g);}
  for(const c of this.connections){if(c.retired)continue;if(!this.db.ready){this.retire(c,'RETIRED');continue;}if(!c.binding){if(Date.now()>=c.deadline)this.retire(c,'EXPIRED');continue;}
   if(!this.valid(c)){this.retire(c,'EXPIRED');continue;}if(c.busy&&Date.now()>=c.deadline){this.retire(c,'EXPIRED');continue;}if(c.stalledAt&&Date.now()-c.stalledAt>=P.slowConsumerDeadlineMs){this.retire(c,'CAPACITY');continue;}
   const peers=(groups.get(c.binding.scope.roomId!)??[]).filter(s=>s.scope.sessionId!==c.binding!.scope.sessionId).map(s=>({principalId:s.scope.principalId!,sessionId:s.scope.sessionId!,attachmentId:s.scope.attachmentId!,worldToken:s.scope.worldToken!,poses:s.poses.filter((_,i)=>now-s.poseTimes[i]!<P.poseTtlMs),acceptedAt:new Date(s.acceptedAt).toISOString()}));
   this.send(c,{snapshot:{peers,acceptedAt:new Date(now).toISOString()}},true);
  }
 }
 async register(app:FastifyInstance){
  await app.register(websocket,{options:{maxPayload:P.frameBytes,perMessageDeflate:false,autoPong:true,handleProtocols:protocols=>protocols.size===1&&protocols.has(P.subprotocol)?P.subprotocol:false}});
  app.get('/v3/live/info',async()=>({revision:schema.revision,serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,endpoint:P.endpoint,subprotocol:P.subprotocol,encoding:P.encoding,implemented:true,policy:P}));
  app.get(P.endpoint,{websocket:true,preValidation:async(req,reply)=>{
   if(!this.db.ready||this.connections.size>=P.connections){await reply.code(503).send();return;}
   if(!(req.raw.socket as TLSSocket).encrypted||req.headers.host!==new URL(this.auth.security.keys.issuer).host||req.headers.authorization||req.headers['sec-websocket-protocol']!==P.subprotocol){await reply.code(403).send();return;}
  }},socket=>this.connect(socket));
  app.addHook('preClose',async()=>{this.close();await this.debug?.close();});
 }
 close(){this.closing=true;clearInterval(this.timer);this.projection.invalidateAll();for(const c of this.connections){this.retire(c,'RETIRED');c.socket.terminate();this.cleanup(c);}delete this.db.transactionProjection;}
}

