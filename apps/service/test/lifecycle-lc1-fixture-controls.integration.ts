import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID,createHash} from 'node:crypto';
import {readFile} from 'node:fs/promises';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {verifyLifecycleFixtureRuntime} from '../src/rejoin/lifecycle-fixture-profile.js';
import {createG3CandidateApp} from '../src/g3/candidate-loot01.js';
const runtime={path:'spikes/noita-entry/.build/lifecycle-lc1-client-20261018-01/fixture-profile-0d926e368b4649c5848cf21f9d172dd2/runtime.json',sha256:'66c403787368d904ecc4c130c8c3b563273c10998c199de560cb325b2a991933'};
const privateRuntime={path:'spikes/noita-entry/.build/lifecycle-lc1-client-20261018-01/fixture-profile-36b4e43cf6754f5a9e0359c30eafa147/runtime.json',sha256:'679909938267f569adde787ee57ecb6333507adfd038e239a73ccd305545e0d7'};
test('LC1 pinned fixture runtime verifies current closure and refuses wrong pin/non-build input/unready private declaration',async()=>{
 const r=await verifyLifecycleFixtureRuntime(runtime);assert.equal(r.runtimeMappings.length,50);assert.deepEqual(r.runtimeApproval,runtime);
 await assert.rejects(verifyLifecycleFixtureRuntime({...runtime,sha256:'0'.repeat(64)}),/changed/);
 await assert.rejects(verifyLifecycleFixtureRuntime({path:'deploy/profiles/g3-se1-runtime.20261018.json',sha256:'28e855ead443bbfcf98b474f0db8ffe61bb11268c92f17307ece3bcf22899409'}),/private uninstalled/);
 await assert.rejects(verifyLifecycleFixtureRuntime({path:'spikes/noita-entry/.build/lifecycle-lc1-client-20261018-01/fixture-profile-0a84a0458d4e4c3aa173c549282872a8/runtime.json',sha256:'f8c591043b9d544cc6f577a8f29a1470369555491684e6b56401543898e4e1a3'}));
});
test('LC1 corrected private fixture profile matches normal endpoints and cannot select installed namespace',async()=>{
 const metadata=await verifyLifecycleFixtureRuntime(privateRuntime);assert.equal(metadata.fixtureNative.private,true);assert.ok(metadata.nativeArtifacts.filter((a:any)=>a.mode!=='broker').every((a:any)=>a.exports.includes('nt2_g3_private_api')));
 await assert.rejects(createG3CandidateApp({rejoin:true,rejoinFixtureRuntime:privateRuntime,namespace:'g3_rejoin_lan_192_168_0_104_4358',databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1'} as any),/FORBIDDEN/);
 const x=await startLifecycleLc1Fixture({runtime:privateRuntime});try{
  const social=JSON.parse((await x.fixture.call('/v3/social/info')).body),info=JSON.parse((await x.fixture.call('/v1/info')).body),body=(await x.fixture.call('/v3/social/runtime')).body;
  assert.equal(social.sourceApproved,true);assert.deepEqual(social.runtimeApproval,privateRuntime);assert.deepEqual(social.runtimeProfile,metadata.profile);assert.deepEqual(info.profile,metadata.buildRegistry);assert.equal(createHash('sha256').update(body).digest('hex'),privateRuntime.sha256);
  assert.equal((await x.send(x.desktop,x.request())).world.state,'waiting');
 }finally{await x.close();}
});
test('LC1 controls bootstrap real peer, select/exclude inspected roster, ban either fixture account, End and configure next run',async()=>{
 const x=await startLifecycleLc1Fixture({runtime});try{
  const info=JSON.parse((await x.fixture.call('/v1/info')).body),social=JSON.parse((await x.fixture.call('/v3/social/info')).body),resource=await x.fixture.call('/v3/social/runtime');
  assert.equal(info.profile.id,x.fixture.profile.id);assert.equal(social.sourceApproved,true);assert.deepEqual(social.runtimeApproval,runtime);assert.deepEqual(social.runtimeProfile,x.fixture.profile.profile);assert.equal(createHash('sha256').update(resource.body).digest('hex'),runtime.sha256);
  assert.equal((await x.send(x.desktop,x.request())).world.state,'waiting');
  const peer=await x.bootstrapPeer();assert.equal((await x.bootstrapPeer()).desktop.sessionId,peer.desktop.sessionId);assert.notEqual(peer.desktop.accountId,x.desktop.accountId);assert.notEqual(x.host.accountId,x.desktop.accountId);assert.notEqual(x.host.accountId,peer.desktop.accountId);
  const peerRequest=x.request(peer.desktop,1,'peer-context',peer.observation);peerRequest.reconcile!.association=peer.association;peerRequest.reconcile!.proof=peer.proof;
  assert.equal((await x.send(peer.desktop,peerRequest)).world.state,'waiting');
  await assert.rejects(x.nextRun(),/FIXTURE_END_REQUIRED/);
  await assert.rejects(x.start({selectedWorldIds:[randomUUID()],excludedDesktopIds:[]}),/FIXTURE_ROSTER/);
  await x.start({selectedWorldIds:[peer.association.worldId!],excludedDesktopIds:[x.desktop.sessionId!]});
  const selected=await x.send(peer.desktop,{...peerRequest,requestId:randomUUID()});assert.equal(selected.world.mayPlay,true);
  const current=await x.state();assert.equal(current.participants.length,1);assert.equal(current.participants[0].worldId,peer.association.worldId);assert.equal(current.participants[0].participantId,selected.world.participantId);
  assert.equal((await x.send(x.desktop,x.request())).world.reason,'not_selected_by_start');
  await x.moderate('peer','ban');assert.equal((await x.send(peer.desktop,{...peerRequest,requestId:randomUUID()})).world.reason,'host_ban');
  await assert.rejects(x.moderate('other' as 'peer','ban'),/FIXTURE_ACCOUNT/);
  const oldRun=x.room.runId;await x.end();assert.equal(x.room.state,'ended');
  await x.nextRun({...x.room.config,seed:424243});assert.equal(x.room.state,'staging');assert.notEqual(x.room.runId,oldRun);assert.equal((await x.state(oldRun)).participants[0].terminal,true);await assert.rejects(x.state(randomUUID()),/FIXTURE_RUN/);
  await x.rpc(x.desktop,{roomJoin:{}});const fresh=await x.prepareWorld();const freshRequest=x.request(x.desktop,1,'next-run',{...x.observation,actualSeed:'424243',worldToken:fresh.association.worldId!,gameInstanceId:'next-run'});freshRequest.reconcile!.association=fresh.association;freshRequest.reconcile!.proof=fresh.proof;
  assert.equal((await x.send(x.desktop,freshRequest)).world.state,'waiting');assert.equal((await x.send(x.desktop,x.request())).world.mayPlay,false);
  await x.moderate('consumer','ban');assert.equal((await x.send(x.desktop,{...freshRequest,requestId:randomUUID()})).world.reason,'host_ban');await x.moderate('consumer','unban');
  // Inputs and tests never refresh either Client's credentials. Host controls
  // may renew their own browser session after a controlled clock expiry.
  x.advance(920000);await x.end();assert.equal(x.room.state,'ended');
 }finally{await x.close();}
});
