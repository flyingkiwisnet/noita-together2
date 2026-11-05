import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {request} from 'node:https';
import {checkServerIdentity} from 'node:tls';
import {access,readFile} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import type {Request,Response} from '../../../packages/contracts/g3/g3.js';
import {verifyNativeDevelopmentArtifacts} from '../src/g3/native-profile.js';
import {verifyNativeSocialDevelopmentArtifacts} from '../src/g3/native-social-profile.js';

test('frozen native HTTPS baseline releases ghosts, withholds gold/social and refuses targeted drift before startup/restart',async()=>{
 await assert.rejects(startG3Fixture({native:true}),/requires TLS/);
 await assert.rejects(startG3Fixture({native:true,tls:true,social:true}),/cannot grant social/);
 const metadata=await verifyNativeDevelopmentArtifacts();let target='';let reached=false;
 const reader=async(url:URL)=>{if(target&&url.pathname.endsWith('/'+target)){reached=true;return Buffer.from('changed');}return readFile(url);};
 const f=await startG3Fixture({native:true,tls:true,artifactReader:reader});try{
  const info=JSON.parse((await f.call('/v1/info')).body);assert.equal(info.profile.id,'nt2-g3-native-development-2');assert.equal(info.profile.evidenceStatus,'candidate-engine-evidence-pending');
  assert.deepEqual(info.capabilities,['core.seed','core.ghost']);assert.equal((await f.call('/v3/social/info')).status,404);
  const state=await provisionG3Fixture(f),game=state.games[0]!;
  const call=async(payload:Partial<Request>)=>JSON.parse((await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:game.sessionEpoch,requestId:randomUUID(),scope:state.scopes[0],...payload},game.accessToken)).body) as Response;
  assert.ok((await call({ghostList:{}})).error);await state.start();assert.ok((await call({ghostList:{}})).error);await state.playing();
  assert.equal((await call({movement:{frames:[{sequence:1,xMilli:'1',yMilli:'2',vxMilli:'0',vyMilli:'0',facingLeft:false}]}})).error,undefined);
  const peer=state.games[1]!;const ghosts=JSON.parse((await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:peer.sessionEpoch,requestId:randomUUID(),scope:state.scopes[1],ghostList:{}},peer.accessToken)).body);assert.equal(ghosts.ghosts[0].gameSessionId,game.sessionId);
  assert.equal((await call({goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}})).error?.code,'FORBIDDEN');
  await f.restart();assert.ok((await call({ghostList:{}})).error);
  for(const suffix of ['g3_candidate.exe','nt2_g3_transport.dll','init-g3.lua','inspection.lua','frame.cpp','review-request.json']){
   target=metadata.provenance.pins.find((p:{path:string})=>p.path.endsWith('/'+suffix))?.path??metadata.provenance.snapshotRequest.path;reached=false;
   await assert.rejects(startG3Fixture({native:true,tls:true,artifactReader:reader,databaseUrl:'invalid-database-not-reached'}),/changed/);assert.equal(reached,true,suffix);
  }
  target=metadata.provenance.snapshotRequest.path;reached=false;await assert.rejects(f.restart(),/changed/);assert.equal(reached,true);
 }finally{await f.close();}
});

for(const tls of [false,true])test(`disposable G3 ${tls?'verified HTTPS':'HTTP'} fixture gates gameplay and preserves original recovery`,async()=>{
 const f=await startG3Fixture({tls});const caPath=f.caPath;
 try{
  const state=await provisionG3Fixture(f),game=state.games[0]!,scope=state.scopes[0]!;
  const call=async(body:Partial<Request>,credentials=game)=>{
   const response=await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:credentials.sessionEpoch,requestId:randomUUID(),scope,...body},credentials.accessToken);
   assert.equal(response.status,200,response.body);return JSON.parse(response.body) as Response;
  };
  const pose={movement:{frames:[{sequence:1,xMilli:'1',yMilli:'2',vxMilli:'0',vyMilli:'0',facingLeft:false}]}};
  assert.ok((await call(pose)).error);await state.pulse();await state.start();assert.ok((await call(pose)).error);await state.playing();await state.pulse();
  assert.equal((await call(pose)).error,undefined);
  const peer=state.games[1]!,peerResponse=await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:peer.sessionEpoch,requestId:randomUUID(),scope:state.scopes[1],ghostList:{}},peer.accessToken);
  const ghosts=(JSON.parse(peerResponse.body) as Response).ghosts!;assert.equal(ghosts[0]!.principalId,state.desktops[0]!.accountId);assert.equal(ghosts[0]!.gameSessionId,game.sessionId);assert.equal(ghosts[0]!.worldToken,scope.worldToken);
  const deposit=(await call({goldIntent:{operationId:randomUUID(),direction:'deposit',amount:10}})).operation!;
  const proof={goldReceipt:{operationId:deposit.operationId!,deliveryId:deposit.deliveryId!,payloadDigest:deposit.payloadDigest!,receiptId:randomUUID(),outcome:'escrowed',evidenceDigest:'a'.repeat(64)}};
  assert.equal((await call(proof)).operation!.balance,10);
  assert.equal((await f.call('/v1/info')).status,200);
  assert.equal((await f.call('/v3/info')).status,200);
  assert.equal((await f.call('/v3/info',undefined,undefined,{origin:'https://foreign.example'})).status,403);
  const oversized=await f.call('/v3/rpc','x'.repeat(20000));assert.equal(oversized.status,400);
  await assert.rejects(f.call('/v3/rpc','x'.repeat(65537)),/too large/);
  if(tls){
   const tlsGet=(trust:boolean,wrongIdentity=false)=>new Promise<void>((resolve,reject)=>{
    const req=request(f.origin+'/v3/info',{agent:false,...(trust?{ca:f.ca!}:{}),checkServerIdentity:(host,cert)=>checkServerIdentity(wrongIdentity?'127.0.0.2':host,cert)},res=>{res.resume();res.once('end',resolve);});req.once('error',reject);req.end();
   });
   await tlsGet(true);await assert.rejects(tlsGet(false));await assert.rejects(tlsGet(true,true),{code:'ERR_TLS_CERT_ALTNAME_INVALID'});
   assert.equal((await f.call('/v3/info',undefined,undefined,{host:'foreign.example'})).status,403);
  }
  await state.rpc(state.host,{endRun:{}});assert.equal((await call({goldStatus:{operationId:deposit.operationId!}},state.desktops[0]!)).operation!.balance,10);
  await f.restart();assert.equal((await call({goldStatus:{operationId:deposit.operationId!}},state.desktops[0]!)).operation!.balance,10);
  assert.ok((await call(pose)).error);
 }finally{await f.close();await f.close();}
 if(caPath)await assert.rejects(access(caPath));
});

