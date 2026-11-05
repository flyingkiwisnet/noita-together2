// Actual TLS/SQL host End, with labelled policy refusal injection only.
import test from'node:test';import assert from'node:assert/strict';import{b1Fixture}from'./b1-fixture-support.js';import{PolicyError}from'../src/g1/security.js';
test('End retries the identical optional prerequisite after busy and submits one real control mutation',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{const rooms=f.x.fixture.service.rooms as any,execute=rooms.execute,reads:any[]=[],ends:any[]=[];
  rooms.execute=async function(tx:any,req:any,actor:any){if(req.roomGet){reads.push({request:JSON.stringify(req),session:actor.sessionId});if(reads.length===1)throw new PolicyError('CAPACITY');}if(req.endRun)ends.push(req.requestId);return execute.call(this,tx,req,actor);};
  try{const value=await f.x.end();assert.equal(value.room.state,'ended');assert.equal(reads.length,2);assert.deepEqual(reads[0],reads[1]);assert.equal(ends.length,1);assert.ok(f.x.hostActionDiagnostics().some(e=>e.stage==='read'&&e.code==='CAPACITY'));assert.ok(f.x.hostActionDiagnostics().some(e=>e.stage==='submit'&&e.outcome==='accepted'));
   const row=(await f.x.fixture.service.db.transaction(tx=>tx.query('SELECT state FROM runs WHERE id=$1',[f.scope(0).runId]))).rows[0];assert.equal(row.state,'ended');
  }finally{rooms.execute=execute;}
 }finally{await f.close();}
});
test('Actual control End CAPACITY is reported at submit and never retried',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{const rooms=f.x.fixture.service.rooms as any,execute=rooms.execute;let calls=0;
  rooms.execute=async function(tx:any,req:any,actor:any){if(req.endRun){calls++;throw new PolicyError('CAPACITY');}return execute.call(this,tx,req,actor);};
  try{await assert.rejects(f.x.end(),/CAPACITY/);assert.equal(calls,1);assert.equal(f.x.hostActionDiagnostics().at(-1).stage,'submit');assert.equal(f.x.hostActionDiagnostics().at(-1).code,'CAPACITY');assert.equal(f.x.room.state,'running');}finally{rooms.execute=execute;}
 }finally{await f.close();}
});
