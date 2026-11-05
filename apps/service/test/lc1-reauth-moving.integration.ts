// Server-only diagnostic: actual TLS/SQL/LC1/WSS, synthetic engine and pose input.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';import {livePeer} from '../scripts/g3-live-fixture.js';
import {LiveService} from '../src/g3/live-service.js';
import {decode} from '../../../packages/contracts/src/g3-live.js';import {liveScopeDigest} from '../../../packages/domain/src/g3-live.js';
const sleep=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));
test('moving peers keep current canonical renewal and WSS reauthentication; exact stale guards remain diagnostic',async()=>{
 const x=await startLifecycleLc1Fixture(),sockets:Awaited<ReturnType<typeof livePeer>>[]=[],events:any[]=[],timeline:any[]=[];let timer:NodeJS.Timeout|undefined,release:()=>void=()=>{};
 try{
  const peer=await x.bootstrapPeer(),a=x.request(),b=x.request(peer.desktop,1,'moving-peer',peer.observation);b.reconcile!.association=peer.association;b.reconcile!.proof=peer.proof;
  await x.send(x.desktop,a);await x.send(peer.desktop,b);await x.start();a.reconcile!.observation={...x.observation,protected:false,observationRevision:2};b.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};
  const initial=[await x.send(x.desktop,a),await x.send(peer.desktop,b)],live=x.fixture.service.live as any;
  if(process.env.NT2_LC1_DIRECT_LIVE==='1')live.connect=LiveService.prototype.connect.bind(live);
  const oldConnect=live.connect.bind(live),oldRetire=live.retire.bind(live),last=new WeakMap<object,any>();
  live.connect=(socket:any)=>{oldConnect(socket);const c:any=[...live.connections].find((c:any)=>c.socket===socket);socket.prependListener('message',(bytes:Buffer)=>{const f=decode('ClientFrame',bytes) as any,bind=c.binding,m=bind&&live.projection.member(bind.scope.roomId,bind.scope.sessionId);if(!bind)return;timeline.push({direction:'in',kind:f.pose?'pose':f.reauthenticate?'reauth':'control',sequence:f.sequence,bound:bind.lastSequence});last.set(c,{input:f.pose?'pose':f.reauthenticate?'reauth':'control',frameSequence:f.sequence,boundFrameSequence:bind.lastSequence,frameSequenceNewer:f.sequence>bind.lastSequence,authorityMatches:f.authorityGeneration===bind.authorityGeneration,scopeMatches:liveScopeDigest(f.scope)===liveScopeDigest(bind.scope),poseNewer:!f.pose||f.pose.sequence>bind.lastPoseSequence,projectionPoseNewer:!f.pose||f.pose.sequence>(m?.lastPose??0),authRevisionNext:!f.reauthenticate||f.reauthenticate.authRevision===bind.authRevision+1,busy:c.busy,deadlineLive:Date.now()<c.deadline});});};
  live.retire=(c:any,code:string,...rest:any[])=>{if(!c.retired){const sample=last.get(c),caller=new Error().stack?.includes('LiveService.authenticate')?'authenticate':'frame-or-tick';events.push({code,caller,...sample,currentBoundFrameSequence:c.binding?.lastSequence,currentBoundPoseSequence:c.binding?.lastPoseSequence,currentDeadlineLive:Date.now()<c.deadline,currentValid:live.valid(c),memberGenerationMatches:!!c.binding&&live.projection.member(c.binding.scope.roomId,c.binding.scope.sessionId)?.generation===c.binding.authorityGeneration});}return oldRetire(c,code,...rest);};
  const open=async(i:number)=>{const p=await livePeer(x.fixture as any,{games:[initial[i].credentials],scopes:[initial[i].canonicalAdmission.scope]} as any,0);sockets.push(p);const send=p.socket.send.bind(p.socket);(p.socket as any).send=(bytes:any,...args:any[])=>{const f=decode('ClientFrame',bytes) as any;timeline.push({direction:'out',peer:i,kind:f.pose?'pose':f.reauthenticate?'reauth':'control',sequence:f.sequence});return (send as any)(bytes,...args);};return p;};
  const moving=[await open(0),await open(1)],poses=[0,0];let sends=0;
  timer=setInterval(()=>{x.advance(125);moving.forEach((p,i)=>{if(p.socket.readyState===1){p.send({pose:{sequence:++poses[i]!,xMilli:String(poses[i]!*10),yMilli:'1000',vxMilli:'0',vyMilli:'0'}});sends++;}});},125);
  const auth=x.fixture.service.auth as any,original=auth.authenticate.bind(auth);let delayToken='';
  auth.authenticate=async(...args:any[])=>{const result=await original(...args);if(args[1]===delayToken){delayToken='';await new Promise<void>(r=>{release=r;setTimeout(r,200);});}return result;};
  for(let cycle=0;cycle<6;cycle++){
   const values=[await x.send(x.desktop,{...a,requestId:randomUUID()}),await x.send(peer.desktop,{...b,requestId:randomUUID()})];
   for(let i=0;i<2;i++){assert.equal(values[i].canonicalAdmission.decision.playing,true);assert.equal(values[i].credentials.sessionId,initial[i].credentials.sessionId);if(cycle===2&&i===0)delayToken=values[i].credentials.accessToken;moving[i]!.send({reauthenticate:{accessToken:values[i].credentials.accessToken,authRevision:cycle+2}});}
   for(const p of moving){const frame=await p.wait(f=>!!f.retire||f.welcome?.authRevision===cycle+2);assert.ok(frame.welcome,JSON.stringify({events,timeline:timeline.slice(-20)}));}
   await sleep(1600);
   for(const p of moving){assert.equal(p.socket.readyState,1,JSON.stringify(events));assert.ok(p.received.some(v=>v.frame.snapshot?.peers?.some(peer=>(peer.poses?.at(-1)?.sequence??0)>cycle*5)));}
  }
  clearInterval(timer);timer=undefined;assert.equal(events.length,0);assert.ok(sends>100);
  await moving[0]!.close();x.advance(1100);const revision=await open(0);revision.send({reauthenticate:{accessToken:initial[0].credentials.accessToken,authRevision:3}});assert.equal((await revision.wait(f=>!!f.retire)).retire!.code,'STALE_EPOCH');assert.equal(events.at(-1).authRevisionNext,false);assert.equal(events.at(-1).frameSequenceNewer,true);
  await revision.close();x.advance(1100);const stalePose=await open(0);stalePose.send({pose:{sequence:1,xMilli:'0',yMilli:'0',vxMilli:'0',vyMilli:'0'}});assert.equal((await stalePose.wait(f=>!!f.retire)).retire!.code,'STALE_EPOCH');assert.equal(events.at(-1).poseNewer,false);assert.equal(events.at(-1).authRevisionNext,true);
  assert.equal(moving[1]!.socket.readyState,1);
  console.log(JSON.stringify({movingReauthAudit:{namespace:x.fixture.namespace,canonicalRenewals:12,reauthAcknowledgements:12,delayedReauthMs:200,offeredPoseFrames:sends,unexpectedRetire:0,healthyPeerOpen:true,negativeGuardDiagnostics:events}}));
 }finally{clearInterval(timer);release();for(const p of sockets)await p.close();await x.close();}
});
