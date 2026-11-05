import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';import {G3Authority} from '../src/g3/authority.js';
const frame=(sequence:number)=>({sequence,xMilli:String(-sequence*1000),yMilli:'2000',vxMilli:'-1000',vyMilli:'0',facingLeft:true});
async function setup(){let now=Date.now();const f=await startG3Fixture({tls:true,now:()=>now}),s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=25;}});await s.start();await s.playing();now+=1000;
 const call=async(i:number,payload:object,scope=s.scopes[i]!)=>{const c=s.games[i]!;const r=await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope,...payload},c.accessToken);assert.equal(r.status,200);return JSON.parse(r.body);};
 return {f,s,call,get now(){return now;},advance(ms:number){now+=ms;}};
}
test('A1 actual TLS latest-batch burst/gaps preserve sequence order and stable service timestamps; duplicate/invalid/rate refusals do not refresh ghosts',async()=>{
 const t=await setup();try{
  let high=0;for(let n=0;n<10;n++){const frames=[high+1,high+2,high+4,high+7].map(frame);high+=7;assert.equal((await t.call(0,{movement:{frames}})).error,undefined);}
  const first=(await t.call(1,{ghostList:{}})).ghosts[0];assert.deepEqual(first.frames.map((v:any)=>v.sequence),[64,65,67,70]);assert.equal(first.principalId,t.s.desktops[0]!.accountId);assert.equal(first.gameSessionId,t.s.games[0]!.sessionId);assert.equal(first.capturedAt,new Date(t.now).toISOString());
  assert.equal((await t.call(0,{movement:{frames:[frame(high)]}})).error.code,'INVALID_MESSAGE');assert.equal((await t.call(0,{movement:{frames:[frame(high+1)]}})).error.code,'CAPACITY');
  t.advance(1000);for(const frames of [[frame(71),frame(70)],[frame(71),{...frame(72),xMilli:'-0'}],[]])assert.equal((await t.call(0,{movement:{frames}})).error.code,'INVALID_MESSAGE');
  assert.deepEqual((await t.call(1,{ghostList:{}})).ghosts[0],first);
  assert.equal((await t.call(0,{movement:{frames:[frame(71)]}})).error,undefined);const next=(await t.call(1,{ghostList:{}})).ghosts[0];assert.equal(next.frames.length,1);assert.equal(next.frames[0].sequence,71);assert.notEqual(next.capturedAt,first.capturedAt);
  t.advance(1999);assert.deepEqual((await t.call(1,{ghostList:{}})).ghosts[0],next);t.advance(1);assert.deepEqual((await t.call(1,{ghostList:{}})).ghosts,[]);
  assert.equal((await t.call(0,{movement:{frames:[frame(71)]}})).error.code,'INVALID_MESSAGE');assert.equal((await t.call(0,{movement:{frames:[frame(1000)]}})).error,undefined);
 }finally{await t.f.close();}
});
test('A1 authenticated actor/room/run/attachment/world isolation, source protection and terminal End reject stale publication and remove peers',async()=>{
 const t=await setup();try{
  assert.equal((await t.call(0,{movement:{frames:[frame(1)]}})).error,undefined);
  for(const patch of [{roomId:randomUUID()},{runId:randomUUID()},{runEpoch:randomUUID()},{configRevision:2},{attachmentId:t.s.scopes[1]!.attachmentId!,worldToken:t.s.scopes[1]!.worldToken!},{worldToken:'foreign-world'}])assert.ok((await t.call(0,{movement:{frames:[frame(2)]}},{...t.s.scopes[0]!,...patch})).error);
  assert.equal((await t.call(1,{ghostList:{}})).ghosts.length,1);assert.deepEqual((await t.call(0,{ghostList:{}})).ghosts,[]);
  await t.s.rpc(t.s.games[0]!,{gameObservation:{...t.s.observations[0]!,observationRevision:3,protected:true}});assert.ok((await t.call(0,{movement:{frames:[frame(2)]}})).error);assert.deepEqual((await t.call(1,{ghostList:{}})).ghosts,[]);
  await t.s.rpc(t.s.host,{endRun:{}});assert.ok((await t.call(1,{ghostList:{}})).error);assert.ok((await t.call(0,{movement:{frames:[frame(3)]}})).error);
  await t.f.restart();assert.ok((await t.call(0,{movement:{frames:[frame(4)]}})).error);
 }finally{await t.f.close();}
});
test('A1 source lease expiry and desktop revocation remove peers; refused running-room leave is not an authority-removal signal',async()=>{
 for(const terminal of ['expiry','logout']){const t=await setup();try{
  assert.equal((await t.call(0,{movement:{frames:[frame(1)]}})).error,undefined);assert.equal((await t.call(1,{ghostList:{}})).ghosts.length,1);
  if(terminal==='logout'){await assert.rejects(t.s.rpc(t.s.desktops[0]!,{roomLeave:{}}),/INVALID_STATE/);assert.equal((await t.call(1,{ghostList:{}})).ghosts.length,1);await t.s.rpc(t.s.desktops[0]!,{logout:{}});}else{await t.f.service.db.pool.query('UPDATE attachments SET presence_until=$2 WHERE id=$1',[t.s.scopes[0]!.attachmentId,new Date(t.now+100)]);t.advance(100);}
  const result=await t.call(1,{ghostList:{}});assert.ok(result.error||result.ghosts.length===0);assert.ok((await t.call(0,{movement:{frames:[frame(2)]}})).error);
 }finally{await t.f.close();}}
});
test('A1 delayed post-commit callbacks cannot replace a newer batch',async()=>{
 const t=await setup();try{
  await t.call(1,{ghostList:{}});const db=t.f.service.db,authority=new G3Authority(()=>db.epoch,()=>t.now),actors=await Promise.all(t.s.games.map(c=>db.transaction(tx=>t.f.service.auth.authenticate(tx,c.accessToken!))));
  const submit=async(sequence:number)=>{const effects:(()=>void)[]=[];await db.transaction(tx=>authority.execute(tx,actors[0]!,{scope:t.s.scopes[0]!,movement:{frames:[frame(sequence)]}},effects));return effects;};
  const older=await submit(1);t.advance(50);const newer=await submit(2);newer.forEach(fn=>fn());older.forEach(fn=>fn());
  const read=()=>db.transaction(tx=>authority.execute(tx,actors[1]!,{scope:t.s.scopes[1]!,ghostList:{}}));assert.equal((await read()).ghosts![0]!.frames![0]!.sequence,2);
 }finally{await t.f.close();}
});
test('A1 a delayed committed publication cannot extend the lifetime of already-old samples',async()=>{
 const t=await setup();try{
  await t.call(1,{ghostList:{}});const db=t.f.service.db,authority=new G3Authority(()=>db.epoch,()=>t.now),actors=await Promise.all(t.s.games.map(c=>db.transaction(tx=>t.f.service.auth.authenticate(tx,c.accessToken!)))),effects:(()=>void)[]=[];
  await db.transaction(tx=>authority.execute(tx,actors[0]!,{scope:t.s.scopes[0]!,movement:{frames:[frame(1)]}},effects));t.advance(2000);effects.forEach(fn=>fn());
  assert.deepEqual((await db.transaction(tx=>authority.execute(tx,actors[1]!,{scope:t.s.scopes[1]!,ghostList:{}}))).ghosts,[]);
 }finally{await t.f.close();}
});
test('A1 route work has bounded immediate backpressure rather than an unbounded movement request queue',async()=>{
 const t=await setup(),db=t.f.service.db,original=(db as any).transaction.bind(db);let release!:()=>void,entered!:()=>void,held=0;
 const hold=new Promise<void>(resolve=>{release=resolve;}),full=new Promise<void>(resolve=>{entered=resolve;});
 try{
  // Schedule only: authentication and all SQL remain the real service's work.
  (db as any).transaction=async(...args:any[])=>{if(++held<=16){if(held===1)entered();await hold;}return original(...args);};
  const calls=Array.from({length:18},()=>t.f.service.app.inject({method:'POST',url:'/v3/rpc',headers:{host:new URL(t.f.origin).host,authorization:'Bearer '+t.s.games[0]!.accessToken},payload:{major:1,revision:'g3-draft-1',serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:t.s.games[0]!.sessionEpoch,requestId:randomUUID(),scope:t.s.scopes[0],ghostList:{}}}));
  let rejected=0,limited!:()=>void;const capacity=new Promise<void>(resolve=>{limited=resolve;}),results=calls.map(async call=>{const answer=(await call).json();if(answer.error?.code==='CAPACITY'&&++rejected===2)limited();return answer;});let timer:ReturnType<typeof setTimeout>|undefined;try{await Promise.race([Promise.all([full,capacity]),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Immediate finite request gate rejection was not reached')),2500);})]);}finally{clearTimeout(timer);}
  // Candidate sweep coalesces its SQL, so one held transaction may hold all16
  // route slots. Excess routes must refuse without waiting for that SQL.
  assert.ok(held>=1&&held<=16);release();const answers=await Promise.all(results);assert.ok(answers.filter(r=>r.error?.code==='CAPACITY').length>=2);assert.ok(answers.some(r=>!r.error));
 }finally{release();(db as any).transaction=original;await t.f.close();}
});


