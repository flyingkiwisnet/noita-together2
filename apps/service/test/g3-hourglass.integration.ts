import {request as httpsRequest} from 'node:https';import {checkServerIdentity} from 'node:tls';
import {readFile} from 'node:fs/promises';
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {HOURGLASS_CATALOGUE,HourglassLedger} from '../../../packages/domain/src/g3-hourglass.js';
import {loadHourglass} from '../src/g3/hourglass-storage.js';
import {encode,decode} from '../../../packages/contracts/src/g3-hourglass-v2.js';
import type {Request,Response,Activate} from '../../../packages/contracts/g3-hourglass-v2/g3-hourglass-v2.js';
async function fixture(mode='core',requiredCapabilities?:string[],roster:Parameters<typeof provisionG3Fixture>[1]={}){
 let now=Date.now();const f=await startG3Fixture({hourglass:true,tls:true,now:()=>now});
 try{
 const state=await provisionG3Fixture(f,{mode,...(requiredCapabilities?{requiredCapabilities}:{}),...roster,beforeRpc:()=>{now+=(roster.playerCount??2)>2?100:25;}});
 const input=(i:number,payload:Partial<Request>,desktop=false):Request=>({major:1,revision:'g3-hourglass-draft-2',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:(desktop?state.desktops[i]:state.games[i])!.sessionEpoch!,requestId:randomUUID(),catalogueRevision:HOURGLASS_CATALOGUE.revision,scope:state.scopes[i]!,...payload});
 const call=async(i:number,payload:Partial<Request>,options:{desktop?:boolean;binary?:boolean;pulse?:boolean}={})=>{
  now+=1100;if(options.pulse!==false)await state.pulse();const body=input(i,payload,options.desktop),token=(options.desktop?state.desktops[i]:state.games[i])!.accessToken;
  if(options.binary){const result=await new Promise<Buffer>((resolve,reject)=>{const req=httpsRequest(f.origin+'/v3/hourglass/rpc',{method:'POST',ca:f.ca,servername:'',checkServerIdentity:(_host,cert)=>checkServerIdentity('127.0.0.1',cert),headers:{authorization:'Bearer '+token,'content-type':'application/x-protobuf'}},res=>{assert.equal(res.statusCode,200);const chunks:Buffer[]=[];res.on('data',b=>chunks.push(b));res.on('end',()=>resolve(Buffer.concat(chunks)));res.on('error',reject);});req.on('error',reject);req.end(Buffer.from(encode('Request',body)));});return decode('Response',result) as Response;}
  return JSON.parse((await f.call('/v3/hourglass/rpc',body,token)).body) as Response;
 };
 return {f,state,call,input,advance:(ms:number)=>{now+=ms;},ok:async(p:Promise<Response>)=>{const result=await p;assert.equal(result.error,undefined,result.error?.code);return result;}};
 }catch(e){await f.close();throw e;}
}
const activation=(index=0):Required<Activate>=>({activationId:randomUUID(),effectId:HOURGLASS_CATALOGUE.entries[index]!.id,materialId:HOURGLASS_CATALOGUE.entries[index]!.materialId});
test('one issuer selected Start/playing gates all seven durable hourglass effects, JSON/binary, private attributed deliveries and receipts',async()=>{
 const t=await fixture();try{
  const p=activation();assert.ok((await t.call(0,{activate:p})).error);await t.state.start();assert.ok((await t.call(0,{activate:p})).error);await t.state.playing();
  for(let i=0;i<7;i++){const effect=i===0?p:activation(i);const accepted=await t.ok(t.call(0,{activate:effect},{binary:i===1}));assert.equal(accepted.activation!.recipientCount,1);assert.deepEqual((await t.ok(t.call(0,{activate:effect}))).activation,accepted.activation);}
  assert.equal((await t.ok(t.call(0,{pending:{}}))).deliveries?.length??0,0);
  const page=await t.ok(t.call(1,{pending:{}},{binary:true}));assert.equal(page.deliveries!.length,7);const d=page.deliveries![0]!;
  assert.equal(d.source!.principalId,t.state.games[0]!.principal!.principalId);assert.equal(d.recipientGameSessionId,t.state.games[1]!.sessionId);assert.equal(d.scope!.worldToken,t.state.scopes[1]!.worldToken);assert.equal(d.provenance,'validated-client-report');
  const receipt={deliveryId:d.deliveryId!,receiptId:randomUUID(),outcome:'unknown'};
  const duplicates=await Promise.all([0,1].map(()=>t.f.call('/v3/hourglass/rpc',t.input(1,{receipt}),t.state.games[1]!.accessToken).then(r=>JSON.parse(r.body) as Response)));assert.ok(duplicates.every(r=>!r.error));
  await t.ok(t.call(1,{receipt}));assert.equal((await t.call(1,{receipt:{...receipt,outcome:'applied_reported'}})).error?.code,'CONFLICT');
  for(const [index,outcome] of [[1,'applied_reported'],[2,'not_applied_reported']] as const){const d=page.deliveries![index]!;await t.ok(t.call(1,{receipt:{deliveryId:d.deliveryId!,receiptId:randomUUID(),outcome}}));}
  assert.equal((await t.ok(t.call(1,{pending:{}}))).deliveries!.length,4);assert.equal((await t.ok(t.call(1,{status:{activationId:p.activationId}},{desktop:true}))).recipientStatuses![0]!.outcome,'unknown');
  assert.equal((await t.call(0,{receipt})).error?.code,'NOT_FOUND');assert.equal((await t.call(1,{status:{activationId:randomUUID()}})).error?.code,'NOT_FOUND');
  assert.equal((await t.call(0,{activate:{...activation(),effectId:'unapproved'}})).error?.code,'FORBIDDEN');assert.equal((await t.call(0,{activate:{...activation(),materialId:'water'}})).error?.code,'FORBIDDEN');
  await t.state.rpc(t.state.host,{endRun:{}});
  const ended=t.input(0,{activate:activation()});assert.ok(JSON.parse((await t.f.call('/v3/hourglass/rpc',ended,t.state.games[0]!.accessToken)).body).error);
  const status=t.input(1,{status:{activationId:p.activationId}},true);assert.equal(JSON.parse((await t.f.call('/v3/hourglass/rpc',status,t.state.desktops[1]!.accessToken)).body).recipientStatuses[0].outcome,'unknown');
 }finally{await t.f.close();}
});
test('R09 concurrent cross-source originals are unique, changed payload conflicts, durable DB guards cannot mutate or delete identity',async()=>{
 const t=await fixture();try{
  await t.state.start();await t.state.playing();const p=activation();
  const race=await Promise.all([0,1].map(i=>t.f.call('/v3/hourglass/rpc',t.input(i,{activate:p}),t.state.games[i]!.accessToken).then(r=>JSON.parse(r.body) as Response)));
  assert.equal(race.filter(r=>!r.error).length,1);assert.equal(race.filter(r=>r.error?.code==='CONFLICT').length,1);
  const winner=race[0]!.error?1:0;const original=await t.ok(t.call(winner,{activate:p}));assert.equal((await t.call(winner,{activate:{...p,effectId:'berserk',materialId:'magic_liquid_berserk'}})).error?.code,'CONFLICT');
  assert.equal((await t.f.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_hourglass_activations'))).rows[0].n,1);
  const rejects=async(sql:string)=>t.f.service.db.transaction(async tx=>{await tx.query('SAVEPOINT guard');try{await tx.query(sql);assert.fail('guard should refuse');}catch(e){await tx.query('ROLLBACK TO SAVEPOINT guard');return (e as {code:string}).code;}});
  assert.equal(await rejects("UPDATE g3_hourglass_activations SET record=jsonb_set(record,'{plan,source,worldToken}','\"replacement\"')"),'P0001');
  assert.equal(await rejects('DELETE FROM g3_hourglass_activations'),'P0001');
  assert.equal(await rejects('INSERT INTO g3_hourglass_activations(run_id,activation_id,sequence,record) SELECT run_id,activation_id,2,record FROM g3_hourglass_activations'),'23505');
  assert.deepEqual((await t.ok(t.call(winner,{status:{activationId:p.activationId}}))).activation,original.activation);
  const bad=t.input(winner,{activate:p,scope:{...t.state.scopes[winner],worldToken:'replacement'}});assert.ok(JSON.parse((await t.f.call('/v3/hourglass/rpc',bad,t.state.games[winner]!.accessToken)).body).error);
 }finally{await t.f.close();}
});
test('durable private pages capture32+1 prefix; restart retains originals and uncertainty but fences old authority',async()=>{
 const t=await fixture();try{
  await t.state.start();await t.state.playing();const originals:Required<Activate>[]=[];
  for(let i=0;i<33;i++){const p=activation(i%7);originals.push(p);await t.ok(t.call(0,{activate:p}));}
  const first=await t.ok(t.call(1,{pending:{}}));assert.equal(first.deliveries!.length,8);assert.equal(first.highSequence,33);
  let cursor=first.nextSequence!;while(cursor<32){const page=await t.ok(t.call(1,{pending:{afterSequence:cursor,throughSequence:first.highSequence!}}));assert.ok(page.deliveries!.length<=8);cursor=page.nextSequence!;}
  await t.ok(t.call(0,{activate:activation()}));const tail=await t.ok(t.call(1,{pending:{afterSequence:32,throughSequence:first.highSequence}}));assert.equal(tail.deliveries!.length,1);assert.equal(tail.deliveries![0]!.sequence,33);assert.deepEqual((await t.ok(t.call(1,{pending:{afterSequence:32,throughSequence:33}}))).deliveries,tail.deliveries);
  assert.equal((await t.call(1,{pending:{throughSequence:35}})).error?.code,'INVALID_MESSAGE');
  const before=(await t.f.service.db.transaction(tx=>tx.query('SELECT record FROM g3_hourglass_activations ORDER BY sequence'))).rows;
  const uncertain=first.deliveries![0]!;await t.ok(t.call(1,{receipt:{deliveryId:uncertain.deliveryId!,receiptId:randomUUID(),outcome:'unknown'}}));
  const persisted=(await t.f.service.db.transaction(tx=>tx.query('SELECT record FROM g3_hourglass_activations ORDER BY sequence'))).rows;
  assert.notDeepEqual(persisted,before);assert.equal(persisted[0]!.record.deliveries[0].outcome,'unknown');
  const old=t.input(1,{pending:{}});await t.f.restart();assert.equal(JSON.parse((await t.f.call('/v3/hourglass/rpc',old,t.state.games[1]!.accessToken)).body).error.code,'STALE_EPOCH');
  assert.deepEqual((await t.f.service.db.transaction(tx=>tx.query('SELECT record FROM g3_hourglass_activations ORDER BY sequence'))).rows,persisted);
  const status=t.input(1,{status:{activationId:originals[0]!.activationId}},true);assert.equal(JSON.parse((await t.f.call('/v3/hourglass/rpc',status,t.state.desktops[1]!.accessToken)).body).activation.activationId,originals[0]!.activationId);
 }finally{await t.f.close();}
});
test('protected observation/revocation refuses delivery writes while private original status retains source facts',async()=>{
 const t=await fixture();try{
  await t.state.start();await t.state.playing();const p=activation();await t.ok(t.call(0,{activate:p}));const delivery=(await t.ok(t.call(1,{pending:{}}))).deliveries![0]!;
  const scope=t.state.scopes[1]!;await t.f.service.db.transaction(tx=>tx.query("UPDATE world_observations SET body=jsonb_set(body,'{protected}','true') WHERE attachment_id=$1",[scope.attachmentId]));
  for(const payload of [{pending:{}},{receipt:{deliveryId:delivery.deliveryId!,receiptId:randomUUID(),outcome:'applied_reported'}},{activate:activation()}]){const body=t.input(1,payload);assert.ok(JSON.parse((await t.f.call('/v3/hourglass/rpc',body,t.state.games[1]!.accessToken)).body).error);}
  const status=t.input(1,{status:{activationId:p.activationId}},true);assert.equal(JSON.parse((await t.f.call('/v3/hourglass/rpc',status,t.state.desktops[1]!.accessToken)).body).recipientStatuses[0].outcome,'pending');
  const guessed={...status,scope:{...scope,roomId:randomUUID()}};assert.ok(JSON.parse((await t.f.call('/v3/hourglass/rpc',guessed,t.state.desktops[1]!.accessToken)).body).error);
  await t.f.service.db.transaction(tx=>tx.query('UPDATE sessions SET revoked_at=now() WHERE id=$1',[t.state.games[1]!.sessionId]));
  assert.ok(JSON.parse((await t.f.call('/v3/hourglass/rpc',t.input(1,{pending:{}}),t.state.games[1]!.accessToken)).body).error);
 }finally{await t.f.close();}
});
for(const mode of ['nemesis','missing-capability'])test(`hourglass refuses ${mode} and strict injected fields/epochs`,async()=>{
 const t=await fixture(mode==='nemesis'?'nemesis':'core',mode==='missing-capability'?['core.seed','core.ghost']:undefined);try{
  await t.state.start();await t.state.playing();assert.equal((await t.call(0,{activate:activation()})).error?.code,'FORBIDDEN');
  for(const patch of [{catalogueRevision:'unknown'},{sessionEpoch:randomUUID()},{serviceEpoch:randomUUID()},{serviceId:'forged'}]){const body={...t.input(0,{activate:activation()}),...patch};assert.ok(JSON.parse((await t.f.call('/v3/hourglass/rpc',body,t.state.games[0]!.accessToken)).body).error);}
  const body=t.input(0,{activate:activation()}) as any;body.activate.callback='evil()';assert.ok((await t.f.call('/v3/hourglass/rpc',body,t.state.games[0]!.accessToken)).status>=400);
 }finally{await t.f.close();}
});
test('R10 real one-issuer TLS maximum roster pages survive terminal receipts, End and restart below unchanged wire/database bounds',async()=>{
 const names=Array.from({length:32},(_,i)=>i%2?'"'.repeat(80):'😀'.repeat(20)),worlds=Array.from({length:32},(_,i)=>i%2?'\u0001'.repeat(80):'😀'.repeat(20));
 const t=await fixture('core',undefined,{playerCount:32,displayNames:names,worldTokens:worlds});try{
  await t.state.start();await t.state.playing();const p=activation(6),accepted=await t.ok(t.call(0,{activate:p}));assert.equal(accepted.activation!.recipientCount,31);assert.deepEqual((await t.ok(t.call(0,{activate:p}))).activation,accepted.activation);
  const discover=async(binary:boolean)=>{let cursor='',ids:string[]=[];do{const page=await t.ok(t.call(0,{status:{activationId:p.activationId,afterRecipientKey:cursor}},{desktop:true,binary,pulse:false}));assert.ok(Buffer.byteLength(JSON.stringify(page))<=32768);assert.equal(page.original!.recipientCount,31);assert.equal(page.original!.source!.name,names[0]);assert.ok(page.recipientStatuses!.length<=8);ids.push(...page.recipientStatuses!.map(d=>d.deliveryId!));cursor=page.nextRecipientKey!;if(cursor===page.highRecipientKey)break;}while(true);assert.equal(new Set(ids).size,31);const tail=await t.ok(t.call(0,{status:{activationId:p.activationId,afterRecipientKey:cursor}},{desktop:true,binary,pulse:false}));assert.equal(tail.recipientStatuses?.length??0,0);assert.equal(tail.original!.activationId,p.activationId);return ids;};
  const ids=await discover(false);assert.deepEqual(await discover(true),ids);
  const pending=await t.ok(t.call(1,{pending:{}},{binary:true}));assert.equal(pending.deliveries!.length,1);const d=pending.deliveries![0]!;const receipt={deliveryId:d.deliveryId!,receiptId:randomUUID(),outcome:'unknown'};
  const concurrent=await Promise.all([0,1].map(()=>t.f.call('/v3/hourglass/rpc',t.input(1,{receipt}),t.state.games[1]!.accessToken).then(r=>JSON.parse(r.body))));assert.ok(concurrent.every(r=>!r.error));
  const own=await t.ok(t.call(1,{status:{activationId:p.activationId}},{desktop:true,binary:true}));assert.equal(own.recipientStatuses!.length,1);assert.equal(own.recipientStatuses![0]!.receiptId,receipt.receiptId);assert.equal((await t.call(1,{status:{activationId:p.activationId,afterRecipientKey:t.state.games[2]!.sessionId!}},{desktop:true})).error?.code,'INVALID_MESSAGE');
  // Storage guard/bound fixture: retain all31 terminal reports simultaneously.
  // These synthesized reports are not authenticated applications or engine proof.
  await t.f.service.db.transaction(async tx=>{const row=(await tx.query('SELECT record FROM g3_hourglass_activations WHERE activation_id=$1',[p.activationId])).rows[0]!.record;for(const delivery of row.deliveries)if(delivery.outcome==='pending'){delivery.outcome='not_applied_reported';delivery.receiptId=randomUUID();}await tx.query('UPDATE g3_hourglass_activations SET record=$2 WHERE activation_id=$1',[p.activationId,row]);});
  const saved=(await t.f.service.db.transaction(tx=>tx.query('SELECT record,octet_length(record::text) AS bytes FROM g3_hourglass_activations'))).rows[0]!;assert.ok(saved.bytes<=32768);assert.equal(saved.record.worldTokenEncoding,'base64-utf8-v1');
  // PostgreSQL serialization bound for the wider128-byte isolated codec range;
  // query-only synthetic record, not a selected-game authority or inserted row.
  const widest=structuredClone(saved.record),token=Buffer.from('\u0001'.repeat(128)).toString('base64');widest.plan.source.worldToken=token;widest.plan.source.name='"'.repeat(80);for(const recipient of widest.plan.recipients)recipient.worldToken=token;for(const delivery of widest.deliveries)delivery.recipient.worldToken=token;
  assert.ok((await t.f.service.db.transaction(tx=>tx.query('SELECT octet_length($1::jsonb::text) AS bytes',[widest]))).rows[0]!.bytes<=32768);
  await t.state.rpc(t.state.host,{endRun:{}});assert.deepEqual(await discover(false),ids);await t.f.restart();assert.deepEqual(await discover(true),ids);assert.deepEqual((await t.f.service.db.transaction(tx=>tx.query('SELECT record FROM g3_hourglass_activations'))).rows[0]!.record,saved.record);
 }finally{await t.f.close();}
});
test('107 retains legacy plain originals and format while allowing only original terminal receipt changes',async()=>{
 const t=await fixture();try{
  await t.state.start();await t.state.playing();const p=activation();await t.ok(t.call(0,{activate:p}));
  const encoded=(await t.f.service.db.transaction(tx=>tx.query('SELECT record FROM g3_hourglass_activations'))).rows[0]!.record,existing=loadHourglass(encoded),oldId=randomUUID();
  const context={...existing.plan.scope,mode:'core',live:true,capabilities:['core.hourglass'],catalogueRevision:HOURGLASS_CATALOGUE.revision,sessionEpoch:randomUUID()};
  const sender={...context,...existing.plan.source},recipient={...context,...existing.plan.recipients[0]!,name:'Legacy fixture recipient'};
  const plain=new HourglassLedger({records:[]}).activate(sender,{...p,activationId:oldId},[sender,recipient]);
  await t.f.service.db.transaction(tx=>tx.query('INSERT INTO g3_hourglass_activations(run_id,activation_id,sequence,record) VALUES($1,$2,2,$3)',[t.state.scopes[0]!.runId,oldId,plain]));
  const pending=await t.ok(t.call(1,{pending:{afterSequence:1,throughSequence:2}}));const d=pending.deliveries![0]!,receipt={deliveryId:d.deliveryId!,receiptId:randomUUID(),outcome:'not_applied_reported'};
  await t.ok(t.call(1,{receipt}));await t.ok(t.call(1,{receipt}));
  await assert.rejects(t.f.service.db.transaction(tx=>tx.query("UPDATE g3_hourglass_activations SET record=record-'worldTokenEncoding' WHERE activation_id=$1",[p.activationId])));
  await assert.rejects(t.f.service.db.transaction(tx=>tx.query("UPDATE g3_hourglass_activations SET record=jsonb_set(record,'{worldTokenEncoding}','\"base64-utf8-v1\"') WHERE activation_id=$1",[oldId])));
  const legacy=(await t.f.service.db.transaction(tx=>tx.query('SELECT record FROM g3_hourglass_activations WHERE activation_id=$1',[oldId]))).rows[0]!.record;
  assert.equal(legacy.worldTokenEncoding,undefined);assert.equal(legacy.deliveries[0].receiptId,receipt.receiptId);
  // Recreate the106 guard/version boundary ONLY in this owned fixture, then
  // execute the actual107 upgrade with both existing formats present.
  const previous=await readFile(new URL('../../../db/migrations/106_g3_hourglass.sql',import.meta.url),'utf8');
  const previousGuard=previous.slice(previous.indexOf('CREATE FUNCTION'),previous.indexOf('CREATE TRIGGER')).replace('CREATE FUNCTION','CREATE OR REPLACE FUNCTION');
  await t.f.service.db.transaction(async tx=>{await tx.query(previousGuard);await tx.query('DELETE FROM schema_migrations WHERE version=107');});
  await t.f.restart();assert.deepEqual((await t.f.service.db.transaction(tx=>tx.query('SELECT record FROM g3_hourglass_activations WHERE activation_id=$1',[oldId]))).rows[0]!.record,legacy);
  await assert.rejects(t.f.service.db.transaction(tx=>tx.query("UPDATE g3_hourglass_activations SET record=jsonb_set(record,'{deliveries,0,receiptId}',to_jsonb($2::text)) WHERE activation_id=$1",[oldId,randomUUID()])));
  await assert.rejects(t.f.service.db.transaction(tx=>tx.query("UPDATE g3_hourglass_activations SET record=jsonb_set(record,'{worldTokenEncoding}','\"base64-utf8-v1\"') WHERE activation_id=$1",[oldId])));
  const status=t.input(1,{status:{activationId:oldId}},true);assert.equal(JSON.parse((await t.f.call('/v3/hourglass/rpc',status,t.state.desktops[1]!.accessToken)).body).recipientStatuses[0].outcome,'not_applied_reported');
 }finally{await t.f.close();}
});
