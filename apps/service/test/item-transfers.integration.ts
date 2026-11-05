// Actual authenticated HTTPS/SQL/LC1. Engine removal/spawn outcomes are injected.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {createRequire} from 'node:module';
import {b1Fixture} from './b1-fixture-support.js';import {ITEM_CATALOGUE as C,normalizeItem} from '../../../packages/domain/src/g3-items.js';
import {fingerprint} from '../src/g1/security.js';import {itemOriginal} from '../../../packages/domain/src/g3-items.js';
import {G3Authority} from '../src/g3/authority.js';import {readRoomAdmissions} from '../src/g3/admission.js';import {PolicyError} from '../src/g1/security.js';import {schema as socialSchema} from '../../../packages/contracts/src/g3-social.js';
const examples=createRequire(import.meta.url)('../../../docs/g3/server-item-transfer-examples.json');
type F=Awaited<ReturnType<typeof b1Fixture>>;
const receipt=(o:any,outcome:string)=>({itemReceipt:{operationId:o.operationId,deliveryId:o.deliveryId,payloadDigest:o.payloadDigest,receiptId:randomUUID(),outcome,evidenceDigest:'b'.repeat(64)}});
async function deposit(f:F,item:any,i=0){f.x.advance(100);const original={itemDeposit:{operationId:randomUUID(),item}},o=(await f.ok(i,original)).itemOperation;f.x.advance(100);const proof=receipt(o,'escrowed'),done=(await f.ok(i,proof)).itemOperation;return {original,o,proof,done};}
const object=()=>({catalogueRevision:C.revision,object:{objectId:C.objects.find(v=>v.available).id}});
test('All supported item families escrow then exclusively claim and individually complete, preserving descriptors/gold/notices',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const baseline=(await f.ok(1,{itemBank:{}})).itemBank;assert.equal(baseline.total,0);assert.equal(baseline.pageCount,1);
  for(const item of [examples.descriptors.spell,examples.descriptors.wand,examples.descriptors.container,examples.descriptors.emptyPowder,object()]){
   const d=await deposit(f,item);assert.equal(d.o.state,'awaiting_local');assert.equal(d.o.effectAllowed,true);assert.equal(d.done.state,'completed');assert.deepEqual(d.done.item,normalizeItem(item));assert.equal(d.o.payloadDigest,fingerprint(itemOriginal(f.values[0].credentials.accountId,f.scope(0),'deposit',normalizeItem(item))));
   const inspected=(await f.ok(1,{itemInspect:{itemId:d.o.itemId}})).itemDetail;assert.equal(inspected.summary.state,'available');assert.deepEqual(inspected.item,normalizeItem(item));
   f.x.advance(100);const claim={itemClaim:{operationId:randomUUID(),itemId:d.o.itemId,expectedRevision:inspected.summary.revision}},o=(await f.ok(1,claim)).itemOperation;assert.equal(o.state,'reserved');assert.equal(o.effectAllowed,true);assert.equal(o.savePersistence,'unproven');
   f.x.advance(100);assert.equal((await f.ok(1,claim)).itemOperation.deliveryId,o.deliveryId);f.x.advance(100);const p=receipt(o,'applied');assert.equal((await f.ok(1,p)).itemOperation.state,'completed');f.x.advance(100);assert.equal((await f.ok(1,p)).itemOperation.state,'completed');
  }
  const bank=(await f.ok(1,{itemBank:{afterNoticeSequence:baseline.noticeSequence}})).itemBank;assert.equal(bank.total,0);assert.equal(bank.notices.length,5);assert.ok(bank.notices.every((n:any)=>n.count===1&&n.displayName));assert.equal((await f.ok(1,{itemBank:{afterNoticeSequence:bank.noticeSequence}})).itemBank.notices.length,0);assert.equal((await f.ok(1,{itemBank:{}})).itemBank.notices.length,0);
  const gold=(await f.ok(0,{goldBank:{}})).bank;assert.equal(gold.availableBalance,0);const outbox=(await f.ok(0,{outboxList:{limit:32}})).outbox;assert.equal(outbox.filter((e:any)=>e.kind==='item.deposit').length,5);assert.ok(outbox.filter((e:any)=>e.itemOperation).every((e:any)=>!e.itemOperation.effectAllowed&&!e.itemOperation.item));
 }finally{await f.close();}
});

