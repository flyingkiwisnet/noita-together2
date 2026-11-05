// Real item authority and existing query path, explicit memory rows; no SQL run.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID}from'node:crypto';import{G3Authority}from'../src/g3/authority.js';import{LifecycleGoldControls}from'../scripts/lc1-gold-controls.js';import{ITEM_CATALOGUE as C}from'../../../packages/domain/src/g3-items.js';
test('Fixture claim diagnosis observes exact locked row and refusal without extra queries or altered result',async()=>{
 const scope={roomId:randomUUID(),runId:randomUUID(),runEpoch:randomUUID(),attachmentId:randomUUID(),configRevision:1,worldToken:randomUUID()},principal=randomUUID(),epoch=randomUUID(),sessionEpoch=randomUUID(),actor:any={kind:'game',accountId:principal,sessionId:randomUUID(),parentId:randomUUID(),sessionEpoch},hooks:any[]=[];
 const controls=new LifecycleGoldControls();controls.bind('peer',principal,scope.roomId);controls.install({app:{addHook:(name:string,hook:any)=>{if(name==='preClose')hooks.push(hook);}}});
 const authority=new G3Authority(()=>epoch,()=>1000);
 try{for(const scenario of [{state:'available',revision:'3',outcome:'CLAIM_LOST'},{state:'claimed',revision:'2',outcome:'CLAIM_LOST'},{missing:true,outcome:'NOT_FOUND'},{capacity:true,outcome:'CAPACITY'}]){
  let queries=0,locked=0;const query=async(sql:string)=>{queries++;
   if(sql.startsWith('SELECT r.*,a.display_name'))return{rows:[{id:scope.runId,room_id:scope.roomId,epoch:scope.runEpoch,config_revision:1,state:'running',item_revision:'2'}]};
   if(sql.startsWith('SELECT o.*,i.descriptor'))return{rows:[]};
   if(sql.startsWith('SELECT c.*'))return{rows:[{principal_id:principal,world_token:scope.worldToken,session_epoch:sessionEpoch,service_epoch:epoch,presence_until:new Date(100000),capabilities:['core.items']}]};
   if(sql.includes(' AS total,count(*) FILTER'))return{rows:[{total:0,pending:scenario.capacity?256:0}]};
   if(sql==='SELECT * FROM g3_bank_items WHERE run_id=$1 AND item_id=$2 FOR UPDATE'){locked++;return{rows:scenario.missing?[]:[{kind:'spell',state:scenario.state,revision:scenario.revision,descriptor:{catalogueRevision:C.revision,spell:{actionId:C.actions[0].id,usesRemaining:'-1'}}}]};}
   if(sql.startsWith('SELECT config FROM rooms'))return{rows:[{config:{mode:'core',settings:[]}}]};throw Error('Unexpected query');
  },tx:any={query},request={scope,itemClaim:{operationId:randomUUID(),itemId:randomUUID(),expectedRevision:'2'}};
  await assert.rejects(authority.execute(tx,actor,request),(e:any)=>e.code===scenario.outcome);assert.equal(tx.query,query);assert.equal(queries,scenario.capacity?4:scenario.missing?5:6);assert.equal(locked,scenario.capacity?0:1);
  const e=controls.status().claims.at(-1);assert.equal(e.outcome,scenario.outcome);assert.equal(e.expectedRevision,'2');assert.equal(e.commit,'unobserved');if(scenario.capacity)assert.equal(e.rowFound,undefined);else{assert.equal(e.rowFound,!scenario.missing);if(!scenario.missing){assert.equal(e.actualRevision,scenario.revision);assert.equal(e.actualState,scenario.state);}}assert.equal(JSON.stringify(e).includes(principal),false);
 }}finally{for(const hook of hooks)await hook();}
});
