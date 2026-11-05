// Production projection/service/protobuf paths; canonical SQL/auth and sockets are
// explicit memory inputs. No DB/network/native/game/rendering acceptance claimed.
import test from 'node:test';import assert from 'node:assert/strict';import {EventEmitter} from 'node:events';import {randomUUID} from 'node:crypto';
import {LiveService} from '../src/g3/live-service.js';import {liveScopeDigest} from '../../../packages/domain/src/g3-live.js';
import {encode,decode,schema} from '../../../packages/contracts/src/g3-live.js';import {publishCommitEffects} from '../src/g1/commit-hooks.js';
import type {Scope,ServerFrame} from '../../../packages/contracts/g3-live/g3-live.js';
class MemorySocket extends EventEmitter{
 readyState=1;bufferedAmount=0;frames:ServerFrame[]=[];closedCode:number|undefined;
 send(bytes:Buffer,options?:unknown,callback?:(e?:Error)=>void){this.frames.push(decode('ServerFrame',bytes) as ServerFrame);callback?.();}
 close(code:number){this.closedCode=code;this.readyState=2;}
 finish(){this.readyState=3;this.emit('close');}
 terminate(){this.finish();}
}
const flush=()=>new Promise<void>(resolve=>setImmediate(resolve));
function fixture(){
 let now=100000;const roomId=randomUUID(),runId=randomUUID(),runEpoch=randomUUID(),actors=new Map<string,any>();let members:any[]=[],owners:string[]=[];
 const tx:any={query:async(sql:string,args:any[]=[])=>({rows:sql.includes('FROM access_tokens')?(args[0] as string[]).map(id=>({id,expires_at:new Date(1000000)})):[]})};
 const db:any={serviceId:'memory-LC1-service',epoch:randomUUID(),ready:true,transaction:async(fn:any)=>{const value=await fn(tx);await db.transactionProjection.prepare(tx);publishCommitEffects(tx);return value;}};
 const auth:any={now:()=>now,authenticate:async(_tx:any,token:string)=>{const actor=actors.get(token);if(!actor)throw Error('FORBIDDEN');return actor;}};
 const live=new LiveService(db,auth,{} as any,{authenticated(){},authorizedRoom(){},take(){}} as any);clearInterval((live as any).timer);
 (live.projection as any).read=async()=>({terminal:false,members:members.map(m=>({...m})),ownerSessionIds:owners});
 const member=(previous?:any)=>{const scope:Scope={principalId:previous?.scope.principalId??randomUUID(),sessionId:randomUUID(),sessionEpoch:randomUUID(),attachmentId:randomUUID(),roomId,runId,runEpoch,configRevision:1,worldToken:previous?.scope.worldToken??randomUUID()},token=randomUUID(),m={scope,digest:liveScopeDigest(scope),until:now+30000,parentId:previous?.parentId??randomUUID(),familyId:previous?.familyId??randomUUID(),token};actors.set(token,{kind:'game',accountId:scope.principalId,sessionId:scope.sessionId,sessionEpoch:scope.sessionEpoch,parentId:m.parentId,familyId:m.familyId,credentialId:token,accessUntil:1000000});return m;};
 const project=async(next:any[],ownerIds=next.map(m=>m.scope.sessionId))=>{members=next;owners=ownerIds;live.projection.watch(roomId);await live.projection.prepare(tx);publishCommitEffects(tx);await flush();};
 const connect=async(m:any)=>{const socket=new MemorySocket(),connectionId=randomUUID();live.connect(socket as any);let sequence=0;const send=(payload:any)=>socket.emit('message',Buffer.from(encode('ClientFrame',{major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,connectionId,clientGeneration:1,authorityGeneration:socket.frames.find(f=>f.welcome)?.authorityGeneration??0,scope:m.scope,sequence:++sequence,...payload})),true);send({authenticate:{accessToken:m.token}});await flush();return {socket,send,pose:(n:number)=>send({pose:{sequence:n,xMilli:String(n*100),yMilli:'0',vxMilli:'0',vyMilli:'0'}})};};
 const tick=async()=>{(live as any).tick();await flush();};
 const snapshot=(s:MemorySocket)=>s.frames.filter(f=>f.snapshot).at(-1)?.snapshot;
 return {live,member,project,connect,tick,snapshot,advance:(ms:number)=>{now+=ms;},close:()=>live.close()};
}