test('Same-world turnover recovers immutable originals; protection, wrong-world, End and deleted lobby fence effects',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const d=await deposit(f,examples.descriptors.wand),original=f.scope(0),oldCredentials=f.values[0].credentials;
  const entry=(await f.ok(0,{itemInspect:{itemId:d.o.itemId}})).itemDetail.summary;
  const o=(await f.ok(0,{itemClaim:{operationId:randomUUID(),itemId:entry.itemId,expectedRevision:entry.revision}})).itemOperation;
  f.qa.reconcile!.observation={...f.qa.reconcile!.observation!,protected:true,observationRevision:3};await f.x.send(f.x.desktop,f.qa);
  assert.equal((await f.ok(0,{itemStatus:{operationId:o.operationId}})).itemOperation.effectAllowed,false);assert.equal((await f.send(0,receipt(o,'applied'))).error.code,'STALE_EPOCH');
  f.x.advance(31001);const recovery=f.x.request(f.x.desktop,2,'items-new-owner',{...f.x.observation,protected:true,observationRevision:4,gameInstanceId:'items-new-game'});await f.x.send(f.x.desktop,recovery);recovery.reconcile!.observation!.protected=false;recovery.reconcile!.observation!.observationRevision=5;f.values[0]=await f.x.send(f.x.desktop,recovery);assert.equal(f.values[0].world.mayPlay,true);
  const scope=f.scope(0);assert.notEqual(scope.attachmentId,original.attachmentId);
  const status=(await f.ok(0,{itemStatus:{operationId:o.operationId},currentScope:scope},original)).itemOperation;assert.equal(status.effectAllowed,true);assert.equal(status.deliveryId,o.deliveryId);assert.equal(status.payloadDigest,o.payloadDigest);assert.deepEqual(status.originalScope,original);
  const wrong={...scope,worldToken:randomUUID()};assert.equal((await f.ok(0,{itemStatus:{operationId:o.operationId},currentScope:wrong},original)).itemOperation.effectAllowed,false);
  const stale=await f.raw(oldCredentials,f.body(0,{itemStatus:{operationId:o.operationId}},original,oldCredentials));assert.ok(stale.error||!stale.itemOperation.effectAllowed);
  const proof=receipt(o,'applied');assert.equal((await f.ok(0,{...proof,currentScope:scope},original)).itemOperation.state,'completed');
  f.x.advance(1000);const pending=(await f.ok(0,{itemDeposit:{operationId:randomUUID(),item:object()}})).itemOperation;
  await f.x.end();const historical=await f.raw(f.x.desktop,f.body(0,{itemStatus:{operationId:pending.operationId}},scope,f.x.desktop));assert.equal(historical.itemOperation.effectAllowed,false);
  assert.equal((await f.raw(f.x.desktop,f.body(0,receipt(pending,'escrowed'),scope,f.x.desktop))).error.code,'FORBIDDEN');
  await f.x.deleteLobby(scope.roomId);const cold=await f.x.signIn();
  for(const payload of [{itemStatus:{operationId:o.operationId}},{...proof,currentScope:scope},d.original])assert.equal((await f.raw(cold,f.body(0,payload,original,cold))).error.code,'ROOM_DELETED');
  const stats=await f.raw(cold,f.body(0,{statisticsList:{limit:32}},original,cold));assert.deepEqual(stats.statistics.map((v:any)=>v.kind).sort(),['item.claim','item.deposit']);
  const row=(await f.x.fixture.service.db.transaction(tx=>tx.query('SELECT (SELECT count(*)::int FROM g3_bank_items) AS items,(SELECT count(*)::int FROM g3_item_operations) AS operations,(SELECT count(*)::int FROM g3_item_receipts) AS receipts,(SELECT count(*)::int FROM g3_item_inventory) AS inventory,(SELECT count(*)::int FROM g3_outbox WHERE kind LIKE \'item.%\') AS outbox'))).rows[0];assert.deepEqual(row,{items:0,operations:0,receipts:0,inventory:0,outbox:0});
 }finally{await f.close();}
});

