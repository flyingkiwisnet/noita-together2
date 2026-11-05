// Isolated fixture transport filtering. Production authority/admission unchanged.
import type {Credentials} from '../../../packages/contracts/g2-next/g2-next.js';
import type {WebSocket,RawData} from 'ws';
import {decode} from '../../../packages/contracts/src/g3-live.js';
import {AsyncLocalStorage} from 'node:async_hooks';import {randomUUID} from 'node:crypto';
import {LIVE_POLICY} from '../../../packages/domain/src/g3-live.js';
import {hash,PolicyError} from '../src/g1/security.js';
import type {startG3Fixture} from './g3-loot01-fixture.js';
import {LifecycleNetwork,NETWORK_LIMITS,type NetworkProfile} from './lifecycle-lc1-network.js';
type Service=Awaited<ReturnType<typeof startG3Fixture>>['service'];
type Identity={accountId:string;deviceId:string;sessionId:string;sessionEpoch:string};
export const LC1_FIXTURE_MESSAGE_LIMITS={frames:64,bytes:65536,aggregateFrames:4096,aggregateBytes:4194304} as const;
export type TransportTarget='consumer'|'peer';
export class LifecycleTransport {
 private service:Service|undefined;
 private owners=new Map<TransportTarget,Pick<Identity,'accountId'|'deviceId'>&{roomId:string}>();
 private cuts={http:new Set<TransportTarget|'all'>(),wss:new Set<TransportTarget|'all'>()};
 private sockets=new Map<WebSocket,Identity>();
 private socketControls=new Map<WebSocket,{stop:(code:number,reason:string)=>void;ingress:()=>{frames:number;bytes:number}}>();
 private rawFrames=0;private rawBytes=0;private rawOverflow=0;private rawCancelled=0;private rawHighWater={frames:0,bytes:0,maxSocketFrames:0,maxSocketBytes:0};
 private requests=new WeakMap<object,{arrival:number;generations:{consumer:number;peer:number};identity?:Identity;target?:TransportTarget;cancelled:boolean}>();
 private authContext=new AsyncLocalStorage<{initial:boolean;identity:Identity|undefined}>();
 private refusal:{armId:string;armed:boolean;consumedCount:number}|undefined;private refusalArms=0;private refusalTotal=0;
 constructor(readonly network=new LifecycleNetwork()){network.onReset=(target,reason)=>{for(const [socket,identity]of this.sockets)if(this.role(identity)===target)this.socketControls.get(socket)?.stop(1012,reason);};}
 networkProfile(target:TransportTarget,profile:NetworkProfile,seed?:number,durationMs?:number){if(!['consumer','peer'].includes(target)||!this.owners.has(target))throw Error('FIXTURE_NETWORK_ARGUMENT');this.network.configure(target,profile,seed,durationMs);return this.networkStatus();}
 networkStatus(){const scheduled=this.network.status(),byTarget={consumer:{frames:0,bytes:0},peer:{frames:0,bytes:0},unclassified:{frames:0,bytes:0}};for(const [socket,c]of this.socketControls){const bucket=byTarget[this.role(this.sockets.get(socket))??'unclassified'],q=c.ingress();bucket.frames+=q.frames;bucket.bytes+=q.bytes;}return {...scheduled,rawIngress:{limits:LC1_FIXTURE_MESSAGE_LIMITS,pending:{frames:this.rawFrames,bytes:this.rawBytes},highWater:{...this.rawHighWater},overflow:this.rawOverflow,clearedFrames:this.rawCancelled,byTarget},injectionBufferUpperBound:{currentPayloadBytes:this.rawBytes+scheduled.pending.bytes,limitPayloadBytes:LC1_FIXTURE_MESSAGE_LIMITS.aggregateBytes+NETWORK_LIMITS.bytes,sharedIngressHeadsDoubleCounted:true,excludes:'application/TLS/kernel buffers and non-payload heap'}};}
 networkReset(target?:TransportTarget){if(target!==undefined&&(!['consumer','peer'].includes(target)||!this.owners.has(target)))throw Error('FIXTURE_NETWORK_ARGUMENT');this.network.reset(target);return this.networkStatus();}
 close(){this.network.close();for(const c of [...this.socketControls.values()])c.stop(1001,'fixture close');}
 private role(identity?:Pick<Identity,'accountId'|'deviceId'>){if(!identity)return;const matches=[...this.owners].filter(([,owner])=>owner.accountId===identity.accountId&&owner.deviceId===identity.deviceId);return matches.length===1?matches[0]![0]:undefined;}
 participantMatches(target:TransportTarget,identity:Pick<Identity,'accountId'|'deviceId'>,roomId:string){const owner=this.owners.get(target);return !!owner&&owner.accountId===identity.accountId&&owner.deviceId===identity.deviceId&&owner.roomId===roomId;}
 denyWssOnce(target:'consumer'){
  if(target!=='consumer'||!this.owners.has('consumer')||this.refusal?.armed||this.refusalArms>=8)throw Error('FIXTURE_WSS_REFUSAL_ARGUMENT');
  this.refusalArms++;this.refusal={armId:randomUUID(),armed:true,consumedCount:0};return this.wssRefusalStatus();
 }
 wssRefusalStatus(){return {target:'consumer',code:'FORBIDDEN',armed:this.refusal?.armed??false,armId:this.refusal?.armId??null,consumedCount:this.refusal?.consumedCount??0,totalConsumed:this.refusalTotal,armsUsed:this.refusalArms,armLimit:8};}
 clearWssRefusal(){if(this.refusal)this.refusal.armed=false;return this.wssRefusalStatus();}
 private matches(kind:'http'|'wss',identity?:Pick<Identity,'accountId'|'deviceId'>){const cut=this.cuts[kind];return cut.has('all')||!!identity&&[...this.owners].some(([role,owner])=>cut.has(role)&&owner.accountId===identity.accountId&&owner.deviceId===identity.deviceId);}
 async bind(target:TransportTarget,credentials:Credentials,roomId:string){
  const actor=await this.service!.db.transaction(tx=>this.service!.auth.authenticate(tx,credentials.accessToken!),true,AbortSignal.timeout(2500));
  if(actor.kind!=='desktop'||actor.accountId!==credentials.accountId||actor.deviceId!==credentials.deviceId)throw Error('FIXTURE_TRANSPORT_OWNER');
  const old=this.owners.get(target);if(old&&(old.accountId!==actor.accountId||old.deviceId!==actor.deviceId))throw Error('FIXTURE_TRANSPORT_OWNER');
  if(!/^[a-f0-9-]{36}$/.test(roomId))throw Error('FIXTURE_TRANSPORT_OWNER');
  this.owners.set(target,{accountId:actor.accountId,deviceId:actor.deviceId,roomId});
 }
 async transport(kind:'http'|'wss',enabled:boolean,target?:TransportTarget){
  if(!['http','wss'].includes(kind)||typeof enabled!=='boolean'||target!==undefined&&!['consumer','peer'].includes(target)||target!==undefined&&!this.owners.has(target))throw Error('FIXTURE_TRANSPORT_ARGUMENT');
  const key=target??'all';enabled?this.cuts[kind].add(key):this.cuts[kind].delete(key);
  // HTTP and WSS are independent. Full outage is two explicit cuts.
  if(enabled){for(const role of target?[target]:['consumer','peer'] as TransportTarget[]){this.network.transition(role,kind+'-cut');this.network.cancel(role,kind);}if(kind==='wss')for(const [socket,identity]of this.sockets)if(this.matches('wss',identity))this.socketControls.get(socket)?.stop(1012,'fixture transport cut');}
 }
 private async bearer(token:string):Promise<Identity|undefined>{
  try{const actor=await this.service!.db.transaction(tx=>this.service!.auth.authenticate(tx,token),true,AbortSignal.timeout(2500));return {accountId:actor.accountId,deviceId:actor.deviceId,sessionId:actor.sessionId,sessionEpoch:actor.sessionEpoch};}catch{return;}
 }
 install(service:Service){
  this.service=service;
  service.app.addHook('preClose',async()=>{this.close();});
  service.app.addHook('onRequest',async(req,reply)=>{
   const state={arrival:this.network.clock.now(),generations:{consumer:this.network.generation('consumer'),peer:this.network.generation('peer')},cancelled:false};this.requests.set(req,state);
   const cancel=()=>{state.cancelled=true;this.network.cancelKey(req);};req.raw.once('aborted',cancel);reply.raw.once('close',cancel);
   if(this.matches(req.url==='/v3/live'?'wss':'http'))await reply.header('x-lc1-fixture-transport-cut','1').code(503).send({error:{code:'SERVICE_UNAVAILABLE'}});
  });
  service.app.addHook('preHandler',async(req,reply)=>{
   const captured=this.requests.get(req),scopeChanged=captured&&(['consumer','peer'] as const).some(role=>captured.generations[role]!==this.network.generation(role));
   if(req.url==='/v3/live'||!this.cuts.http.size&&!this.network.enabled&&!scopeChanged)return;
   const classificationAt=this.network.clock.now(),token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];let identity=token?await this.bearer(token):undefined;
   const secret=(req.body as any)?.refresh?.secret;
   if(!identity&&req.url==='/v1/rpc'&&typeof secret==='string'&&/^[A-Za-z0-9_-]{43}$/.test(secret)){
    // Match the actual hashed bearer refresh secret to its durable family and
    // session. A body device/scope claim never selects the participant.
    identity=await service.db.transaction(async tx=>(await tx.query('SELECT f.account_id AS "accountId",f.device_id AS "deviceId",s.id AS "sessionId",s.epoch AS "sessionEpoch" FROM refresh_tokens r JOIN session_families f ON f.id=r.family_id JOIN sessions s ON s.id=r.session_id AND s.family_id=f.id WHERE r.hash=$1',[hash(secret)])).rows[0],true,AbortSignal.timeout(2500));
   }
   if(identity&&this.matches('http',identity)){await reply.header('x-lc1-fixture-transport-cut','1').code(503).send({error:{code:'SERVICE_UNAVAILABLE'}});return;}
   const state=this.requests.get(req),target=this.role(identity);if(!state||!target||!identity)return;state.identity=identity;state.target=target;
   if(state.cancelled||state.generations[target]!==this.network.generation(target)){reply.hijack();reply.raw.destroy();return;}
   const length=Number(req.headers['content-length']),bytes=Number.isSafeInteger(length)&&length>=0?length:65536;
   const delayed=this.network.enqueue(target,'httpIn',req,state.arrival,bytes,()=>{},()=>{state.cancelled=true;reply.raw.destroy();},{classificationMs:this.network.clock.now()-classificationAt,rawQueueMs:Math.max(0,classificationAt-state.arrival)});if(!await delayed.done){reply.hijack();return;}
  });
  service.app.addHook('onSend',async(req,reply,payload)=>{
   const state=this.requests.get(req);if(!state?.target)return payload;
   if(state.cancelled||state.generations[state.target]!==this.network.generation(state.target)||this.matches('http',state.identity)){reply.raw.destroy();return payload;}
   const bytes=typeof payload==='string'?Buffer.byteLength(payload):Buffer.isBuffer(payload)?payload.length:262144;
   await this.network.enqueue(state.target,'httpOut',req,this.network.clock.now(),bytes,()=>{},()=>{state.cancelled=true;reply.raw.destroy();}).done;return payload;
  });
  const live=service.live;if(!live)throw Error('FIXTURE_LIVE_REQUIRED');const original=live.connect.bind(live);
  const authorize=live.projection.authorize.bind(live.projection);
  live.projection.authorize=async(tx,scope)=>{
   // First pass the actual current canonical admission. Only the existing live
   // authenticate error path then injects FORBIDDEN; never fabricate a frame.
   await authorize(tx,scope);
   const context=this.authContext.getStore(),arm=this.refusal,owner=this.owners.get('consumer'),identity=context?.identity;
   if(!context?.initial||!arm?.armed||!owner||!identity||identity.accountId!==owner.accountId||identity.deviceId!==owner.deviceId||scope.roomId!==owner.roomId||scope.principalId!==identity.accountId||scope.sessionId!==identity.sessionId||scope.sessionEpoch!==identity.sessionEpoch)return;
   const current=(await tx.query('SELECT 1 FROM world_associations w JOIN attachments a ON a.id=w.attachment_id JOIN sessions s ON s.id=a.game_session_id WHERE w.world_id=$1 AND w.account_id=$2 AND w.device_id=$3 AND a.id=$4 AND s.id=$5 AND s.epoch=$6 AND a.desktop_id=s.parent_id AND a.revoked_at IS NULL AND s.revoked_at IS NULL AND w.terminal_reason IS NULL',[scope.worldToken,identity.accountId,identity.deviceId,scope.attachmentId,identity.sessionId,identity.sessionEpoch])).rowCount;
   if(!current||this.refusal!==arm||!arm.armed)return;
   // JS metadata deliberately survives this refused/rolled-back auth attempt;
   // one arm is consumed once, after all verified eligibility guards passed.
   arm.armed=false;arm.consumedCount=1;this.refusalTotal++;throw new PolicyError('FORBIDDEN');
  };
  live.connect=(socket:WebSocket)=>{
   // WSS deliberately has no Authorization upgrade header. Intercept only its
   // bounded authenticate/reauthenticate frame, verify current issuer+SQL
   // identity, then deliver the original frame to untouched live admission.
   const emit=socket.emit.bind(socket),send=socket.send.bind(socket),close=socket.close.bind(socket);let draining=false,stopped=false,buffered=0,outstanding=0,pendingClose:(()=>void)|undefined;
   const queue:{args:any[];bytes:number;arrival:number;generations:{consumer:number;peer:number}}[]=[];
   const clear=()=>{if(stopped)return;stopped=true;this.rawCancelled+=queue.length;this.rawFrames-=queue.length;this.rawBytes-=buffered;queue.length=0;buffered=0;pendingClose=undefined;this.network.cancelKey(socket);this.sockets.delete(socket);this.socketControls.delete(socket);};
   const stop=(code:number,reason:string)=>{clear();if(socket.readyState===1)close(code,reason);};this.socketControls.set(socket,{stop,ingress:()=>({frames:queue.length,bytes:buffered})});
   socket.once('close',clear);socket.once('error',clear);
   socket.send=((data:any,options?:any,callback?:any)=>{
    const done=typeof options==='function'?options:callback,opts=typeof options==='function'?undefined:options,target=this.role(this.sockets.get(socket));
    if(!target||!this.network.active(target)){return (send as any)(data,opts,done);}
    if(stopped||socket.readyState!==1){done?.(Error('FIXTURE_NETWORK_CLOSED'));return;}
    const bytes=Buffer.isBuffer(data)?data:Buffer.from(data),copy=Buffer.from(bytes);outstanding++;
    const result=this.network.enqueue(target,'wssOut',socket,this.network.clock.now(),copy.length,()=>{try{(send as any)(copy,opts,(error?:Error)=>{if(error&&socket.readyState===1)socket.emit('error',error);});}finally{outstanding--;if(!outstanding&&pendingClose){const finish=pendingClose;pendingClose=undefined;finish();}}},()=>{outstanding--;stop(1012,'fixture network cancelled');});
    // send completion acknowledges bounded local buffering, not propagation.
    // Waiting for the delay here would throttle every snapshot to1/latency.
    if(result.accepted)queueMicrotask(()=>done?.());else done?.(Error('FIXTURE_NETWORK_CAPACITY'));
   }) as typeof socket.send;
   socket.close=((code?:number,reason?:string|Buffer)=>{if(outstanding&&!stopped){pendingClose=()=>close(code,reason);return;}close(code,reason);}) as typeof socket.close;
   const drain=async()=>{
    if(draining||stopped)return;draining=true;
    try{while(queue.length&&!stopped){
     const entry=queue[0]!,data=entry.args[0] as Buffer,binary=entry.args[1],headAt=this.network.clock.now();let frame:any;
     try{if(binary&&data.length<=LIVE_POLICY.frameBytes)frame=decode('ClientFrame',data);}catch{/* Untouched admission owns malformed-frame rejection. */}
     const token=frame?.authenticate?.accessToken??frame?.reauthenticate?.accessToken;
     if(typeof token==='string'){
      const classifyAt=this.network.clock.now(),identity=await this.bearer(token),classificationMs=this.network.clock.now()-classifyAt;if(stopped||socket.readyState!==1)return;
      if(identity&&this.matches('wss',identity)){stop(1012,'fixture transport cut');return;}
      if(identity&&!this.sockets.has(socket))this.sockets.set(socket,identity);
      const target=this.role(this.sockets.get(socket));
      if(target&&entry.generations[target]!==this.network.generation(target)){stop(1012,'fixture profile changed');return;}
      const deliver=()=>this.authContext.run({initial:!!frame.authenticate,identity},()=>emit('message',...entry.args));
      if(target){if(!await this.network.enqueue(target,'wssIn',socket,entry.arrival,entry.bytes,deliver,()=>stop(1012,'fixture network cancelled'),{classificationMs,rawQueueMs:Math.max(0,headAt-entry.arrival)}).done)return;}else deliver();
     }else{
      const target=this.role(this.sockets.get(socket));if(target&&entry.generations[target]!==this.network.generation(target)){stop(1012,'fixture profile changed');return;}
      if(target){if(!await this.network.enqueue(target,'wssIn',socket,entry.arrival,entry.bytes,()=>emit('message',...entry.args),()=>stop(1012,'fixture network cancelled'),{classificationMs:0,rawQueueMs:Math.max(0,headAt-entry.arrival)}).done)return;}else emit('message',...entry.args);
     }
     if(!stopped){queue.shift();buffered-=entry.bytes;this.rawFrames--;this.rawBytes-=entry.bytes;}
    }}finally{draining=false;}
   };
   socket.emit=((event:string,...args:any[])=>{
    if(event!=='message')return emit(event,...args);
    if(stopped)return true;
    const data=args[0] as RawData,bytes=Buffer.isBuffer(data)?data:data instanceof ArrayBuffer?Buffer.from(data):Buffer.concat(data);
    // Count the in-flight lookup frame too. No later pose/control/malformed
    // message may overtake it, and no borrowed buffer escapes the byte bound.
    if(queue.length>=LC1_FIXTURE_MESSAGE_LIMITS.frames||buffered+bytes.length>LC1_FIXTURE_MESSAGE_LIMITS.bytes||this.rawFrames>=LC1_FIXTURE_MESSAGE_LIMITS.aggregateFrames||this.rawBytes+bytes.length>LC1_FIXTURE_MESSAGE_LIMITS.aggregateBytes){this.rawOverflow++;clear();socket.close(1013,'fixture message queue capacity');return true;}
    queue.push({args:[Buffer.from(bytes),...args.slice(1)],bytes:bytes.length,arrival:this.network.clock.now(),generations:{consumer:this.network.generation('consumer'),peer:this.network.generation('peer')}});buffered+=bytes.length;this.rawFrames++;this.rawBytes+=bytes.length;this.rawHighWater.frames=Math.max(this.rawHighWater.frames,this.rawFrames);this.rawHighWater.bytes=Math.max(this.rawHighWater.bytes,this.rawBytes);this.rawHighWater.maxSocketFrames=Math.max(this.rawHighWater.maxSocketFrames,queue.length);this.rawHighWater.maxSocketBytes=Math.max(this.rawHighWater.maxSocketBytes,buffered);
    void drain().catch(()=>{clear();if(socket.readyState===1)socket.close(1008,'fixture message dispatch failed');});
    return true;
   }) as typeof socket.emit;
   original(socket);
  };
 }
}
