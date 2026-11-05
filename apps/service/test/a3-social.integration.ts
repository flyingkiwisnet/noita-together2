// Real isolated service/TLS/SQL. Engine observations are injected, never Noita proof.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import pg from 'pg';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {schema,encode,decode} from '../../../packages/contracts/src/g3-social.js';
import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';
import {SocialAuthority} from '../src/g3/social.js';
import {readRoomAdmissions} from '../src/g3/admission.js';
import {EMOTES,ROBES} from '../../../packages/domain/src/g3-social.js';
test('A3 current state: looping/finite age, originals, reliable stop, retained per-actor cosmetics and canonical lifecycle',async()=>{
 const x=await startLifecycleLc1Fixture();try{
  const peer=await x.bootstrapPeer(),qa=x.request(),qb=x.request(peer.desktop,1,'a3-peer',peer.observation);qb.reconcile!.association=peer.association;qb.reconcile!.proof=peer.proof;
  await x.send(x.desktop,qa);await x.send(peer.desktop,qb);await x.start();qa.reconcile!.observation={...x.observation,observationRevision:2,protected:false};qb.reconcile!.observation={...peer.observation,observationRevision:2,protected:false};
  const values=[await x.send(x.desktop,qa),await x.send(peer.desktop,qb)],db=x.fixture.service.db;
  assert.ok(values.every(v=>v.canonicalAdmission.decision.playing));
  assert.deepEqual((await db.transaction(tx=>tx.query('SELECT current_database() AS db,current_user AS owner,current_schema() AS schema'))).rows[0],{db:'nt2_g1',owner:'nt2_g1',schema:x.fixture.namespace});
  const scope=(i:number)=>Object.fromEntries(['roomId','runId','runEpoch','configRevision','attachmentId','worldToken'].map(k=>[k,values[i].canonicalAdmission.scope[k]]));
  const call=async(i:number,payload:any,step=110)=>{x.advance(step);const v=values[i];return JSON.parse((await x.fixture.call('/v3/social/rpc',{major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:v.credentials.sessionEpoch,requestId:randomUUID(),scope:scope(i),...payload},v.credentials.accessToken)).body);};
  const publication=(i:number,kind:'emote'|'cosmetic',sequence:number,body:any)=>({publish:{eventId:transientEventId(scope(i),values[i].credentials.sessionId,kind,sequence),sequence,[kind]:body}});
  const ok=async(p:Promise<any>)=>{const r=await p;assert.equal(r.error,undefined,JSON.stringify(r.error));return r;};
  const snapshot=async()=>{const r=await ok(call(1,{snapshot:{}}));for(const p of r.projections??[])decode('Projection',encode('Projection',p));return r.projections??[];};
  const state=async(i:number,kind:string)=>(await snapshot()).find((p:any)=>p.principalId===values[i].canonicalAdmission.scope.principalId&&p[kind]);
  const wave=publication(0,'emote',1,{emoteId:'wave',active:true});await ok(call(0,wave));
  await ok(call(0,publication(0,'cosmetic',1,{robeId:'red',hat:true,amulet:false,gem:true})));
  await ok(call(1,publication(1,'cosmetic',1,{robeId:'blue',hat:false,amulet:true,gem:false})));
  // Same real canonical authority, counted at its boundary rather than mocked:
  // each snapshot/roster reads it once and never reuses a prior transaction.
  const counted=new SocialAuthority(()=>db.epoch,x.fixture.profile.compatibility.capabilities,x.fixture.service.auth.now,true);let admissionReads=0;counted.admission=async(tx,id)=>{admissionReads++;return readRoomAdmissions(tx,db,x.fixture.profile.compatibility,x.fixture.service.auth.now(),id);};
  for(const operation of [{snapshot:{}},{roster:{}}]){admissionReads=0;await db.transaction(async tx=>{const actor=await x.fixture.service.auth.authenticate(tx,values[1].credentials.accessToken);const result=await counted.execute(tx,actor,{scope:scope(1),...operation});assert.ok(result.projections?.length||result.players?.length);});assert.equal(admissionReads,1,'one fresh canonical view per transaction');}
  for(const gap of [15000,18000]){x.advance(gap);values[0]=await x.send(x.desktop,{...qa,requestId:randomUUID()});values[1]=await x.send(peer.desktop,{...qb,requestId:randomUUID()});}
  let p=await state(0,'emote');assert.equal(p.emote.active,true);assert.ok(p.emote.ageMs>33000);assert.equal(p.expiresAt,undefined);const rev=p.presentationRevision,age=p.emote.ageMs;
  assert.equal((await ok(call(0,wave))).duplicate,true);p=await state(0,'emote');assert.equal(p.presentationRevision,rev);assert.ok(p.emote.ageMs>=age);
  assert.equal((await state(0,'cosmetic')).cosmetic.robeId,'red');assert.equal((await state(1,'cosmetic')).cosmetic.robeId,'blue');
  await ok(call(0,publication(0,'emote',2,{emoteId:'heart',active:true})));p=await state(0,'emote');assert.equal(p.emote.active,true);const heartRev=p.presentationRevision;x.advance(2000);p=await state(0,'emote');assert.equal(p.emote.active,false);assert.equal(p.presentationRevision,heartRev);
  await ok(call(0,publication(0,'emote',3,{emoteId:'wave',active:true})));const firstRepeat=await state(0,'emote');await ok(call(0,publication(0,'emote',4,{emoteId:'wave',active:true})));p=await state(0,'emote');assert.ok(p.presentationRevision>firstRepeat.presentationRevision);assert.notEqual(p.eventId,firstRepeat.eventId);
  x.advance(1100);for(let sequence=5;sequence<=8;sequence++)await ok(call(0,publication(0,'emote',sequence,{emoteId:'clap',active:true}),0));
  assert.equal((await call(0,publication(0,'emote',9,{emoteId:'wave',active:true}),0)).error.code,'CAPACITY');
  const stop=publication(0,'emote',9,{emoteId:'wave',active:false});await ok(call(0,stop,0)); // Deliberately discard the successful reply.
  assert.equal((await ok(call(0,stop,0))).duplicate,true);assert.equal((await state(0,'emote')).emote.active,false);
  assert.equal((await ok(call(0,wave))).duplicate,true);assert.equal((await state(0,'emote')).emote.active,false,'late known original cannot overwrite stop');
  assert.equal((await call(0,publication(0,'emote',10,{emoteId:'clap',active:false}),0)).error.code,'CAPACITY','redundant originals remain bounded');
  // Each catalogue ID traverses actual authentication/SQL; six colours and all flags too.
  let seq=10;for(const e of EMOTES){if(seq%5===0){values[0]=await x.send(x.desktop,{...qa,requestId:randomUUID()});values[1]=await x.send(peer.desktop,{...qb,requestId:randomUUID()});}x.advance(1100);await ok(call(0,publication(0,'emote',seq++,{emoteId:e.id,active:true}),0));assert.equal((await state(0,'emote')).emote.emoteId,e.id);}
  let cosmeticSeq=2;for(const robeId of ROBES){x.advance(1100);await ok(call(0,publication(0,'cosmetic',cosmeticSeq++,{robeId,hat:true,amulet:true,gem:true}),0));assert.deepEqual((await state(0,'cosmetic')).cosmetic,{robeId,hat:true,amulet:true,gem:true});}
  // Maintain the peer leases after the catalogue loop; no social action renews them.
  values[0]=await x.send(x.desktop,{...qa,requestId:randomUUID()});values[1]=await x.send(peer.desktop,{...qb,requestId:randomUUID()});
  assert.equal((await call(0,{scope:{...scope(0),worldToken:'wrong-world'},snapshot:{}})).error.code,'STALE_EPOCH');
  await ok(call(1,publication(1,'emote',1,{emoteId:'wave',active:true})));
  const waiting={...qa,requestId:randomUUID(),reconcile:{...qa.reconcile!,observation:{...qa.reconcile!.observation!,protected:true,observationRevision:3}}};values[0]=await x.send(x.desktop,waiting);
  assert.equal(await state(0,'emote'),undefined);assert.equal((await state(1,'emote')).emote.active,true);assert.equal((await state(0,'cosmetic')).cosmetic.robeId,'purple');
  for(const gap of [15000,16000]){x.advance(gap);values[1]=await x.send(peer.desktop,{...qb,requestId:randomUUID()});}
  assert.equal(await state(0,'cosmetic'),undefined,'disconnected cosmetic state remains private');assert.equal((await state(1,'emote')).emote.active,true);
  const old=values[0],resumed={...qa,requestId:randomUUID(),reconcile:{...qa.reconcile!,attempt:2,connectionId:'a3-return',observation:{...x.observation,gameInstanceId:'a3-return',protected:true,observationRevision:1}}};values[0]=await x.send(x.desktop,resumed);assert.equal(values[0].error,undefined,JSON.stringify(values[0]));assert.ok(values[0].credentials,JSON.stringify(values[0]));assert.notEqual(values[0].credentials.sessionId,old.credentials?.sessionId);
  p=await state(0,'cosmetic');assert.equal(p.gameSessionId,values[0].credentials.sessionId);assert.equal(p.scope.attachmentId,scope(0).attachmentId);assert.equal(await state(0,'emote'),undefined);
  resumed.requestId=randomUUID();resumed.reconcile!.observation={...resumed.reconcile!.observation!,protected:false,observationRevision:2};values[0]=await x.send(x.desktop,resumed);assert.equal(await state(0,'emote'),undefined,'Continue never resumes pre-save animation');await ok(call(0,publication(0,'emote',1,{emoteId:'wave',active:true})));const cold=(await ok(call(0,{snapshot:{}}))).projections;assert.ok(cold.some((v:any)=>v.cosmetic?.robeId==='blue'),'new reader session receives current state without original recipient-list membership');
  // Explicitly injected revision-edge fixture: one final stop remains possible.
  await db.transaction(tx=>tx.query("UPDATE g3_social_presentation SET revision=4294967294 WHERE run_id=$1 AND world_token=$2 AND kind='emote'",[scope(0).runId,scope(0).worldToken]));assert.equal((await call(0,publication(0,'emote',2,{emoteId:'clap',active:true}))).error.code,'CAPACITY');await ok(call(0,publication(0,'emote',2,{emoteId:'wave',active:false})));assert.equal((await state(0,'emote')).presentationRevision,0xffffffff);
  await x.moderate('consumer','kick');assert.equal(await state(0,'emote'),undefined);assert.equal(await state(0,'cosmetic'),undefined);assert.equal((await state(1,'emote')).emote.active,true);
  await x.end();await db.transaction(tx=>new SocialAuthority(()=>db.epoch,[],x.fixture.service.auth.now,true).maintain(tx));assert.equal((await db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_social_presentation'))).rows[0].n,0);
 }finally{const namespace=x.fixture.namespace;await x.close();const c=new pg.Client({connectionString:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1'});try{await c.connect();assert.equal((await c.query('SELECT 1 FROM pg_namespace WHERE nspname=$1',[namespace])).rowCount,0);console.log(JSON.stringify({a3OwnedSchemaAbsent:namespace}));}finally{await c.end();}}
});