test('Item transaction rollback and immutable SQL guard retain originals, with reserved bank headroom before removal',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const d=await deposit(f,object()),db=f.x.fixture.service.db,{auth}=f.x.fixture.service,authority=new G3Authority(()=>db.epoch,auth.now);authority.admission=(tx,id)=>readRoomAdmissions(tx,db,f.x.fixture.profile.compatibility,auth.now(),id);
  const item=(await f.ok(1,{itemInspect:{itemId:d.o.itemId}})).itemDetail.summary,claim={itemClaim:{operationId:randomUUID(),itemId:item.itemId,expectedRevision:item.revision}},q=f.body(1,claim);
  await assert.rejects(db.transaction(async tx=>{const actor=await auth.authenticate(tx,f.values[1].credentials.accessToken);await authority.execute(tx,actor,q);throw new PolicyError('ITEM_ROLLBACK');}),/ITEM_ROLLBACK/);
  assert.equal((await f.ok(1,{itemInspect:{itemId:d.o.itemId}})).itemDetail.summary.state,'available');assert.equal((await f.ok(1,claim)).itemOperation.state,'reserved');
  await assert.rejects(db.transaction(tx=>tx.query('UPDATE g3_item_operations SET delivery_id=$1',[randomUUID()])));await assert.rejects(db.transaction(tx=>tx.query('DELETE FROM g3_item_operations')));await assert.rejects(db.transaction(tx=>tx.query('UPDATE g3_item_receipts SET outcome=\'uncertain\'')));
  // Synthetic owned-schema boundary: build rows using one real validated item.
  await db.transaction(tx=>tx.query("INSERT INTO g3_bank_items SELECT run_id,gen_random_uuid(),principal_id,sender_name,kind,descriptor,'available',revision,presentation,search_text,spell_type,wand_sort,created_at FROM g3_bank_items CROSS JOIN generate_series(1,2047) WHERE item_id=$1",[d.o.itemId]));
  assert.equal((await f.send(0,{itemDeposit:{operationId:randomUUID(),item:object()}})).error.code,'BANK_FULL');
  assert.equal((await db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_item_operations'))).rows[0].n,2);
 }finally{await f.close();}
});

test('Concurrent claims, immutable duplicates and ambiguous outcomes never release or duplicate an item',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const d=await deposit(f,object());
  assert.equal((await f.send(0,{itemDeposit:{...d.original.itemDeposit,item:examples.descriptors.spell}})).error.code,'CONFLICT');
  assert.equal((await f.send(0,{itemReceipt:{...d.proof.itemReceipt,evidenceDigest:'c'.repeat(64)}})).error.code,'CONFLICT');
  const revision=(await f.ok(0,{itemInspect:{itemId:d.o.itemId}})).itemDetail.summary.revision;
  const commands=[0,1].map(()=>({itemClaim:{operationId:randomUUID(),itemId:d.o.itemId,expectedRevision:revision}}));
  const results=await Promise.all(commands.map((p,i)=>f.send(i,p)));assert.equal(results.filter(v=>v.itemOperation).length,1);assert.equal(results.find(v=>v.error).error.code,'CLAIM_LOST');
  const i=results[0].itemOperation?0:1,o=results[i].itemOperation;
  assert.equal((await f.ok(i,commands[i]!)).itemOperation.deliveryId,o.deliveryId);
  assert.equal((await f.send(1-i,{itemStatus:{operationId:o.operationId}},f.scope(i))).error.code,'NOT_FOUND');
  assert.equal((await f.send(i,{itemStatus:{operationId:o.operationId}},{...f.scope(i),worldToken:randomUUID()})).error.code,'NOT_FOUND');
  assert.equal((await f.ok(i,receipt(o,'not_applied'))).itemOperation.state,'uncertain');
  const bank=(await f.ok(i,{itemBank:{}})).itemBank;assert.equal(bank.items[0].state,'claimed');assert.equal(bank.items[0].claimAllowed,false);assert.equal(bank.operations[0].effectAllowed,false);assert.equal(bank.operations[0].item,undefined);
  assert.equal((await f.ok(i,{itemStatus:{operationId:o.operationId}})).itemOperation.effectAllowed,false);
  f.x.advance(1000);const pending=(await f.ok(0,{itemDeposit:{operationId:randomUUID(),item:object()}})).itemOperation;
  assert.equal((await f.ok(0,receipt(pending,'not_applied'))).itemOperation.state,'cancelled');
  const ambiguous=(await f.ok(0,{itemDeposit:{operationId:randomUUID(),item:object()}})).itemOperation;
  assert.equal((await f.ok(0,receipt(ambiguous,'uncertain'))).itemOperation.effectAllowed,false);
  assert.equal((await f.ok(0,{itemBank:{}})).itemBank.total,1);
 }finally{await f.close();}
});