test('Continue new game scope resets only resumed pose floor; healthy socket and fresh bidirectional peer snapshots survive',async()=>{
 const f=fixture();try{
  const a=f.member(),b=f.member();await f.project([a,b]);const ca=await f.connect(a),cb=await f.connect(b);ca.pose(120);cb.pose(220);await f.tick();assert.equal(f.snapshot(cb.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,120);assert.equal(f.snapshot(ca.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,220);
  const peerGeneration=f.live.projection.member(b.scope.roomId!,b.scope.sessionId!)!.generation;
  await f.project([b]);assert.equal(ca.socket.readyState,2);assert.equal(cb.socket.readyState,1);cb.pose(221);await f.tick();assert.equal((f.snapshot(cb.socket)?.peers??[]).length,0);
  const resumed=f.member(a);await f.project([resumed,b]);const current=await f.connect(resumed);assert.ok(current.socket.frames.some(v=>v.welcome));current.pose(1);cb.pose(222);await f.tick();assert.equal(current.socket.readyState,1);assert.equal(cb.socket.readyState,1);assert.equal(f.live.projection.member(b.scope.roomId!,b.scope.sessionId!)!.generation,peerGeneration);
  assert.equal(f.snapshot(cb.socket)?.peers?.[0]?.sessionId,resumed.scope.sessionId);assert.equal(f.snapshot(cb.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,1);assert.equal(f.snapshot(current.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,222);
  const accepted=f.live.stats.receivedPoses;ca.pose(999);ca.socket.finish();assert.equal(f.live.stats.receivedPoses,accepted);current.pose(2);cb.pose(223);await f.tick();assert.equal(f.snapshot(cb.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,2);assert.equal(f.snapshot(current.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,223);
  const obsolete=await f.connect(a);assert.ok(obsolete.socket.frames.some(v=>v.retire?.code==='FORBIDDEN'));assert.equal(cb.socket.readyState,1);
 }finally{f.close();}
});

test('same canonical game scope reconnect retains pose floor: reset rejected, next sequence recovers without retiring healthy peer',async()=>{
 const f=fixture();try{const a=f.member(),b=f.member();await f.project([a,b]);const first=await f.connect(a),peer=await f.connect(b);first.pose(120);peer.pose(220);first.socket.finish();const reset=await f.connect(a);reset.pose(1);await flush();assert.ok(reset.socket.frames.some(v=>v.retire?.code==='STALE_EPOCH'));assert.equal(peer.socket.readyState,1);
  const current=await f.connect(a);reset.socket.finish();current.pose(121);peer.pose(221);await f.tick();assert.equal(current.socket.readyState,1);assert.equal(f.snapshot(peer.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,121);assert.equal(f.snapshot(current.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,221);
 }finally{f.close();}
});

test('temporary held member retains its scope pose fence, renews on return and does not reset healthy peer generation',async()=>{
 const f=fixture();try{const a=f.member(),b=f.member();await f.project([a,b]);const old=await f.connect(a),peer=await f.connect(b);old.pose(300);peer.pose(400);const peerGen=f.live.projection.member(b.scope.roomId!,b.scope.sessionId!)!.generation;
  await f.project([b],[a.scope.sessionId,b.scope.sessionId]);old.socket.finish();f.advance(3000);await f.project([{...a,until:130000},{...b,until:130000}]);assert.equal(f.live.projection.member(a.scope.roomId!,a.scope.sessionId!)!.lastPose,300);assert.equal(f.live.projection.member(b.scope.roomId!,b.scope.sessionId!)!.generation,peerGen);const current=await f.connect(a);current.pose(301);peer.pose(401);await f.tick();assert.equal(f.snapshot(peer.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,301);assert.equal(f.snapshot(current.socket)?.peers?.[0]?.poses?.at(-1)?.sequence,401);
 }finally{f.close();}
});
