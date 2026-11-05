// Real isolated HTTP/SQL, including postcommit response validation failure.
import test from'node:test';import assert from'node:assert/strict';import{startLifecycleLc1Fixture}from'../scripts/lifecycle-lc1-fixture.js';import{PolicyError}from'../src/g1/security.js';
test('Prepared fixture chat actor retries exact read/send CAPACITY and postcommit response loss yields one message',async()=>{
 const x=await startLifecycleLc1Fixture({b1Gold:true,itemTransfers:true});try{
  await x.bootstrapPeer();assert.equal(x.chatDiagnostics().filter(e=>e.stage==='actor-bootstrap'&&e.outcome==='accepted').length,2);
  const db=x.fixture.service.db,rooms=x.fixture.service.rooms as any,execute=rooms.execute,reads:any[]=[],sends:any[]=[];
  const authCount=async()=>+(await db.transaction(tx=>tx.query('SELECT count(*) FROM account_signin_attempts'))).rows[0].count,before=await authCount();
  rooms.execute=async function(tx:any,req:any,actor:any){
   if(req.roomGet){reads.push({request:JSON.stringify(req),session:actor.sessionId});if(reads.length===1)throw new PolicyError('CAPACITY');}
   const result=await execute.call(this,tx,req,actor);
   if(req.chatSend){sends.push({request:JSON.stringify(req),session:actor.sessionId});if(sends.length===1)return{...result,fixtureInvalidResponse:true};}
   return result;
  };
  try{const value=await x.chat('host','one committed chat despite response capacity');assert.ok(value.chat.sequence);assert.equal(reads.length,2);assert.equal(sends.length,2);assert.deepEqual(reads[0],reads[1]);assert.deepEqual(sends[0],sends[1]);assert.equal(await authCount(),before,'measured chat performs no signin or refresh');
   const messages=(await db.transaction(tx=>tx.query('SELECT text FROM room_chat WHERE room_id=$1',[x.room.roomId]))).rows;assert.deepEqual(messages,[{text:'one committed chat despite response capacity'}]);assert.deepEqual(value.chatDiagnostic.filter(e=>e.code==='CAPACITY').map(e=>e.stage),['read','send']);
  }finally{rooms.execute=execute;}
 }finally{await x.close();}
});
