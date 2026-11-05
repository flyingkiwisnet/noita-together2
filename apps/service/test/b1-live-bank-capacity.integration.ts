// Actual TLS/WSS/auth/admission. Only the finite DB work occupancy is injected.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {b1Fixture} from './b1-fixture-support.js';import {livePeer} from '../scripts/g3-live-fixture.js';
test('B1 WSS renewal attributes shared DB admission refusal and reconnects the same scope without another bank effect',async()=>{
 const f=await b1Fixture(),peers:Awaited<ReturnType<typeof livePeer>>[]=[],release:(()=>void)[]=[];
 try{
  const intent={goldIntent:{operationId:randomUUID(),direction:'deposit',amount:10}},deposit=(await f.ok(0,intent)).operation;await f.ok(0,f.receipt(deposit,'escrowed'));
  const credentials=f.values[0].credentials,s={scopes:[f.scope(0)],games:[{...credentials,principal:{principalId:credentials.accountId}}]},connect=()=>livePeer(f.x.fixture as any,s as any,0);
  const first=await connect();peers.push(first);
  // Occupy the existing seven permits, without changing limits, SQL state,
  // tokens, authority, or a production timing/retry path.
  const gate=(f.x.fixture.service.db as any).gate;for(let i=0;i<7;i++)release.push(gate.enter());
  first.send({reauthenticate:{accessToken:credentials.accessToken,authRevision:2}});
  const retired=await first.wait(frame=>!!frame.retire);assert.equal(retired.retire!.code,'CAPACITY');assert.equal(retired.retire!.retryAfterMs,250);
  const evidence=f.x.faults.capacityDiagnostics!.status().firstRetire;assert.equal(evidence.guard,'database-work');assert.equal(evidence.database.active,7);assert.equal(evidence.database.limit,7);assert.equal(evidence.admission.active,1);assert.equal(evidence.admission.rejected,0);assert.equal(evidence.phase,'admission');assert.equal(evidence.commit,'not-attempted');
  release.splice(0).forEach(done=>done());await first.close();
  const resumed=await connect();peers.push(resumed);assert.deepEqual(resumed.scope,first.scope);
  resumed.send({reauthenticate:{accessToken:credentials.accessToken,authRevision:2}});await resumed.wait(frame=>frame.welcome?.authRevision===2);
  resumed.send({pose:{sequence:1,xMilli:'0',yMilli:'0',vxMilli:'0',vyMilli:'0'}});
  for(let n=0;n<100&&!f.x.fixture.service.live!.stats.receivedPoses;n++)await new Promise(r=>setTimeout(r,10));assert.equal(f.x.fixture.service.live!.stats.receivedPoses,1);
  const duplicate=(await f.ok(0,intent)).operation;assert.equal(duplicate.deliveryId,deposit.deliveryId);assert.equal(duplicate.balance,10);assert.equal(duplicate.balanceVersion,'1');
  assert.equal((await f.x.fixture.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM g3_ledger'))).rows[0].n,1);
  console.log(JSON.stringify({injectedBoundary:'seven existing DB work permits occupied; no SQL/credential/authority mutation',firstRetire:evidence,recovery:{sameScope:true,reauthRevision:2,acceptedPoses:1,bankBalance:10,balanceVersion:'1',ledgerEntries:1}}));
 }finally{release.splice(0).forEach(done=>done());for(const peer of peers)await peer.close();await f.close();}
});
