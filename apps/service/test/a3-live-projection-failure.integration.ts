// Actual disposable TLS/SQL/WSS; engine observations injected, no native/Noita claim.
import test from 'node:test';import assert from 'node:assert/strict';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';import {livePeer} from '../scripts/g3-live-fixture.js';import {PolicyError} from '../src/g1/security.js';
test('failed optional transaction projection rolls back without deleting live peers; real revocation and End still retire',async()=>{
 const x=await startLifecycleLc1Fixture(),peers:Awaited<ReturnType<typeof livePeer>>[]=[];try{
  const peer=await x.bootstrapPeer(),qa=x.request(),qb=x.request(peer.desktop,1,'projection-peer',peer.observation);qb.reconcile!.association=peer.association;qb.reconcile!.proof=peer.proof;await x.send(x.desktop,qa);await x.send(peer.desktop,qb);await x.start();qa.reconcile!.observation={...x.observation,protected:false,observationRevision:2};qb.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};const values=[await x.send(x.desktop,qa),await x.send(peer.desktop,qb)],state={games:values.map(v=>v.credentials),scopes:values.map(v=>v.canonicalAdmission.scope)};
  peers.push(await livePeer(x.fixture as any,state as any,0),await livePeer(x.fixture as any,state as any,1));const [a,b]=peers;a!.send({pose:{sequence:1,xMilli:'1',yMilli:'2',vxMilli:'0',vyMilli:'0'}});await b!.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===a!.scope.sessionId)??false);
  const db=x.fixture.service.db,projection=x.fixture.service.live!.projection,room=projection.rooms.get(x.room.roomId)!,before=[...room.members.values()].map(m=>({generation:m.generation,until:m.until})),read=(projection as any).read;
  let entered!:()=>void,release!:()=>void;const pending=new Promise<void>(r=>entered=r),held=new Promise<void>(r=>release=r),locked=db.transaction(async()=>{entered();await held;});await pending;
  try{a!.send({pose:{sequence:2,xMilli:'2',yMilli:'2',vxMilli:'0',vyMilli:'0'}});await b!.wait(v=>v.snapshot?.peers?.some(p=>p.poses?.at(-1)?.sequence===2)??false,700);}finally{release();await locked;}
  (projection as any).read=async()=>{throw new PolicyError('CAPACITY');};try{await assert.rejects(db.transaction(tx=>tx.query('UPDATE accounts SET disabled=true WHERE id=$1',[a!.scope.principalId])),/CAPACITY/);}finally{(projection as any).read=read;}
  assert.equal((await db.pool.query('SELECT disabled FROM accounts WHERE id=$1',[a!.scope.principalId])).rows[0].disabled,false);assert.equal(room.terminal,false);assert.deepEqual([...room.members.values()].map(m=>({generation:m.generation,until:m.until})),before);assert.equal(a!.socket.readyState,1);assert.equal(b!.socket.readyState,1);
  a!.send({pose:{sequence:3,xMilli:'3',yMilli:'2',vxMilli:'0',vyMilli:'0'}});await b!.wait(v=>v.snapshot?.peers?.some(p=>p.poses?.at(-1)?.sequence===3)??false);
  await db.transaction(tx=>tx.query('UPDATE accounts SET disabled=true WHERE id=$1',[a!.scope.principalId]));await a!.wait(v=>!!v.retire);assert.equal(room.members.size,1);assert.equal(b!.socket.readyState,1);await x.end();await b!.wait(v=>!!v.retire);
 }finally{for(const p of peers)await p.close();await x.close();}
});
