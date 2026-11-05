import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,randomBytes,generateKeyPairSync} from 'node:crypto';
import pg from 'pg';
import {createG3App} from '../src/g3/app.js';
import {schema} from '../../../packages/contracts/src/g3.js';
import type {Request,Response,Scope,Operation} from '../../../packages/contracts/g3/g3.js';
import type {Credentials} from '../../../packages/contracts/g1/g1.js';
import {G3Authority} from '../src/g3/authority.js';
import type {Actor} from '../src/g1/auth.js';
const url=process.env.NT2_G2_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
async function fixture(){
 const namespace='g3_test_'+randomUUID().replaceAll('-',''),pair=generateKeyPairSync('ed25519');let now=Date.now();const options={databaseUrl:url,namespace,serviceId:'nt2-g3-fixture',mockEnabled:true,now:()=>now,keys:{privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}).toString(),publicKey:pair.publicKey.export({type:'spki',format:'pem'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:'http://127.0.0.1:4360'}};let service=await createG3App(options);
 const runId=randomUUID(),roomId=randomUUID(),runEpoch=randomUUID();let owner='';
 async function account(name:string,admitted=true){const id=randomUUID(),device=randomUUID(),parent=await service.db.transaction(async tx=>{await tx.query('INSERT INTO accounts(id,provider_namespace,provider_subject) VALUES($1,$2,$3)',[id,'g3-fixture',device]);return service.auth.family(tx,id,device,'desktop');});if(!owner){owner=id;await service.db.transaction(tx=>tx.query("INSERT INTO g3_runs VALUES($1,$2,$3,1,$4,'running','core')",[runId,roomId,runEpoch,id]));}
  const attachmentId=randomUUID(),worldToken=randomUUID(),scope:Scope={roomId,runId,runEpoch,configRevision:1,attachmentId,worldToken};const game=await service.db.transaction(async tx=>{const actor=await service.auth.authenticate(tx,parent.accessToken!);const sid=randomUUID(),epoch=randomUUID();await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) VALUES($1,$2,'game',$3,$4)",[sid,actor.familyId,epoch,actor.sessionId]);if(admitted){await tx.query('INSERT INTO g3_access VALUES($1,$2,$3)',[runId,id,name]);await tx.query('INSERT INTO g3_contexts(attachment_id,run_id,principal_id,game_session_id,session_epoch,service_epoch,world_token,capabilities,presence_until) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)',[attachmentId,runId,id,sid,epoch,service.db.epoch,worldToken,['core.gold','core.ghost','core.progress'],new Date(now+60000)]);}return service.auth.mint(tx,id,device,'game',actor.familyId,sid,epoch,attachmentId);});return {id,parent,game,scope};}
 async function send(c:Credentials,scope:Scope,body:Partial<Request>){const request:Request={major:1,revision:schema.revision,serviceId:service.db.serviceId,serviceEpoch:service.db.epoch,sessionEpoch:c.sessionEpoch!,requestId:randomUUID(),scope,...body};const reply=await service.app.inject({method:'POST',url:'/v3/rpc',headers:{authorization:'Bearer '+c.accessToken},payload:request});return reply.json() as Response;}
 const a=await account('owner'),b=await account('"><svg onload=alert(1)><script>bad</script>');
 const ok=async(c:Credentials,scope:Scope,body:Partial<Request>)=>{const r=await send(c,scope,body);assert.equal(r.error,undefined,r.error?.code);return r;};
 const receipt=(o:Operation,outcome:string)=>({goldReceipt:{operationId:o.operationId!,deliveryId:o.deliveryId!,payloadDigest:o.payloadDigest!,receiptId:randomUUID(),outcome,evidenceDigest:'a'.repeat(64)}});
 return {a,b,account,send,ok,receipt,runId,get now(){return now;},get service(){return service;},advance(ms:number){now+=ms;},async restart(){await service.app.close();service=await createG3App(options);},async close(){await service.app.close();const pool=new pg.Pool({connectionString:url});try{await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);}finally{await pool.end();}}};
}

test('G3 expires 1025 successive poses without capacity exhaustion and keeps live-world fences outside the cache',async()=>{
 const f=await fixture();try{
  const actor=await f.service.db.transaction(tx=>f.service.auth.authenticate(tx,f.a.game.accessToken!));
  const attachments=Array.from({length:1025},()=>randomUUID()),sessions=attachments.map(()=>randomUUID()),epochs=attachments.map(()=>randomUUID());
  await f.service.db.transaction(async tx=>{
   await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) SELECT sid,$1,'game',epoch,$2 FROM unnest($3::uuid[],$4::uuid[]) x(sid,epoch)",[actor.familyId,actor.parentId,sessions,epochs]);
   await tx.query('INSERT INTO g3_contexts(attachment_id,run_id,principal_id,game_session_id,session_epoch,service_epoch,world_token,capabilities,presence_until) SELECT attachment,$1,$2,sid,epoch,$3,attachment::text,$4,$5 FROM unnest($6::uuid[],$7::uuid[],$8::uuid[]) x(attachment,sid,epoch)',[f.runId,actor.accountId,f.service.db.epoch,['core.ghost'],new Date(f.now+86400000),attachments,sessions,epochs]);
  });
  const authority=new G3Authority(()=>f.service.db.epoch,()=>f.now),frame={sequence:1,xMilli:'0',yMilli:'0',vxMilli:'0',vyMilli:'0',facingLeft:false};
  const submit=async(index:number,sequence=1)=>{
   const c:Actor={...actor,sessionId:sessions[index]!,sessionEpoch:epochs[index]!},scope={...f.a.scope,attachmentId:attachments[index]!,worldToken:attachments[index]!},effects:(()=>void)[]=[];
   await f.service.db.transaction(tx=>authority.execute(tx,c,{scope,movement:{frames:[{...frame,sequence}]}},effects));for(const effect of effects)effect();
  };
  for(let i=0;i<1025;i++){f.advance(3000);await submit(i);}
  await assert.rejects(submit(1024),/INVALID_MESSAGE/);await submit(1024,2);
  // Recreating the display cache cannot reset an authenticated world's sequence.
  const rebuilt=new G3Authority(()=>f.service.db.epoch,()=>f.now);
  const c={...actor,sessionId:sessions[1024]!,sessionEpoch:epochs[1024]!},scope={...f.a.scope,attachmentId:attachments[1024]!,worldToken:attachments[1024]!};
  await assert.rejects(f.service.db.transaction(tx=>rebuilt.execute(tx,c,{scope,movement:{frames:[frame]}})),/INVALID_MESSAGE/);
  await f.service.db.transaction(tx=>tx.query('UPDATE g3_contexts SET revoked_at=$2 WHERE attachment_id=$1',[attachments[1024],new Date(f.now)]));
  await assert.rejects(submit(1024,3),/STALE_EPOCH/);
 }finally{await f.close();}
});
test('G3 ledger/outbox atomically handles concurrent withdrawal, duplicate/lost replies and uncertain outcomes',async()=>{
 const f=await fixture();try{
  const {a,b}=f,deposit={goldIntent:{operationId:randomUUID(),direction:'deposit',amount:10}};const staged=(await f.ok(a.game,a.scope,deposit)).operation!;assert.equal(staged.state,'awaiting_local');assert.equal(staged.balance,0);const proof=f.receipt(staged,'escrowed');assert.equal((await f.ok(a.game,a.scope,proof)).operation!.balance,10);assert.equal((await f.ok(a.game,a.scope,proof)).operation!.balance,10);assert.equal((await f.ok(a.game,a.scope,deposit)).operation!.deliveryId,staged.deliveryId);
  assert.equal((await f.send(a.game,a.scope,{goldIntent:{...deposit.goldIntent,amount:11}})).error?.code,'CONFLICT');
  const results=await Promise.all([f.send(a.game,a.scope,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:10}}),f.send(b.game,b.scope,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:10}})]);assert.equal(results.filter(v=>!v.error).length,1);assert.equal(results.find(v=>v.error)!.error!.code,'INSUFFICIENT_FUNDS');const winner=results[0]!.operation?a:b,withdraw=results.find(v=>v.operation)!.operation!;assert.equal(withdraw.balance,0);assert.equal(withdraw.state,'reserved');
  const uncertain=f.receipt(withdraw,'uncertain');assert.equal((await f.ok(winner.game,winner.scope,uncertain)).operation!.state,'uncertain');assert.equal((await f.ok(winner.game,winner.scope,uncertain)).operation!.balance,0);assert.equal((await f.ok(winner.game,winner.scope,f.receipt(withdraw,'not_applied'))).operation!.state,'uncertain');assert.equal((await f.ok(winner.game,winner.scope,f.receipt(withdraw,'applied'))).operation!.state,'completed');
  const ledger=await f.service.db.transaction(tx=>tx.query('SELECT sum(delta)::integer AS total,count(*)::integer AS n FROM g3_ledger'));assert.equal(ledger.rows[0].total,0);assert.equal(ledger.rows[0].n,2);await assert.rejects(f.service.db.transaction(tx=>tx.query('UPDATE g3_ledger SET balance=999')));
  const outbox=await f.ok(a.parent,a.scope,{outboxList:{limit:32}});assert.ok(outbox.outbox!.length>=2);assert.equal((await f.send(b.parent,b.scope,{goldStatus:{operationId:staged.operationId!}})).error?.code,'NOT_FOUND');
  await f.restart();assert.equal((await f.ok(a.parent,a.scope,{goldStatus:{operationId:staged.operationId!}})).operation!.state,'completed');assert.equal((await f.ok(a.parent,a.scope,deposit)).operation!.deliveryId,staged.deliveryId);assert.ok((await f.send(a.game,a.scope,{goldStatus:{operationId:staged.operationId!}})).error);
 }finally{await f.close();}
});
test('G3 movement bounds, immutable sender, monotonic expiry and registered event provenance are scoped',async()=>{
 const f=await fixture();try{const {a,b}=f,frame={sequence:1,xMilli:'-100',yMilli:'200',vxMilli:'0',vyMilli:'0',facingLeft:false};await f.ok(a.game,a.scope,{movement:{frames:[frame]}});let ghosts=(await f.ok(b.game,b.scope,{ghostList:{}})).ghosts!;assert.equal(ghosts[0]!.principalId,a.id);assert.equal(ghosts[0]!.worldToken,a.scope.worldToken);f.advance(2001);assert.equal((await f.ok(b.game,b.scope,{ghostList:{}})).ghosts!.length,0);assert.equal((await f.send(a.game,a.scope,{movement:{frames:[frame]}})).error?.code,'INVALID_MESSAGE');
  for(const xMilli of ['NaN','1.5','-0','2147483648'])assert.ok((await f.send(a.game,a.scope,{movement:{frames:[{...frame,sequence:2,xMilli}]}})).error);
  assert.equal((await f.send(a.game,{...a.scope,worldToken:'different'},{movement:{frames:[{...frame,sequence:2}]}})).error?.code,'STALE_EPOCH');
  const event={gameEvent:{eventId:randomUUID(),catalogId:'player.death',count:1}};await f.ok(a.game,a.scope,event);await f.ok(a.game,a.scope,event);assert.equal((await f.send(a.game,a.scope,{gameEvent:{...event.gameEvent,catalogId:'../../entity.xml'}})).error?.code,'FORBIDDEN');assert.equal((await f.send(a.game,a.scope,{gameEvent:{...event.gameEvent,targetGameSessionId:b.game.sessionId!}})).error?.code,'FORBIDDEN');const emitted=(await f.ok(b.parent,b.scope,{outboxList:{limit:32}})).outbox!.find(v=>v.event)!;assert.equal(emitted.event!.principalId,a.id);assert.equal(emitted.event!.targetGameSessionId,a.game.sessionId);assert.equal(emitted.event!.provenance,'validated-client-report');
 }finally{await f.close();}
});
test('G3 statistics survive restart, reject guessed private IDs, page stably and obey deletion/retention',async()=>{
 const f=await fixture();try{const {a,b}=f;for(let i=0;i<3;i++)await f.ok(b.game,b.scope,{gameEvent:{eventId:randomUUID(),catalogId:'player.win',count:1}});const first=await f.ok(a.parent,a.scope,{statisticsList:{limit:2}});assert.equal(first.statistics!.length,2);assert.match(first.statistics![0]!.displayName!,/<script>/);const next=await f.ok(a.parent,a.scope,{statisticsList:{limit:2,afterSequence:first.nextSequence!}});assert.equal(next.statistics!.length,1);assert.notEqual(next.statistics![0]!.sequence,first.statistics![0]!.sequence);
  const outsider=await f.account('outsider',false);for(const command of [{statisticsList:{}},{statisticsAggregate:{}},{statisticsDelete:{}}])assert.equal((await f.send(outsider.parent,a.scope,command)).error?.code,'NOT_FOUND');assert.equal((await f.send(b.parent,b.scope,{statisticsDelete:{}})).error?.code,'FORBIDDEN');assert.equal((await f.send(a.parent,a.scope,{statisticsList:{limit:33}})).error?.code,'INVALID_MESSAGE');
  await f.restart();assert.equal((await f.ok(a.parent,a.scope,{statisticsAggregate:{}})).aggregates![0]!.count,'3');await f.ok(a.parent,a.scope,{statisticsDelete:{}});assert.equal((await f.ok(a.parent,a.scope,{statisticsList:{}})).statistics!.length,0);
  await f.service.db.transaction(tx=>tx.query("INSERT INTO g3_statistics(run_id,principal_id,display_name,version,kind,count,provenance,captured_at,expires_at) VALUES($1,$2,'retention',1,'player.win',1,'validated-client-report',now(),now()-interval '1 day')",[f.runId,a.id]));await f.service.db.transaction(tx=>f.service.authority.retainStatistics(tx));assert.equal((await f.ok(a.parent,a.scope,{statisticsList:{}})).statistics!.length,0);
 }finally{await f.close();}
});
