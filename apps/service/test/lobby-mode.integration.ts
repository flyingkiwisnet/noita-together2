import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {Rooms} from '../src/g1/rooms.js';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';

test('Base configuration path fixes every lobby type at creation and preserves same-type edits',async()=>{
 const x=await startLifecycleLc1Fixture({race:true,b1Gold:true});try{
  const {db,auth}=x.fixture.service,compatibility={...x.fixture.profile.compatibility,modes:['core','nemesis','race']};
  const rooms=new Rooms(db,auth,compatibility,true);
  const call=(payload:any,r?:any)=>db.transaction(async tx=>{
   const actor=await auth.authenticate(tx,x.host.accessToken!);
   return rooms.execute(tx,{major:1,serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:actor.sessionEpoch,requestId:randomUUID(),...(r?{roomId:r.roomId,configRevision:r.configRevision,expectedRevision:r.revision}:{}),...payload},actor);
  });
  for(const mode of compatibility.modes){
   let r=(await call({roomCreate:{name:mode+' immutable',config:{seed:1,mode,requiredCapabilities:['core.seed']}}})).room!;
   for(const target of compatibility.modes.filter((m:string)=>m!==mode)){
    const before=await db.transaction(async tx=>(await tx.query('SELECT * FROM rooms WHERE id=$1',[r.roomId])).rows[0]);
    await assert.rejects(call({configure:{...r.config,mode:target,seed:2}},r),e=>(e as any).code==='INVALID_STATE');
    const after=await db.transaction(async tx=>(await tx.query('SELECT * FROM rooms WHERE id=$1',[r.roomId])).rows[0]);assert.deepEqual(after,before);
   }
   r=(await call({configure:{...r.config,seed:3}},r)).room!;assert.equal(r.config!.mode,mode);assert.equal(r.config!.seed,3);
  }
 }finally{await x.close();}
});

test('Current HTTP configuration refuses host-admin type changes before Start and after End without effects',async()=>{
 const x=await startLifecycleLc1Fixture({race:true,b1Gold:true});try{
  await x.fixture.service.db.transaction(tx=>tx.query("UPDATE accounts SET role='admin',role_revision=role_revision+1 WHERE id=$1",[x.host.accountId]));
  const config=structuredClone(x.room.config);
  const state=()=>x.fixture.service.db.transaction(async tx=>({
   room:(await tx.query('SELECT * FROM rooms WHERE id=$1',[x.room.roomId])).rows[0],
   runs:(await tx.query('SELECT * FROM runs WHERE room_id=$1 ORDER BY id',[x.room.roomId])).rows,
   memberships:(await tx.query('SELECT * FROM memberships WHERE room_id=$1 ORDER BY desktop_id',[x.room.roomId])).rows,
   preparations:(await tx.query('SELECT * FROM preparations WHERE run_id IN (SELECT id FROM runs WHERE room_id=$1) ORDER BY id',[x.room.roomId])).rows,
   attachments:(await tx.query('SELECT * FROM attachments WHERE run_id IN (SELECT id FROM runs WHERE room_id=$1) ORDER BY id',[x.room.roomId])).rows,
   receipts:(await tx.query('SELECT count(*) FROM command_results')).rows[0].count
  }));
  for(const mode of ['core','race']){
   const {race,...base}=config,next=mode==='race'?config:{...base,mode,requiredCapabilities:base.requiredCapabilities.filter((c:string)=>c!=='race.goals.v1')};
   await x.rpc(x.host,{roomCreate:{name:mode+' current immutable',visibility:'public',config:next}});assert.equal(x.room.state,'staging');
   const refusal=async()=>{const before=await state();for(const target of ['core','nemesis','race'].filter(m=>m!==mode))await assert.rejects(x.rpc(x.host,{configure:{...x.room.config,mode:target}}),/INVALID_STATE/);assert.deepEqual(await state(),before);};
   await refusal();
   await x.rpc(x.host,{configure:{...x.room.config,seed:101}});assert.equal(x.room.config.seed,101);assert.equal(x.room.config.mode,mode);assert.equal(x.room.state,'staging');
   await refusal();
   await x.rpc(x.host,{configure:{...x.room.config,seed:102}});assert.equal(x.room.config.seed,102);assert.equal(x.room.config.mode,mode);
   await x.end();assert.equal(x.room.state,'ended');await refusal();
   await x.rpc(x.host,{configure:{...x.room.config,seed:103}});assert.equal(x.room.config.seed,103);assert.equal(x.room.config.mode,mode);
  }
 }finally{await x.close();}
});
