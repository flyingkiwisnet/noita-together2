import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID,createHash} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';import {livePeer} from '../scripts/g3-live-fixture.js';
import {nativeC2RuntimeResource,nativeC2SourceResource} from '../src/g3/native-c2-profile.js';
import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';
const {requiredRoomCapabilities}=await import(new URL('../../web/candidate/app.js',import.meta.url).href);

test('matched C2 TLS metadata/defaults and first selected peers agree over HTTP, social and WSS; reduced/protected scope refuses',async()=>{
 for(const seedOnly of [false,true]){
  let now=Date.now();const f=await startG3Fixture({c2Runtime:true,live:true,tls:true,now:()=>now});const peers:Awaited<ReturnType<typeof livePeer>>[]=[];
  try{
   assert.match(f.namespace,/^g3_c2_fixture_[0-9a-f]{32}$/);
   const lobby=JSON.parse((await f.call('/v1/info')).body),social=JSON.parse((await f.call('/v3/social/info')).body),live=JSON.parse((await f.call('/v3/live/info')).body);
   assert.equal(lobby.profile.id,'nt2-g3-native-c2-development-2');assert.equal(lobby.compatibility.modVersion,'nt2-g3-c2-dev-1');assert.equal(lobby.compatibility.bridgeVersion,'g3-c2-ipc-1');
   const caps=requiredRoomCapabilities(lobby);assert.deepEqual(caps,['core.seed','core.ghost','core.hud','core.poi']);
   assert.deepEqual(social.runtimeApproval,nativeC2RuntimeResource);assert.deepEqual(social.desktopApproval,nativeC2SourceResource);assert.deepEqual(social.runtimeProfile,f.profile.profile);assert.equal(social.installApproved,false);
   assert.equal(live.endpoint,'/v3/live');assert.equal(live.subprotocol,'nt2.g3.live.v1');
   const document=await f.call('/v3/social/runtime');assert.equal(document.headers['cache-control'],'no-store');assert.equal(createHash('sha256').update(document.body).digest('hex'),nativeC2RuntimeResource.sha256);assert.equal(JSON.parse(document.body).requiredLocalFiles.length,22);
   const s=await provisionG3Fixture(f,{playerCount:3,requiredCapabilities:seedOnly?['core.seed']:caps,beforeRpc:()=>{now+=100;}});
   const call=async(i:number,payload:object,scope=s.scopes[i]!,social=false)=>{
    now+=100;const c=s.games[i]!,r=await f.call(social?'/v3/social/rpc':'/v3/rpc',{major:1,revision:social?'g3-social-draft-1':'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope,...payload},c.accessToken);assert.equal(r.status,200);return JSON.parse(r.body);
   };
   const movement=(sequence:number)=>({movement:{frames:[{sequence,xMilli:'1000',yMilli:'2000',vxMilli:'0',vyMilli:'0',facingLeft:false}]}});
   await s.rpc(s.host,{finalStart:{participants:s.reports.filter(r=>r.desktopId!==s.desktops[2]!.sessionId).map(r=>({desktopId:r.desktopId!,attachmentId:r.attachmentId!,worldToken:r.worldToken!,observationRevision:r.observationRevision!,contentDigest:r.contentDigest!})),excludedDesktopIds:[s.desktops[2]!.sessionId!]}});
   assert.equal((await call(0,movement(1))).error.code,'STALE_EPOCH');await assert.rejects(livePeer(f,s,0));
   for(let i=0;i<2;i++)await s.rpc(s.games[i]!,{gameObservation:{...s.observations[i]!,observationRevision:2,protected:false}});
   assert.equal((await call(2,movement(1))).error.code,'STALE_EPOCH');await assert.rejects(livePeer(f,s,2));
   if(seedOnly){assert.equal((await call(0,movement(1))).error.code,'FORBIDDEN');await assert.rejects(livePeer(f,s,0));}
   else{
    for(let i=0;i<2;i++)assert.equal((await call(i,movement(1))).error,undefined);
    for(let i=0;i<2;i++)assert.equal((await call(i,{ghostList:{}})).ghosts.length,1);
    peers.push(await livePeer(f,s,0),await livePeer(f,s,1));const [a,b]=peers;
    a!.send({pose:{sequence:1,xMilli:'3333',yMilli:'4444',vxMilli:'0',vyMilli:'0'}});
    const snap=await b!.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===a!.scope.sessionId)??false);assert.equal(snap.snapshot!.peers!.find(p=>p.sessionId===a!.scope.sessionId)!.poses![0]!.xMilli,'3333');
    b!.send({pose:{sequence:1,xMilli:'5555',yMilli:'6666',vxMilli:'0',vyMilli:'0'}});await a!.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===b!.scope.sessionId)??false);
    const hud={eventId:transientEventId(s.scopes[0]!,s.games[0]!.sessionId!,'hud',1),sequence:1,hud:{hpMilli:1000,maxHpMilli:2000,sampo:false}};
    assert.equal((await call(0,{publish:hud},s.scopes[0]!,true)).error,undefined);assert.equal((await call(1,{snapshot:{}},s.scopes[1]!,true)).projections[0].hud.hpMilli,1000);
    assert.equal((await call(0,movement(2),{...s.scopes[0]!,worldToken:'wrong-world'})).error.code,'STALE_EPOCH');
    await s.rpc(s.games[0]!,{gameObservation:{...s.observations[0]!,observationRevision:3,protected:true}});await a!.wait(v=>!!v.retire);assert.equal((await call(1,{ghostList:{}})).ghosts.length,0);
    await s.rpc(s.host,{endRun:{}});await b!.wait(v=>!!v.retire);assert.equal(f.service.live!.stats.samples,0);
   }
  }finally{for(const p of peers)await p.close();await f.close();}
 }
});
