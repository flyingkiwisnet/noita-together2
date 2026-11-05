import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID,createHash} from 'node:crypto';import {readFile,access} from 'node:fs/promises';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';import {verifyNativeSocialDevelopmentArtifacts,nativeSocialRuntimeResource} from '../src/g3/native-social-profile.js';
import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';import {createG3CandidateApp} from '../src/g3/candidate.js';
// @ts-expect-error Production browser controller has no Node-only type layer.
import {createHostController} from '../../web/candidate/app.js';
// @ts-expect-error Exercise the production browser client over the real service.
import {HostClient,ServiceError} from '../../web/candidate/client.js';
// @ts-expect-error Test DOM adapter; real service remains the sole authority.
import {controllerDom} from '../../web/test/dom-fixture.mjs';

for(const a1Runtime of [false,true])test('actual website controller creates and configures reviewed native-social rooms over verified TLS with all four frozen required capabilities',async()=>{
 const f=await startG3Fixture({nativeSocial:true,tls:true,a1Runtime});let controller:any;
 try{
  const calls:any[]=[],transport=async(path:string,body?:any,token?:string)=>{if(body?.roomCreate||body?.configure)calls.push(structuredClone(body));const r=await f.call(path,body,token);assert.equal(r.status,200);const value=JSON.parse(r.body);if(value.error)throw new ServiceError(value.error.code);return value;};
  const client=new HostClient(transport,randomUUID),dom=await controllerDom('../candidate/index.html');controller=createHostController({...dom,client,schedule:()=>{}});await controller.ready;await dom.click('mock-website');assert.ok(client.credentials);
  await dom.submit('create',{name:'Website native social',seed:'424242',mode:'core',visibility:'public'});assert.equal(dom.get('notice').dataset.error,'false');const roomId=dom.get('invite').value;assert.ok(roomId);
  const created=(await client.rpc({roomId,roomGet:{}})).room;assert.deepEqual(created.config.requiredCapabilities,f.profile.compatibility.capabilities);assert.deepEqual(calls[0].roomCreate.config.requiredCapabilities,f.profile.compatibility.capabilities);
  await dom.submit('configure',{seed:'1234',mode:'core'});assert.equal(dom.get('notice').dataset.error,'false');assert.deepEqual(calls[1].configure.requiredCapabilities,f.profile.compatibility.capabilities);
  const configured=(await client.rpc({roomId,roomGet:{}})).room;assert.equal(configured.config.seed,1234);assert.deepEqual(configured.config.requiredCapabilities,f.profile.compatibility.capabilities);
  await dom.click('prepare');assert.equal(dom.get('notice').dataset.error,'false');const prepared=(await client.rpc({roomId,roomGet:{}})).room;assert.equal(prepared.state,'staging');assert.deepEqual(prepared.config.requiredCapabilities,f.profile.compatibility.capabilities);
 }finally{controller?.dispose();await f.close();}
});
test('explicit frozen native-social verified TLS one-issuer protected inspection/Start profile grants only HUD/POI and keeps source metadata exact through restart',async()=>{
 await assert.rejects(startG3Fixture({nativeSocial:true}),/requires TLS/);for(const selector of ['native','social','hourglass','nemesis'])await assert.rejects(startG3Fixture({nativeSocial:true,tls:true,[selector]:true}));
 let now=Date.now(),target='',reached=false;const reader=async(url:URL)=>{if(target&&url.pathname.endsWith('/'+target)){reached=true;return Buffer.from('changed');}return readFile(url);};const f=await startG3Fixture({nativeSocial:true,tls:true,artifactReader:reader,now:()=>now});const ca=f.caPath;
 try{
  assert.match(f.namespace,/^g3_social_fixture_[0-9a-f]{32}$/);const state=await provisionG3Fixture(f,{beforeRpc:()=>{now+=25;}}),lobby=JSON.parse((await f.call('/v1/info')).body),info=JSON.parse((await f.call('/v3/social/info')).body);
  assert.equal(lobby.serviceId,'nt2-g3-native-social-development');assert.equal(lobby.profile.id,'nt2-g3-native-social-development-1');assert.deepEqual(lobby.compatibility.capabilities,['core.seed','core.ghost','core.hud','core.poi']);assert.deepEqual(info.capabilities,['core.hud','core.poi']);assert.equal(info.installApproved,false);assert.equal(info.sourceApproved,true);assert.equal(info.developmentOnly,true);assert.deepEqual(info.runtimeProfile,f.profile.profile);assert.deepEqual(info.runtimeApproval,nativeSocialRuntimeResource);
  const document=await f.call('/v3/social/runtime');assert.equal(document.status,200);assert.equal(document.headers['cache-control'],'no-store');assert.ok(Buffer.byteLength(document.body)<=32768);assert.equal(createHash('sha256').update(document.body).digest('hex'),nativeSocialRuntimeResource.sha256);assert.equal(JSON.parse(document.body).requiredLocalFiles.length,20);
  const call=async(i:number,payload:object)=>{now+=100;const c=state.games[i]!;const r=await f.call('/v3/social/rpc',{major:1,revision:'g3-social-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope:state.scopes[i],...payload},c.accessToken);assert.equal(r.status,200);return JSON.parse(r.body);};
  const hud={eventId:transientEventId(state.scopes[0]!,state.games[0]!.sessionId!,'hud',1),sequence:1,hud:{hpMilli:3000,maxHpMilli:2000,sampo:true,hpKnown:true,sampoKnown:true}};
  assert.ok((await call(0,{publish:hud})).error);await state.start();assert.ok((await call(0,{publish:hud})).error);await state.playing();assert.equal((await call(0,{publish:hud})).error,undefined);assert.equal((await call(0,{publish:hud})).duplicate,true);
  const snapshot=await call(1,{snapshot:{}});assert.equal(snapshot.projections[0].hud.hpMilli,3000);assert.equal(snapshot.projections[0].principalId,state.desktops[0]!.accountId);assert.equal(snapshot.projections[0].provenance,'validated-client-report');
  const marker={eventId:randomUUID(),sequence:1,poi:{text:'literal <img> 雪',xMilli:'100000',yMilli:'-10000'}};assert.equal((await call(0,{publish:marker})).error,undefined);assert.equal((await call(0,{publish:marker})).duplicate,true);const pois=await call(1,{poiSnapshot:{}});assert.equal(pois.markers[0].poi.text,marker.poi.text);assert.equal(pois.markers[0].displayName,'Alex');
  assert.equal((await call(1,{dismissPoi:{poiSequence:pois.markers[0].poiSequence}})).error,undefined);assert.equal((await call(1,{poiSnapshot:{}})).markers.length,0);assert.equal((await call(0,{poiSnapshot:{}})).markers.length,1);
  for(const [kind,body] of [['emote',{emoteId:'wave',active:true}],['cosmetic',{robeId:'purple',hat:false,amulet:false,gem:false}]] as const){const eventId=transientEventId(state.scopes[0]!,state.games[0]!.sessionId!,kind,1);assert.equal((await call(0,{publish:{eventId,sequence:1,[kind]:body}})).error?.code,'FORBIDDEN');}
  const denied=await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:state.games[0]!.sessionEpoch,requestId:randomUUID(),scope:state.scopes[0],goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}},state.games[0]!.accessToken);assert.equal(JSON.parse(denied.body).error.code,'FORBIDDEN');assert.equal((await f.call('/v3/hourglass/info')).status,404);assert.equal((await f.call('/v3/nemesis/info')).status,404);
  await state.rpc(state.games[0]!,{gameObservation:{...state.observations[0]!,observationRevision:3,protected:true}});assert.ok((await call(0,{snapshot:{}})).error);assert.equal((await call(1,{snapshot:{}})).projections.length,0);
  await state.rpc(state.host,{endRun:{}});assert.ok((await call(1,{poiSnapshot:{}})).error);await f.restart();assert.deepEqual(JSON.parse((await f.call('/v3/social/info')).body).runtimeProfile,info.runtimeProfile);assert.ok((await call(0,{publish:hud})).error);
  target=f.profile.runtimeMappings.find((p:{path:string})=>p.path.endsWith('/init.lua')).sourcePath;await assert.rejects(f.restart(),/changed/);assert.equal(reached,true);
 }finally{await f.close();}
 if(ca)await assert.rejects(access(ca));
});
test('native-social targeted runtime/manifest/native/asset drift refuses before database work; caller profile/compatibility/TLS cannot grant capability',async()=>{
 const metadata=await verifyNativeSocialDevelopmentArtifacts();for(const target of [nativeSocialRuntimeResource.path,metadata.desktopReview.manifest.path,metadata.desktopReview.snapshotRequest.path,...['init.lua','files/nt2_g3_social.dll','files/g3/ui/poi.png','files/g3/social_native.lua'].map(path=>metadata.runtimeMappings.find((p:{path:string})=>p.path==='mods/noita_together2/'+path).sourcePath)]){let reached=false;await assert.rejects(startG3Fixture({nativeSocial:true,tls:true,databaseUrl:'invalid-database-not-reached',artifactReader:async url=>{if(url.pathname.endsWith('/'+target)){reached=true;return Buffer.from('changed');}return readFile(url);}}),/changed/);assert.equal(reached,true,target);}
 const base:any={databaseUrl:'invalid-database-not-reached',namespace:'g3_social_unreached',serviceId:metadata.serviceId,mockEnabled:true,nativeSocial:true,compatibility:metadata.compatibility,keys:{},tls:{pfx:Buffer.alloc(0),passphrase:''}};
 for(const patch of [{profile:metadata.buildRegistry},{namespace:'g3_other'},{tls:undefined},{native:true},{social:true},{serviceId:'nt2-g3-candidate'},{compatibility:{...metadata.compatibility,capabilities:[...metadata.compatibility.capabilities,'core.gold']}}])await assert.rejects(createG3CandidateApp({...base,...patch}),/FORBIDDEN/);
});
