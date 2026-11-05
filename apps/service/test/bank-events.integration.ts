// Actual authenticated TLS/WSS/SQL; local engine outcomes are injected receipts.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {b1Fixture} from './b1-fixture-support.js';import {livePeer} from '../scripts/g3-live-fixture.js';
import {createRequire} from 'node:module';
import {ITEM_CATALOGUE} from '../../../packages/domain/src/g3-items.js';
import {markBankDirty,bankRevision} from '../src/g3/bank-events.js';import {PolicyError} from '../src/g1/security.js';import {G3Authority} from '../src/g3/authority.js';
const pause=(ms:number)=>new Promise<void>(resolve=>setTimeout(resolve,ms));
const item=()=>({catalogueRevision:ITEM_CATALOGUE.revision,object:{objectId:ITEM_CATALOGUE.objects.find(v=>v.available)!.id}});
const proof=(o:any,outcome='escrowed')=>({itemReceipt:{operationId:o.operationId,deliveryId:o.deliveryId,payloadDigest:o.payloadDigest,receiptId:randomUUID(),outcome,evidenceDigest:'b'.repeat(64)}});
test('committed bank revisions, real WSS invalidation, partial exact-original batches and lost committed batch replies',async()=>{
 const f=await b1Fixture({itemTransfers:true}),peers:Awaited<ReturnType<typeof livePeer>>[]=[];
 try{
  const db=f.x.fixture.service.db,scope=f.scope(0),games=f.values.map(v=>({...v.credentials,principal:{principalId:v.credentials.accountId}})),s={scopes:[scope,f.scope(1)],games};
  const foreign={...scope,runId:randomUUID(),roomId:randomUUID()};const inaccessible=await f.send(0,{bankBatch:{entries:[{scope:foreign,itemStatus:{operationId:randomUUID()}}]}},foreign);assert.equal(inaccessible.error.code,'NOT_FOUND');assert.equal(inaccessible.bankRevision,undefined,'all-refused batch cannot disclose a foreign bank revision');
  for(let i=0;i<2;i++)peers.push(await livePeer(f.x.fixture as any,s as any,i));
  await peers[1]!.wait(v=>!!v.bankChanged);peers.forEach(p=>p.received.splice(0));
  const original={operationId:randomUUID(),item:item()},second={operationId:randomUUID(),item:item()},invalid={operationId:randomUUID(),item:{catalogueRevision:ITEM_CATALOGUE.revision,object:{objectId:'not-catalogued'}}};
  const request={bankBatch:{entries:[{scope,itemDeposit:original},{scope,itemDeposit:invalid},{scope,itemDeposit:second},{scope,itemDeposit:original}]}};
  f.x.advance(1100);f.x.faults.goldControl.arm({target:'consumer',operationId:original.operationId,kind:'itemDeposit'});await assert.rejects(f.send(0,request));
  f.x.advance(1100);const accepted=await f.ok(0,request),entries=accepted.bankBatch.entries;assert.equal(entries[1].error.code,'UNSUPPORTED_CONTENT');assert.equal(entries[0].itemOperation.deliveryId,entries[3].itemOperation.deliveryId);
  assert.equal((await db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_item_operations'))).rows[0].n,2);
  const a=proof(entries[0].itemOperation),b=proof(entries[2].itemOperation),receipts={bankBatch:{entries:[{scope,...a},{scope,...b},{scope,...a},{scope,itemReceipt:{...a.itemReceipt,evidenceDigest:'c'.repeat(64)}}]}};
  f.x.advance(1100);f.x.faults.goldControl.arm({target:'consumer',operationId:original.operationId,kind:'itemReceipt'});await assert.rejects(f.send(0,receipts));
  f.x.advance(1100);const committed=await f.ok(0,receipts);assert.equal(committed.bankBatch.entries[3].error.code,'CONFLICT');assert.equal(committed.bankBatch.entries[0].itemOperation.state,'completed');assert.ok(BigInt(committed.bankRevision)>BigInt(accepted.bankRevision));
  const notified=await peers[1]!.wait(v=>!!v.bankChanged&&BigInt(v.bankChanged.revision!)>=BigInt(committed.bankRevision));assert.equal(notified.scope!.runId,scope.runId);assert.equal(notified.scope!.principalId,games[1].accountId);
  assert.equal((await db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_item_receipts'))).rows[0].n,2,'lost replies and duplicate entries never add receipts');
  const before=await db.transaction(tx=>bankRevision(tx,scope.runId!)),seen=peers[1]!.received.filter(v=>v.frame.bankChanged).length;
  await assert.rejects(db.transaction(async tx=>{await tx.query('UPDATE g3_bank_items SET state=\'cancelled\' WHERE run_id=$1',[scope.runId]);markBankDirty(tx,scope.runId!);throw new PolicyError('FORBIDDEN');}),/FORBIDDEN/);
  await pause(350);assert.equal(await db.transaction(tx=>bankRevision(tx,scope.runId!)),before);assert.equal(peers[1]!.received.filter(v=>v.frame.bankChanged).length,seen,'rollback publishes no invalidation');
  f.x.advance(1100);const page=await f.ok(1,{itemBank:{}});assert.equal(page.itemBank.total,2);assert.equal(page.itemBank.operations.length,0);assert.equal(page.bankRevision,before);
  const row=page.itemBank.items[0];f.x.advance(1100);const claims=await Promise.all([0,1].map(i=>f.send(i,{bankBatch:{entries:[{scope:f.scope(i),itemClaim:{operationId:randomUUID(),itemId:row.itemId,expectedRevision:row.revision}}]}})));
  assert.equal(claims.filter(v=>v.bankBatch.entries[0].itemOperation).length,1);assert.equal(claims.find(v=>v.bankBatch.entries[0].error).bankBatch.entries[0].error.code,'CLAIM_LOST');
  f.x.advance(1100);const gold=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:7}})).operation;const goldDone=await f.ok(0,f.receipt(gold,'escrowed'));await peers[1]!.wait(v=>!!v.bankChanged&&BigInt(v.bankChanged.revision!)>=BigInt(goldDone.bankRevision));assert.equal((await f.ok(1,{goldBank:{}})).bank.availableBalance,7);
  await f.x.end();await peers[1]!.wait(v=>!!v.retire);assert.ok(peers[1]!.received.some(v=>v.frame.retire));
 }finally{for(const p of peers)await p.close();await f.close();}
});
test('public page reads share only committed query/revision data while private originals and current authority remain caller scoped',async()=>{
 const f=await b1Fixture({itemTransfers:true});let restore=()=>{};
 try{
  const prototype=G3Authority.prototype as any,execute=prototype.execute;let cache:any;const observed=async function(this:G3Authority,...args:any[]){cache=this.bankReads;return execute.apply(this,args);};prototype.execute=observed;restore=()=>{if(prototype.execute===observed)prototype.execute=execute;};
  const original=(await f.ok(0,{itemDeposit:{operationId:randomUUID(),item:item()}})).itemOperation;
  const first=await f.ok(0,{itemBank:{}}),hits=cache.stats.hits;assert.equal(first.itemBank.operations.length,1);
  const peer=await f.ok(1,{itemBank:{}});assert.equal(cache.stats.hits,hits+1);assert.equal(peer.itemBank.operations.length,0,'shared public rows do not share private pending originals');
  const misses=cache.stats.misses;await f.ok(1,{itemBank:{search:'no-result'}});assert.equal(cache.stats.misses,misses+1);
  await f.ok(0,proof(original));const after=await f.ok(1,{itemBank:{}});assert.ok(BigInt(after.bankRevision)>BigInt(first.bankRevision));assert.equal(after.itemBank.total,1,'revision change cannot reuse stale empty public page');
  f.x.advance(1100);await f.x.moderate('peer','ban');const priorHits=cache.stats.hits;const denied=await f.send(1,{itemBank:{}});assert.ok(denied.error);assert.equal(cache.stats.hits,priorHits,'cached data cannot bypass current canonical exclusion');
 }finally{restore();await f.close();}
});
test('real response-budget savepoint rollback removes a large new original and its revision; exact single retry remains usable',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const examples=createRequire(import.meta.url)('../../../docs/g3/server-item-transfer-examples.json'),wand=structuredClone(examples.descriptors.wand),longest=[...ITEM_CATALOGUE.actions].sort((a,b)=>b.id.length-a.id.length)[0]!.id;
  wand.wand.deckCapacity='1024';wand.wand.deck=Array.from({length:1024},(_,slot)=>({slot,spell:{actionId:longest,usesRemaining:'2147483647'}}));wand.wand.permanent=Array.from({length:256},(_,slot)=>({slot,spell:{actionId:longest,usesRemaining:'2147483647'}}));
  assert.ok(Buffer.byteLength(JSON.stringify(wand))<131072);const scope=f.scope(0),ids:string[]=[];
  for(let i=0;i<3;i++){f.x.advance(1100);const id=randomUUID();ids.push(id);await f.ok(0,{itemDeposit:{operationId:id,item:wand}});}
  const db=f.x.fixture.service.db,before=await db.transaction(tx=>bankRevision(tx,scope.runId!)),newId=randomUUID();f.x.advance(1100);
  const result=await f.ok(0,{bankBatch:{entries:[...ids.map(operationId=>({scope,itemStatus:{operationId}})),{scope,itemDeposit:{operationId:newId,item:wand}}]}});
  assert.equal(result.bankBatch.entries[3].error.code,'BATCH_TOO_LARGE');assert.equal(result.bankRevision,before);
  assert.equal((await db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_item_operations WHERE operation_id=$1',[newId]))).rows[0].n,0);
  f.x.advance(1100);const retry=await f.ok(0,{itemDeposit:{operationId:newId,item:wand}});assert.equal(retry.itemOperation.operationId,newId);assert.ok(BigInt(retry.bankRevision)>BigInt(before));
 }finally{await f.close();}
});
