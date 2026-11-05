// Real TLS/SQL/WSS. The2.1s movement gap is an explicitly injected service-clock
// stimulus, not a reproduction of the owner's unknown ghost disappearance.
import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtemp,readFile,readdir} from 'node:fs/promises';import {join,relative} from 'node:path';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';import {livePeer} from '../scripts/g3-live-fixture.js';
import {diagnosticServiceTag} from '../src/g3/debug-diagnostics.js';
test('actual Server diagnostics persist TTL loss/recovery and correlate successful renewal with actual SQL; debug-off preserves movement',async()=>{
 const root=await mkdtemp(join(process.cwd(),'spikes/noita-entry/.build/server-debug-component-'));
 for(const enabled of [true,false]){
  const output=join(root,enabled?'on':'off'),x=await startLifecycleLc1Fixture({diagnosticCapture:{root:output,enabled}}),sockets:Awaited<ReturnType<typeof livePeer>>[]=[];
  try{
   const peer=await x.bootstrapPeer(),qa=x.request(),qb=x.request(peer.desktop,1,'debug-peer',peer.observation);qb.reconcile!.association=peer.association;qb.reconcile!.proof=peer.proof;
   await x.send(x.desktop,qa);await x.send(peer.desktop,qb);await x.start();qa.reconcile!.observation={...x.observation,protected:false,observationRevision:2};qb.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};
   const values=[await x.send(x.desktop,qa),await x.send(peer.desktop,qb)],state={games:values.map(v=>v.credentials),scopes:values.map(v=>v.canonicalAdmission.scope)};
   sockets.push(await livePeer(x.fixture as any,state as any,0),await livePeer(x.fixture as any,state as any,1));const a=sockets[0]!,b=sockets[1]!;
   a.send({pose:{sequence:1,xMilli:'1',yMilli:'2',vxMilli:'0',vyMilli:'0'}});b.send({pose:{sequence:1,xMilli:'3',yMilli:'4',vxMilli:'0',vyMilli:'0'}});
   const first=await b.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===a.scope.sessionId&&p.poses?.at(-1)?.sequence===1)??false);
   await a.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===b.scope.sessionId)??false);
   a.send({reauthenticate:{accessToken:state.games[0]!.accessToken!,authRevision:2}});await a.wait(v=>v.welcome?.authRevision===2);
   x.advance(2101);await b.wait(v=>!!v.snapshot&&v.sequence!>first.sequence!&&(v.snapshot.peers?.length??0)===0);
   a.send({pose:{sequence:2,xMilli:'5',yMilli:'6',vxMilli:'0',vyMilli:'0'}});b.send({pose:{sequence:2,xMilli:'7',yMilli:'8',vxMilli:'0',vyMilli:'0'}});await b.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===a.scope.sessionId&&p.poses?.at(-1)?.sequence===2)??false);await a.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===b.scope.sessionId&&p.poses?.at(-1)?.sequence===2)??false);
   assert.equal(a.socket.readyState,1);assert.equal(b.socket.readyState,1);
   const diagnostic=x.fixture.service.live!.debug!;await diagnostic.flush();assert.equal(diagnostic.status.enabled,enabled);
   if(enabled){
    const manifest=JSON.parse(await readFile(join(diagnostic.folder,'manifest.json'),'utf8')),loss=JSON.parse(await readFile(join(diagnostic.folder,'first-loss.json'),'utf8')),events=(await readFile(join(diagnostic.folder,'events.jsonl'),'utf8')).trim().split('\n').map(row=>JSON.parse(row));
    assert.equal(manifest.serviceTag,diagnosticServiceTag(x.fixture.service.db.serviceId,x.fixture.service.db.epoch));assert.equal(loss.trigger.kind,'snapshot_empty');assert.equal(loss.recovery.kind,'recovery');
    assert.ok(loss.before.some((r:any)=>r.kind==='sample_removed'&&r.reason==='pose_ttl'&&r.eligible&&r.sampleAgeMs>=2101&&r.lastAcceptedPoseSequence===1));
    const renewal=events.find((r:any)=>r.kind==='renew_result'&&r.outcome==='accepted');assert.ok(renewal?.operationId);assert.ok(events.some((r:any)=>r.kind==='db_result'&&r.operationId===renewal.operationId&&r.commit==='confirmed'));
    const text=JSON.stringify({manifest,loss,events});for(const secret of [state.games[0]!.accessToken,state.games[1]!.accessToken,x.proof,peer.proof])assert.equal(text.includes(secret!),false);
    console.log(JSON.stringify({serverDebugOutput:relative(process.cwd(),diagnostic.folder).replaceAll('\\','/'),schema:manifest.schema,runtimeId:manifest.runtimeId,serviceTag:manifest.serviceTag,firstLoss:'pose_ttl',recovery:true,renewalActualSqlCorrelated:true,stimulus:'explicit2.1s service-clock movement gap',actualNoita:false}));
   }else{let exists=false;try{exists=(await readdir(output)).length>0;}catch{}assert.equal(exists,false);}
  }finally{for(const socket of sockets)await socket.close();await x.close();}
 }
});