test('Lost committed item replies recover exact status and slow replies leave gold and movement usable',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const original={itemDeposit:{operationId:randomUUID(),item:object()}};
  f.x.faults.goldControl.arm({target:'consumer',operationId:original.itemDeposit.operationId,kind:'itemDeposit'});await assert.rejects(f.send(0,original));
  const o=(await f.ok(0,original)).itemOperation,p=receipt(o,'escrowed');
  f.x.faults.goldControl.arm({target:'consumer',operationId:o.operationId,kind:'itemReceipt'});await assert.rejects(f.send(0,p));
  assert.equal((await f.ok(0,{itemStatus:{operationId:o.operationId}})).itemOperation.state,'completed');assert.equal((await f.ok(0,p)).itemOperation.state,'completed');
  const entry=(await f.ok(1,{itemBank:{}})).itemBank.items[0],claim={itemClaim:{operationId:randomUUID(),itemId:entry.itemId,expectedRevision:entry.revision}};
  f.x.faults.goldControl.arm({target:'peer',operationId:claim.itemClaim.operationId,kind:'itemClaim',effect:'delay',delayMs:1000});const slow=f.ok(1,claim);
  assert.equal((await f.ok(0,{goldBank:{}})).bank.availableBalance,0);
  await f.ok(0,{movement:{frames:[{sequence:1,xMilli:'0',yMilli:'0',vxMilli:'0',vyMilli:'0',facingLeft:false}]}});
  const reserved=(await slow).itemOperation;assert.equal(reserved.state,'reserved');
  assert.equal((await f.ok(1,claim)).itemOperation.deliveryId,reserved.deliveryId);
  const counts=(await f.x.fixture.service.db.transaction(tx=>tx.query('SELECT (SELECT count(*)::int FROM g3_item_operations) AS operations,(SELECT count(*)::int FROM g3_item_receipts) AS receipts,(SELECT count(*)::int FROM g3_statistics) AS statistics'))).rows[0];assert.deepEqual(counts,{operations:2,receipts:1,statistics:1});
 }finally{await f.close();}
});

test('Each category policy gates deposits and claims while spells and gold stay independent',async()=>{
 const f=await b1Fixture({itemTransfers:true,sendWands:false,sendFlasks:false,sendItems:false});try{
  for(const item of [examples.descriptors.wand,examples.descriptors.container,examples.descriptors.emptyPowder,object()])assert.equal((await f.send(0,{itemDeposit:{operationId:randomUUID(),item}})).error.code,'POLICY_DISABLED');
  await deposit(f,examples.descriptors.spell);
  const gold=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:3}})).operation;assert.equal((await f.ok(0,f.receipt(gold,'escrowed'))).operation.balance,3);
  // Synthetic pre-existing item rows isolate the policy fence; production runs
  // freeze settings and cannot toggle policy after accepting a deposit.
  const db=f.x.fixture.service.db;await db.transaction(tx=>tx.query("UPDATE g3_bank_items SET kind='wand',descriptor=$1 WHERE run_id=$2",[normalizeItem(examples.descriptors.wand),f.scope(0).runId]));
  const bank=(await f.ok(1,{itemBank:{}})).itemBank;assert.equal(bank.items[0].claimAllowed,false);
  assert.equal((await f.send(1,{itemClaim:{operationId:randomUUID(),itemId:bank.items[0].itemId,expectedRevision:bank.items[0].revision}})).error.code,'POLICY_DISABLED');
 }finally{await f.close();}
 const old=await b1Fixture();try{assert.equal((await old.send(0,{itemBank:{}})).error.code,'FORBIDDEN');}finally{await old.close();}
});