for(const {social,native,hourglass,nemesis,nativeSocial} of [{social:false,native:false,hourglass:false},{social:true,native:false,hourglass:false},{social:false,native:true,hourglass:false},{social:false,native:false,hourglass:true},{social:false,native:false,hourglass:false,nemesis:true},{social:false,native:false,hourglass:false,nativeSocial:true}])test(`client ${nativeSocial?'native-social source':native?'native development (refuses artifact drift)':nemesis?'nemesis':hourglass?'hourglass':social?'social':'base'} fixture launcher bounds private bundle startup and quit cleanup`,{timeout:30000},async()=>{
 if(native)await verifyNativeDevelopmentArtifacts();
 if(nativeSocial)await verifyNativeSocialDevelopmentArtifacts();
 const root=new URL('../../../',import.meta.url),child=spawn(process.execPath,[fileURLToPath(new URL('node_modules/tsx/dist/cli.mjs',root)),fileURLToPath(new URL('apps/service/scripts/g3-client-fixture.ts',root)),'--tls','--playing',...(social?['--social']:[]),...(hourglass?['--hourglass']:[]),...(nemesis?['--nemesis']:[]),...(native?['--native']:[]),...(nativeSocial?['--native-social']:[])],{cwd:fileURLToPath(root),stdio:['pipe','pipe','pipe']});
 let output='',errors='';child.stderr.on('data',chunk=>{errors+=chunk;});
 const ended=new Promise<number|null>((resolve,reject)=>{child.once('error',reject);child.once('close',resolve);});
 try{
  const ready=await new Promise<{bundlePath:string;caPath:string;origin:string;state:string}>((resolve,reject)=>{
   child.stdout.on('data',chunk=>{output+=chunk;const line=output.split('\n')[0];if(line?.startsWith('{')){try{resolve(JSON.parse(line));}catch{/* incomplete line */}}});
   child.once('close',()=>reject(Error('Fixture exited before ready: '+errors)));
  });
  assert.equal(ready.state,'simulated-playing');assert.match(ready.origin,/^https:\/\/127\.0\.0\.1:[0-9]+$/);
  const bundle=JSON.parse(await readFile(ready.bundlePath,'utf8'));assert.equal(bundle.evidenceStatus,'simulated-game-facts');assert.equal(bundle.games.length,2);assert.equal(bundle.scopes.length,2);
  if(social||nativeSocial){assert.equal(bundle.social.revision,'g3-social-draft-1');assert.match(bundle.social.schemaSha256,/^[a-f0-9]{64}$/);assert.deepEqual(bundle.social.capabilities,nativeSocial?['core.hud','core.poi']:['core.hud','core.emotes','core.cosmetics','core.poi']);}else assert.equal(bundle.social,undefined);
  if(nativeSocial){assert.equal(bundle.nativeSocialDevelopmentProfile.id,'nt2-g3-native-social-development-1');assert.equal(bundle.nativeSocialDevelopmentProfile.requiredLocalFiles.length,20);assert.equal(bundle.nativeSocialDevelopmentProfile.installApproved,false);assert.deepEqual(bundle.nativeSocialDevelopmentProfile.profile,bundle.social.runtimeProfile);assert.equal(bundle.nativeDevelopmentProfile,undefined);assert.match(bundle.namespace,/^g3_social_fixture_[0-9a-f]{32}$/);}
  if(native){assert.equal(bundle.nativeDevelopmentProfile.id,'nt2-g3-native-development-2');assert.deepEqual(bundle.nativeDevelopmentProfile.compatibility.capabilities,['core.seed','core.ghost']);assert.equal(bundle.nativeDevelopmentProfile.installApproved,false);}
  if(hourglass){assert.equal(bundle.hourglass.revision,'g3-hourglass-draft-2');assert.equal(bundle.hourglass.catalogueRevision,'nt2-hourglass-builtins-1');assert.deepEqual(bundle.hourglass.capabilities,['core.hourglass']);}
  if(nemesis){assert.equal(bundle.nemesis.revision,'g3-nemesis-draft-1');assert.equal(bundle.nemesis.catalogueRevision,'nt2-nemesis-builtins-2');assert.equal(bundle.nemesis.earningEnabled,false);assert.equal(bundle.nemesis.offeringEnabled,false);assert.deepEqual(bundle.nemesis.capabilities,['nemesis.abilities']);}
  assert.ok(bundle.games[0].accessToken);assert.ok(!output.includes(bundle.games[0].accessToken));
  child.stdin.end('quit\n');assert.equal(await ended,0,errors);
  await assert.rejects(access(ready.bundlePath));await assert.rejects(access(ready.caPath));
 }finally{if(child.exitCode===null){child.stdin.end('quit\n');await ended;}}
});
