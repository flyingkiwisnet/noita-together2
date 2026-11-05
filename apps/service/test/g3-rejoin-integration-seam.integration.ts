import test from 'node:test';import assert from 'node:assert/strict';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
test('RJ2 consumer provisioning cannot supply recovering inspection; actual HTTP loss targets only owned session and issuer restart is retained',async()=>{
 const f=await startG3Fixture({rejoin:true,loot01Runtime:true,live:true,tls:true});try{
  const s=await provisionG3Fixture(f,{consumerPlayer:0});assert.equal(s.games.length,1);assert.equal(s.reports.length,1);assert.equal(s.consumerPlayer,0);assert.deepEqual(s.injectedDesktopIds,[s.desktops[1]!.sessionId]);
  await assert.rejects(s.start(),/ROSTER_MISMATCH|NOT_READY/);assert.equal((await f.service.db.pool.query('SELECT count(*) FROM attachments WHERE desktop_id=$1',[s.desktops[0]!.sessionId])).rows[0].count,'0');
  await f.setSessionLoss(s.desktops[0]!.sessionId!,true);
  const body={major:1,revision:'g2-draft-5',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId:'http-loss',sessionEpoch:s.desktops[0]!.sessionEpoch,roomId:s.room.roomId,roomGet:{}};
  // Inject goes through the actual HTTP server hook, not fixture.call's test
  // wrapper. A real consumer using the same token receives the same refusal.
  const denied=await f.service.app.inject({method:'POST',url:'/v1/rpc',headers:{host:new URL(f.origin).host,authorization:'Bearer '+s.desktops[0]!.accessToken},payload:body});assert.equal(denied.statusCode,503);
  assert.ok((await s.rpc(s.desktops[1],{roomGet:{}})).room);
  await f.setSessionLoss(s.desktops[0]!.sessionId!,false);assert.ok((await s.rpc(s.desktops[0],{roomGet:{}})).room);
  // These injected consumer facts test only the fixture boundary, never
  // Core/native composition. No provision/peer helper can manufacture them.
  const desktop=s.desktops[0]!,prep=(await s.rpc(desktop,{roomGet:{}})).preparation!,gameInstanceId='seam-consumer';
  const game=(await s.rpc(desktop,{gameAttach:{preparationId:prep.preparationId!,gameInstanceId,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}})).credentials!;
  await s.rpc(game,{gameObservation:{...s.observations[0]!,attachmentId:game.attachmentId!,gameInstanceId,worldToken:'seam-consumer-world',observationRevision:1,protected:true}});await s.start();await s.playing();
  const epoch=f.service.db.epoch,issuer=f.service.auth.security.keys.issuer,ca=f.ca;await f.restart();assert.notEqual(f.service.db.epoch,epoch);assert.equal(f.service.auth.security.keys.issuer,issuer);assert.deepEqual(f.ca,ca);
  await s.recoverInjectedPeers();await s.pulse();
  const participants=(await f.service.db.pool.query('SELECT desktop_id,generation,state FROM rejoin_participants ORDER BY id')).rows;
  assert.equal(participants.find(p=>p.desktop_id===desktop.sessionId).state,'reconnecting');assert.equal(participants.find(p=>p.desktop_id===desktop.sessionId).generation,1);
  assert.equal(participants.find(p=>p.desktop_id===s.desktops[1]!.sessionId).state,'active');assert.equal(participants.find(p=>p.desktop_id===s.desktops[1]!.sessionId).generation,2);
 }finally{await f.close();}
});