test('Whole-bank literal search, exact page boundaries, spell filters and deterministic wand criteria',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  for(let n=1;n<=80;n++){
   const item=n===41?{...examples.descriptors.wand,wand:{...examples.descriptors.wand.wand,named:true,uiName:'Unique [100%] Wand'}}:object();await deposit(f,item);
   if([1,39,40,41,80].includes(n)){const b=(await f.ok(1,{itemBank:{page:999}})).itemBank;assert.equal(b.total,n);assert.equal(b.pageCount,Math.ceil(n/40));assert.equal(b.page,b.pageCount-1);assert.equal(b.items.length,n%40||40);}
  }
  const a=(await f.ok(1,{itemBank:{}})).itemBank,b=(await f.ok(1,{itemBank:{page:1}})).itemBank;assert.equal(new Set([...a.items,...b.items].map(v=>v.itemId)).size,80);
  assert.equal((await f.ok(1,{itemBank:{search:'[100%]'}})).itemBank.total,1);assert.equal((await f.ok(1,{itemBank:{search:'unique ['}})).itemBank.total,1);assert.equal((await f.ok(1,{itemBank:{search:'.*'}})).itemBank.total,0);
  const spells=[C.actions.find(v=>v.type===0),C.actions.find(v=>v.type===1)];for(const action of spells)await deposit(f,{catalogueRevision:C.revision,spell:{actionId:action.id,usesRemaining:'-1'}});
  assert.equal((await f.ok(1,{itemBank:{tab:'spells'}})).itemBank.total,2);assert.equal((await f.ok(1,{itemBank:{tab:'spells',useFilters:true,spellTypes:[]}})).itemBank.total,0);assert.equal((await f.ok(1,{itemBank:{tab:'spells',useFilters:true,spellTypes:[1]}})).itemBank.total,1);
  for(const [shuffle,casts]of [[true,'1'],[false,'9'],[false,'2']] as const)await deposit(f,{...examples.descriptors.wand,wand:{...examples.descriptors.wand.wand,shuffleDeckWhenEmpty:shuffle,actionsPerRound:casts}});
  const w=(await f.ok(1,{itemBank:{tab:'wands',useFilters:true,wandSort:['actionsPerRound','shuffleDeckWhenEmpty']}})).itemBank;
  const details=await Promise.all(w.items.map(async(v:any)=>(await f.ok(1,{itemInspect:{itemId:v.itemId}})).itemDetail.item.wand));
  assert.equal(details.at(-1).shuffleDeckWhenEmpty,true);assert.deepEqual(w.items.map((v:any)=>v.itemId),(await f.ok(1,{itemBank:{tab:'wands',useFilters:true,wandSort:['shuffleDeckWhenEmpty','actionsPerRound']}})).itemBank.items.map((v:any)=>v.itemId));
  assert.equal((await f.send(1,{itemBank:{search:'x'.repeat(33)}})).error.code,'INVALID_MESSAGE');assert.equal((await f.send(1,{itemBank:{useFilters:true,wandSort:['arbitrary']}})).error.code,'INVALID_MESSAGE');
 }finally{await f.close();}
});

test('Read-only inventory uses changed revisions, bounded slot pages, explicit clearing and no transfer rights',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const target=f.values[0].credentials.sessionId;
  assert.equal((await f.ok(1,{inventoryInspect:{gameSessionId:target}})).inventory.state,'unknown');
  const snapshot={inventoryPublish:{revision:1,heldSlot:'3',wands:[{slot:1,wand:examples.descriptors.wand.wand},{slot:3,unavailableReason:'unsupported-content'}]}};
  await f.ok(0,snapshot);await f.ok(0,snapshot);
  assert.equal((await f.send(0,{inventoryPublish:{...snapshot.inventoryPublish,heldSlot:'1'}})).error.code,'CONFLICT');
  const first=(await f.ok(1,{inventoryInspect:{gameSessionId:target}})).inventory;assert.equal(first.totalSlots,2);assert.equal(first.wands.length,1);assert.equal(first.nextSlot,'3');assert.equal(first.heldWandSpriteId,'');assert.deepEqual(first.wands[0].wand,normalizeItem(examples.descriptors.wand).wand);
  const second=(await f.ok(1,{inventoryInspect:{gameSessionId:target,slot:3}})).inventory;assert.equal(second.wands[0].unavailableReason,'unsupported-content');assert.equal(second.nextSlot,'');
  await f.ok(0,{inventoryPublish:{revision:2,heldSlot:'1',wands:[snapshot.inventoryPublish.wands[0]!]}});
  assert.equal((await f.ok(1,{inventoryInspect:{gameSessionId:target}})).inventory.heldWandSpriteId,examples.descriptors.wand.wand.spriteId);
  const roster=JSON.parse((await f.x.fixture.call('/v3/social/rpc',{...f.body(1,{}),revision:socialSchema.revision,roster:{}},f.values[1].credentials.accessToken)).body);assert.equal(roster.error,undefined);const player=roster.players.find((p:any)=>p.gameSessionId===target);assert.equal(player.inventoryRevision,2);assert.equal(player.heldWandSpriteId,examples.descriptors.wand.wand.spriteId);assert.equal(player.wands,undefined);
  assert.equal((await f.send(0,snapshot)).error.code,'STALE_EPOCH');
  await f.ok(0,{inventoryPublish:{revision:3,heldSlot:'-1',wands:[]}});assert.equal((await f.ok(1,{inventoryInspect:{gameSessionId:target}})).inventory.state,'empty');
  assert.equal((await f.ok(1,{inventoryInspect:{gameSessionId:randomUUID()}})).inventory.state,'removed');
  assert.equal((await f.ok(1,{itemBank:{}})).itemBank.total,0);
 }finally{await f.close();}
});

