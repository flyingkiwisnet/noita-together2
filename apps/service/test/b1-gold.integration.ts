// Actual authenticated TLS/SQL/LC1 service; engine/wallet outcomes are injected.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {b1Fixture} from './b1-fixture-support.js';
import {G3Authority} from '../src/g3/authority.js';
import {readRoomAdmissions} from '../src/g3/admission.js';
import {PolicyError} from '../src/g1/security.js';
test('B1 original deposit/reservation receipts, shared versioned balance and named live notices',async()=>{
 const f=await b1Fixture();try{
  const initial=(await f.ok(1,{goldBank:{}})).bank;assert.equal(initial.availableBalance,0);assert.equal(initial.balanceVersion,'0');assert.equal(initial.sendGoldEnabled,true);assert.equal(initial.notices.length,0);
  const original={goldIntent:{operationId:randomUUID(),direction:'deposit',amount:100}},deposit=(await f.ok(0,original)).operation;
  assert.equal(deposit.balance,0);assert.equal(deposit.state,'awaiting_local');assert.equal(deposit.effectAllowed,true);assert.deepEqual(deposit.originalScope,f.scope(0));assert.equal(deposit.savePersistence,'unproven');
  const proof=f.receipt(deposit,'escrowed'),credited=(await f.ok(0,proof)).operation;assert.equal(credited.balance,100);assert.equal(credited.balanceVersion,'1');assert.equal(credited.state,'completed');
  assert.equal((await f.ok(0,proof)).operation.balanceVersion,'1');assert.equal((await f.ok(0,original)).operation.deliveryId,deposit.deliveryId);
  const live=(await f.ok(1,{goldBank:{afterNoticeSequence:initial.noticeSequence}})).bank;assert.equal(live.availableBalance,100);assert.equal(live.notices.length,1);assert.equal(live.notices[0].amount,100);assert.equal(live.notices[0].principalId,f.values[0].credentials.accountId);assert.ok(live.notices[0].displayName);
  assert.equal((await f.ok(1,{goldBank:{afterNoticeSequence:live.noticeSequence}})).bank.notices.length,0);assert.equal((await f.ok(1,{goldBank:{}})).bank.notices.length,0,'late/reconnected baseline has no history');
  const reserve=(await f.ok(1,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:30}})).operation;assert.equal(reserve.state,'reserved');assert.equal(reserve.balance,70);assert.equal(reserve.balanceVersion,'2');
  assert.equal((await f.ok(1,{goldBank:{}})).bank.operations[0].deliveryId,reserve.deliveryId);
  assert.equal((await f.ok(1,f.receipt(reserve,'applied'))).operation.balance,70);
  const rows=(await f.x.fixture.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS n,sum(delta)::int AS total FROM g3_ledger'))).rows[0];assert.deepEqual(rows,{n:2,total:70});
  const out=(await f.ok(0,{outboxList:{limit:32}})).outbox;assert.equal(out.filter((v:any)=>v.kind==='gold.deposit').length,1);
 }finally{await f.close();}
});
test('B1 validates bounds and reserves deposit headroom before any wallet debit',async()=>{
 const f=await b1Fixture();try{
  for(const amount of [0,-1,0.5,1000001,4294967296])assert.equal((await f.send(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount}})).error.code,'INVALID_MESSAGE');
  const db=f.x.fixture.service.db;
  // Labelled synthetic capacity boundary in the fresh owned fixture schema only.
  await db.transaction(tx=>tx.query('INSERT INTO g3_balances(run_id,amount) VALUES($1,4294967290)',[f.scope(0).runId]));
  const held=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:5}})).operation;
  assert.equal((await f.send(1,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}})).error.code,'BANK_FULL');
  assert.equal((await f.ok(0,f.receipt(held,'not_applied'))).operation.state,'cancelled');
  const max=(await f.ok(1,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:5}})).operation;
  assert.equal((await f.ok(1,f.receipt(max,'escrowed'))).operation.balance,4294967295);
  assert.equal((await f.send(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}})).error.code,'BANK_FULL');
  assert.equal((await f.ok(0,{goldBank:{}})).bank.balanceVersion,'1');
 }finally{await f.close();}
});
test('B1 concurrent TAKE, original/receipt conflicts and uncertain reservations never duplicate or refund',async()=>{
 const f=await b1Fixture();try{
  const original={goldIntent:{operationId:randomUUID(),direction:'deposit',amount:100}},d=(await f.ok(0,original)).operation;
  assert.equal((await f.send(0,{goldIntent:{...original.goldIntent,amount:101}})).error.code,'CONFLICT');
  const receipt=f.receipt(d,'escrowed');await f.ok(0,receipt);
  assert.equal((await f.send(0,{goldReceipt:{...receipt.goldReceipt,evidenceDigest:'b'.repeat(64)}})).error.code,'CONFLICT');
  const results=await Promise.all([0,1].map(i=>f.send(i,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:100}})));
  assert.equal(results.filter(v=>v.operation).length,1);assert.equal(results.find(v=>v.error).error.code,'INSUFFICIENT_FUNDS');
  const i=results[0].operation?0:1,o=results[i].operation;assert.equal(o.balance,0);assert.equal(o.balanceVersion,'2');
  const pending=(await f.ok(i,{goldStatus:{operationId:o.operationId}})).operation;assert.equal(pending.state,'reserved');assert.equal(pending.effectAllowed,true);
  const uncertain=(await f.ok(i,f.receipt(o,'uncertain'))).operation;assert.equal(uncertain.state,'uncertain');assert.equal(uncertain.effectAllowed,false);
  assert.equal((await f.ok(i,{goldBank:{}})).bank.availableBalance,0);assert.equal((await f.ok(i,{goldBank:{}})).bank.operations[0].effectAllowed,false);
  assert.equal((await f.x.fixture.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_ledger'))).rows[0].n,2);
 }finally{await f.close();}
});
test('B1 temporary admission and lost committed replies recover exact originals with one ledger entry and one live notice',async()=>{
 const f=await b1Fixture();try{
  const baseline=(await f.ok(1,{goldBank:{}})).bank;
  const original={goldIntent:{operationId:randomUUID(),direction:'deposit',amount:40}};
  // Hold real database optional admission, without issuing an original or
  // changing the admission limit. Release before the existing absolute deadline.
  const db=f.x.fixture.service.db;let release!:()=>void,started!:()=>void;
  const entered=new Promise<void>(resolve=>{started=resolve;}),held=new Promise<void>(resolve=>{release=resolve;});
  const occupying=[db.transaction(async()=>{started();await held;})];await entered;
  for(let i=1;i<db.workStats.optionalLimit;i++)occupying.push(db.transaction(async()=>{}));
  try{assert.equal((await f.send(0,original)).error.code,'CAPACITY');}
  finally{release();await Promise.all(occupying);}
  assert.equal((await db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_operations'))).rows[0].n,0);
  f.x.faults.goldControl.arm({target:'consumer',operationId:original.goldIntent.operationId,kind:'goldIntent'});
  await assert.rejects(f.send(0,original));
  const d=(await f.ok(0,original)).operation,proof=f.receipt(d,'escrowed');
  f.x.faults.goldControl.arm({target:'consumer',operationId:d.operationId!,kind:'goldReceipt'});
  await assert.rejects(f.send(0,proof));
  assert.equal((await f.ok(0,{goldStatus:{operationId:d.operationId}})).operation.state,'completed');
  assert.equal((await f.ok(0,proof)).operation.balanceVersion,'1');
  assert.equal((await f.ok(1,{goldBank:{afterNoticeSequence:baseline.noticeSequence}})).bank.notices.length,1);
  assert.equal(f.x.faults.goldControl.status().events.filter(v=>v.committed).length,2);
  const rows=(await f.x.fixture.service.db.transaction(tx=>tx.query('SELECT (SELECT count(*)::int FROM g3_operations) AS operations,(SELECT count(*)::int FROM g3_receipts) AS receipts,(SELECT count(*)::int FROM g3_ledger) AS ledger'))).rows[0];assert.deepEqual(rows,{operations:1,receipts:1,ledger:1});
 }finally{await f.close();}
});
test('B1 a transaction failure rolls back reservation, balance version, ledger and outbox together',async()=>{
 const f=await b1Fixture();try{
  const d=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:50}})).operation;await f.ok(0,f.receipt(d,'escrowed'));
  const {db,auth}=f.x.fixture.service,authority=new G3Authority(()=>db.epoch,auth.now);
  authority.admission=(tx,id)=>readRoomAdmissions(tx,db,f.x.fixture.profile.compatibility,auth.now(),id);
  const original={goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:20}},q=f.body(0,original);
  await assert.rejects(db.transaction(async tx=>{const actor=await auth.authenticate(tx,f.values[0].credentials.accessToken);await authority.execute(tx,actor,q);throw new PolicyError('B1_INJECTED_ROLLBACK');}),/B1_INJECTED_ROLLBACK/);
  const row=(await db.transaction(tx=>tx.query('SELECT amount,version,(SELECT count(*)::int FROM g3_operations) AS operations,(SELECT count(*)::int FROM g3_ledger) AS ledger,(SELECT count(*)::int FROM g3_outbox) AS outbox FROM g3_balances'))).rows[0];assert.deepEqual(row,{amount:'50',version:'1',operations:1,ledger:1,outbox:4});
  assert.equal((await f.ok(0,original)).operation.balance,30);
 }finally{await f.close();}
});
test('B1 current authority can recover same-world originals while stale, foreign and terminal effects are fenced',async()=>{
 const f=await b1Fixture();try{
  const d=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:100}})).operation;await f.ok(0,f.receipt(d,'escrowed'));
  const o=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:20}})).operation,original=f.scope(0),oldCredentials=f.values[0].credentials;
  assert.equal((await f.send(1,{goldStatus:{operationId:o.operationId}},original)).error.code,'NOT_FOUND');
  assert.equal((await f.send(0,{goldStatus:{operationId:o.operationId}},{...original,worldToken:randomUUID()})).error.code,'NOT_FOUND');
  f.qa.reconcile!.observation={...f.qa.reconcile!.observation!,protected:true,observationRevision:3};await f.x.send(f.x.desktop,f.qa);
  assert.equal((await f.ok(0,{goldStatus:{operationId:o.operationId}})).operation.effectAllowed,false);
  assert.equal((await f.send(0,f.receipt(o,'applied'))).error.code,'STALE_EPOCH');
  f.x.advance(31001);
  const recovery=f.x.request(f.x.desktop,2,'b1-new-owner',{...f.x.observation,protected:true,observationRevision:4,gameInstanceId:'b1-new-game'});await f.x.send(f.x.desktop,recovery);recovery.reconcile!.observation!.protected=false;recovery.reconcile!.observation!.observationRevision=5;const current=await f.x.send(f.x.desktop,recovery);assert.equal(current.error,undefined);assert.equal(current.world.mayPlay,true);
  f.values[0]=current;const scope=f.scope(0);assert.notEqual(scope.attachmentId,original.attachmentId);
  const status=(await f.ok(0,{goldStatus:{operationId:o.operationId},currentScope:scope},original)).operation;
  assert.equal(status.effectAllowed,true);assert.equal(status.state,'reserved');assert.equal(status.payloadDigest,o.payloadDigest);assert.equal(status.deliveryId,o.deliveryId);assert.deepEqual(status.originalScope,original);
  const stale=await f.raw(oldCredentials,f.body(0,{goldStatus:{operationId:o.operationId}},original,oldCredentials));assert.ok(stale.error||stale.operation.effectAllowed===false);
  const proof=f.receipt(o,'applied');assert.equal((await f.ok(0,{...proof,currentScope:scope},original)).operation.state,'completed');
  const pending=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:1}})).operation;
  await f.x.end();
  const historical=await f.raw(f.x.desktop,f.body(0,{goldStatus:{operationId:pending.operationId}},scope,f.x.desktop));assert.equal(historical.operation.state,'reserved');assert.equal(historical.operation.effectAllowed,false);
  const retry=await f.raw(f.x.desktop,f.body(0,proof,original,f.x.desktop));assert.equal(retry.operation.state,'completed');
  const denied=await f.raw(f.x.desktop,f.body(0,f.receipt(pending,'applied'),scope,f.x.desktop));assert.equal(denied.error.code,'FORBIDDEN');
  const outbox=await f.raw(f.x.desktop,f.body(0,{outboxList:{limit:32}},scope,f.x.desktop));assert.ok(outbox.outbox.filter((v:any)=>v.operation).every((v:any)=>v.operation.effectAllowed===false));
 }finally{await f.close();}
});
test('B1 paged pending reservations and live notice expiry preserve authoritative balance',async()=>{
 const f=await b1Fixture();try{
  const baseline=(await f.ok(1,{goldBank:{}})).bank;
  const d=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:100}})).operation;await f.ok(0,f.receipt(d,'escrowed'));
  for(let i=0;i<10;i++)await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:1}});
  const a=(await f.ok(0,{goldBank:{}})).bank,b=(await f.ok(0,{goldBank:{afterOperationId:a.nextOperationId}})).bank;assert.equal(a.operations.length,8);assert.equal(b.operations.length,2);assert.equal(b.nextOperationId,'');assert.equal(new Set([...a.operations,...b.operations].map(v=>v.operationId)).size,10);assert.equal(b.availableBalance,90);assert.equal(b.balanceVersion,'11');
  f.x.advance(10001);const expired=(await f.ok(1,{goldBank:{afterNoticeSequence:baseline.noticeSequence}})).bank;assert.equal(expired.notices.length,0);assert.equal(expired.noticeReset,true);assert.equal(expired.availableBalance,90);
  const future=(await f.ok(1,{goldBank:{afterNoticeSequence:'9223372036854775807'}})).bank;assert.equal(future.noticeReset,true);assert.equal(future.noticeSequence,expired.noticeSequence);
  await f.x.end();await f.x.nextRun();await f.x.rpc(f.x.desktop,{roomJoin:{}});
  const fresh=await f.x.prepareWorld(f.x.desktop),q=f.x.request();q.reconcile!.association=fresh.association;q.reconcile!.proof=fresh.proof;q.reconcile!.observation!.worldToken=fresh.association.worldId!;
  await f.x.send(f.x.desktop,q);await f.x.start({selectedWorldIds:[fresh.association.worldId!],excludedDesktopIds:[]});q.reconcile!.observation!.protected=false;q.reconcile!.observation!.observationRevision=2;f.values[0]=await f.x.send(f.x.desktop,q);
  const clean=(await f.ok(0,{goldBank:{}})).bank;assert.equal(clean.availableBalance,0);assert.equal(clean.balanceVersion,'0');assert.equal(clean.operations.length,0);assert.equal(clean.notices.length,0);
 }finally{await f.close();}
});
test('B1 slow committed bank reply leaves lifecycle health, movement and social requests progressing',async()=>{
 const f=await b1Fixture();try{
  const original={goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}};
  f.x.faults.goldControl.arm({target:'consumer',operationId:original.goldIntent.operationId,kind:'goldIntent',effect:'delay',delayMs:5000});
  let resolved=false;const pending=f.ok(0,original).then(v=>{resolved=true;return v;});
  try{
   for(let n=0;n<100&&!f.x.faults.goldControl.status().events.length;n++)await new Promise(r=>setTimeout(r,10));
   assert.equal(f.x.faults.goldControl.status().events[0]?.effect,'delay');assert.equal(resolved,false);
   assert.equal((await f.x.send(f.x.desktop,f.qa)).world.mayPlay,true);
   await f.ok(0,{movement:{frames:[{sequence:1,xMilli:'0',yMilli:'0',vxMilli:'0',vyMilli:'0',facingLeft:false}]}});
   assert.equal((await f.ok(1,{ghostList:{}})).ghosts.length,1);
   assert.ok((await f.x.chat('peer','Bank delay does not hold social work')).chat.sequence);assert.equal(resolved,false);
  }finally{f.x.faults.goldControl.release();await pending;}
 }finally{await f.close();}
});
test('B1 moderation fences queued effects while preserving historical original inspection',async()=>{
 const f=await b1Fixture();try{
  const o=(await f.ok(1,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}})).operation,original=f.scope(1);
  await f.x.moderate('peer','kick');
  const status=await f.raw(f.peer.desktop,f.body(1,{goldStatus:{operationId:o.operationId}},original,f.peer.desktop));assert.equal(status.operation.state,'awaiting_local');assert.equal(status.operation.effectAllowed,false);
  const effect=await f.send(1,f.receipt(o,'escrowed'));assert.ok(effect.error);assert.equal((await f.ok(0,{goldBank:{}})).bank.availableBalance,0);
 }finally{await f.close();}
});
test('B1 Send Gold off and missing current gold capability deny new actions without mutation',async()=>{
 for(const options of [{sendGold:false},{includeGoldCapability:false}]){
  const f=await b1Fixture(options);try{
   if(options.sendGold===false)assert.equal((await f.ok(0,{goldBank:{}})).bank.sendGoldEnabled,false);
   else assert.equal((await f.send(0,{goldBank:{}})).error.code,'FORBIDDEN');
   for(const direction of ['deposit','withdraw'])assert.equal((await f.send(0,{goldIntent:{operationId:randomUUID(),direction,amount:1}})).error.code,'FORBIDDEN');
   assert.equal((await f.x.fixture.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_operations'))).rows[0].n,0);
  }finally{await f.close();}
 }
});
