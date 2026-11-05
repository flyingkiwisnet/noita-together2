// Real isolated service/SQL. Bounded held transaction, no native/game process.
import test from'node:test';import assert from'node:assert/strict';import{b1Fixture}from'./b1-fixture-support.js';import{readRoomAdmissions}from'../src/g3/admission.js';
test('Existing total7 schedules real control ahead of optional saturation while canonical exclusion and expiry remain enforced',async()=>{
 const f=await b1Fixture({itemTransfers:true,worldRules:true});let release:(()=>void)|undefined;const pending:Promise<any>[]=[];
 try{const db=f.x.fixture.service.db,order:string[]=[],previous=db.transactionDiagnostic;db.transactionDiagnostic=e=>{previous?.(e);if(e.code==='COMMIT')order.push(e.workClass??'unknown');};
  const held=db.transaction(async tx=>{await tx.query('SELECT 1');await new Promise<void>(r=>{release=r;});});pending.push(held);
  for(let n=0;n<100&&!release;n++)await new Promise(r=>setTimeout(r,2));assert.ok(release);
  order.length=0;for(let n=db.workStats.optionalActive;n<5;n++)pending.push(db.transaction(tx=>tx.query('SELECT 1')));
  assert.equal(db.workStats.optionalActive,5);assert.equal(db.workStats.queuedOptional,4);assert.equal(db.workStats.executing,'optional');
  for(const family of ['c1','c2']){const c=f.values[0].credentials,q={major:1,revision:'g3-'+family+'-1',serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:c.sessionEpoch,requestId:'reserved-'+family,roomId:f.scope(0).roomId,rulesGet:{}};const response=JSON.parse((await f.x.fixture.call('/v3/'+family+'/rpc',q,c.accessToken)).body);assert.equal(response.error?.code,'CAPACITY');assert.equal(db.workStats.active,5);assert.equal(db.workStats.queuedControl,0);}
  const control=f.x.send(f.x.desktop,f.qa),presence=f.x.rpc(f.x.desktop,{presence:{present:true}});pending.push(control,presence);
  for(let n=0;n<100&&db.workStats.queuedControl<2;n++)await new Promise(r=>setTimeout(r,2));assert.equal(db.workStats.queuedControl,2);assert.equal(db.workStats.active,7);assert.equal(db.workStats.poolWaiting,0);
  const refused=await f.send(0,{itemBank:{}});assert.equal(refused.error.code,'CAPACITY');assert.equal(db.workStats.active,7);
  release!();release=undefined;await Promise.all(pending);assert.equal((await control).world.mayPlay,true);assert.deepEqual(order.slice(0,3),['optional','control','control']);for(let n=0;n<100&&db.workStats.active;n++)await new Promise(r=>setTimeout(r,2));assert.equal(db.workStats.active,0);assert.ok(db.workStats.peak<=7);
  const room=f.scope(0).roomId!,compat=f.x.fixture.profile.compatibility,now=f.x.fixture.service.auth.now();
  await db.transaction(async tx=>{const expired=await readRoomAdmissions(tx,db,compat,now+31001,room);assert.ok(expired.worlds.every(w=>!w.decision.playing));});
  await f.x.moderate('peer','kick');assert.ok((await f.send(1,{itemBank:{}})).error);assert.equal((await f.ok(0,{itemBank:{}})).itemBank.total,0);db.transactionDiagnostic=previous;
 }finally{release?.();await Promise.allSettled(pending);await f.close();}
});