test('Large wand originals stay individual; pending bank/outbox pages omit descriptors and enforce queue bounds',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const card=(slot:number)=>({slot,spell:{actionId:C.actions[0].id,usesRemaining:'-1'}}),item={...examples.descriptors.wand,wand:{...examples.descriptors.wand.wand,deck:Array.from({length:1024},(_,i)=>card(i)),permanent:Array.from({length:256},(_,i)=>card(i))}};
  assert.ok(Buffer.byteLength(JSON.stringify(item))>65536);const ids:string[]=[];
  for(let i=0;i<9;i++){f.x.advance(100);ids.push((await f.ok(0,{itemDeposit:{operationId:randomUUID(),item}})).itemOperation.operationId);}
  const a=(await f.ok(0,{itemBank:{}})).itemBank,b=(await f.ok(0,{itemBank:{afterOperationId:a.nextOperationId}})).itemBank;assert.equal(a.operations.length,8);assert.equal(b.operations.length,1);assert.equal(b.nextOperationId,'');assert.ok(a.operations.every((o:any)=>!o.item));assert.ok(Buffer.byteLength(JSON.stringify(a))<16384);
  const status=(await f.ok(0,{itemStatus:{operationId:ids[0]!}})).itemOperation;assert.deepEqual(status.item,normalizeItem(item));assert.equal(status.effectAllowed,true);
  const outbox=(await f.ok(0,{outboxList:{limit:32}})).outbox;assert.ok(outbox.every((v:any)=>!v.itemOperation.item&&v.itemOperation.effectAllowed===false));
  // Synthetic owned-schema pending-cap boundary, retaining real FK identities.
  await f.x.fixture.service.db.transaction(tx=>tx.query("INSERT INTO g3_item_operations SELECT run_id,principal_id,gen_random_uuid(),item_id,original_scope,direction,payload_digest,gen_random_uuid(),state,local_outcome,created_at FROM g3_item_operations CROSS JOIN generate_series(1,247) WHERE operation_id=$1",[ids[0]]));
  assert.equal((await f.send(0,{itemDeposit:{operationId:randomUUID(),item:object()}})).error.code,'CAPACITY');
 }finally{await f.close();}
});

test('Deleting one lobby purges all item categories across runs while another lobby and statistics survive',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{
  const all=[examples.descriptors.spell,examples.descriptors.wand,examples.descriptors.container,examples.descriptors.emptyPowder,object()],first=[];
  for(const item of all)first.push(await deposit(f,item));const room=f.scope(0).roomId!;await f.x.end();
  const enter=async(fresh=false)=>{await f.x.rpc(f.x.desktop,{roomJoin:{}});const q=f.x.request();if(fresh){const p=await f.x.prepareWorld();q.reconcile!.association=p.association;q.reconcile!.proof=p.proof;q.reconcile!.observation!.worldToken=p.association.worldId!;}await f.x.send(f.x.desktop,q);await f.x.start({selectedWorldIds:[q.reconcile!.association!.worldId!],excludedDesktopIds:[]});q.reconcile!.observation!.protected=false;q.reconcile!.observation!.observationRevision=2;f.values[0]=await f.x.send(f.x.desktop,q);};
  await f.x.nextRun();await enter(true);for(const item of all)await deposit(f,item);await f.x.end();
  const before=await f.x.lobbyTransferSummary(room);assert.equal(before.runs,2);assert.equal(before.bankItems,10);assert.equal(before.itemOperations,10);assert.equal(before.statistics,10);
  await f.x.createLobby();await enter();await deposit(f,object());const other=f.scope(0).roomId!,preserved=await f.x.lobbyTransferSummary(other);
  await f.x.deleteLobby(room);const after=await f.x.lobbyTransferSummary(room);for(const key of ['bankItems','itemOperations','itemReceipts','itemInventories','itemOutbox'])assert.equal(after[key],0);assert.equal(after.statistics,10);assert.deepEqual(await f.x.lobbyTransferSummary(other),preserved);
  assert.equal((await f.ok(0,{itemBank:{}})).itemBank.total,1);
 }finally{await f.close();}
});



