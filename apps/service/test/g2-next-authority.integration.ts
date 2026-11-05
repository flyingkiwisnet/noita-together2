import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,randomBytes} from 'node:crypto';
import {generateKeyPair,exportPKCS8,exportSPKI} from 'jose';
import pg from 'pg';
import {createCandidateApp} from '../src/g2-next/app.js';
import {schema} from '../../../packages/contracts/src/g2-next.js';
import type {Request,Response,Credentials,Room,GameObservation} from '../../../packages/contracts/g2-next/g2-next.js';
const url=process.env.NT2_G2_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
async function fixture(){
 const namespace='g2_next_routing_'+randomUUID().replaceAll('-',''),pair=await generateKeyPair('EdDSA',{extractable:true});let now=Date.now(),room:Room|undefined;
 const profile={gameVersion:'test',modVersion:'test',bridgeVersion:'test',capabilities:['core.seed'],modes:['core'],settingKeys:[]};
 const options={databaseUrl:url,serviceId:'nt2-g2-next-routing',namespace,keys:{privateKey:await exportPKCS8(pair.privateKey),publicKey:await exportSPKI(pair.publicKey),recoveryKey:randomBytes(32).toString('base64url'),issuer:'http://127.0.0.1:4354'},mockEnabled:true,now:()=>now,compatibility:profile};
 const service=await createCandidateApp(options);
 const envelope=(credentials:Credentials|undefined,payload:Partial<Request>):Request=>({major:1,revision:schema.revision,serviceId:service.db.serviceId,serviceEpoch:service.db.epoch,requestId:randomUUID(),...(credentials?{sessionEpoch:credentials.sessionEpoch!}:{}),...(room?{roomId:room.roomId!,expectedRevision:room.revision!,configRevision:room.configRevision!,runId:room.runId??'',runEpoch:room.runEpoch??''}:{}),...payload});
 const send=async(credentials:Credentials|undefined,request:Request):Promise<Response>=>{const response=await service.app.inject({method:'POST',url:'/v1/rpc',headers:credentials?{authorization:'Bearer '+credentials.accessToken}:{},payload:request});assert.equal(response.statusCode,200);const result=response.json() as Response;if(result.room)room=result.room;return result;};
 const rpc=(credentials:Credentials|undefined,payload:Partial<Request>)=>send(credentials,envelope(credentials,payload));
 const ok=async(credentials:Credentials|undefined,payload:Partial<Request>)=>{const result=await rpc(credentials,payload);assert.equal(result.error,undefined,result.error?.code);return result;};
 const host=(await service.app.inject({method:'POST',url:'/v1/mock/browser-login',payload:{subject:'website-local-alex',deviceId:'host-tab',requestId:randomUUID()}})).json().credentials as Credentials;
 await ok(host,{roomCreate:{name:'routed inspection',allowGuests:true,config:{seed:424242,mode:'core',requiredCapabilities:['core.seed']}}});
 const desktop=async(name:string)=>{const result=await ok(undefined,{guestIssue:{deviceId:name,displayName:name,issuanceProof:randomBytes(32).toString('base64url')}}),c=result.credentials!;await ok(c,{roomJoin:{}});await ok(c,{ready:{ready:true}});return c;};
 const prepare=async(parent:Credentials)=>{const result=await ok(parent,{prepare:{}});assert.ok(result.preparation);return result.preparation;};
 const attach=async(parent:Credentials,id:string)=>{const prep=await prepare(parent),result=await ok(parent,{gameAttach:{preparationId:prep!.preparationId!,gameInstanceId:id,gameVersion:'test',modVersion:'test',bridgeVersion:'test',capabilities:['core.seed']}});return result.credentials!;};
 const observation=(game:Credentials,instance:string,world:string,extra:Partial<GameObservation>={}):GameObservation=>({attachmentId:game.attachmentId!,gameInstanceId:instance,worldToken:world,observationRevision:1,actualSeed:'424242',seedStatus:'known',activeMods:['noita_together2','duplicate','duplicate'],inventoryComplete:true,collectorFailure:'',beta:'unknown',protected:true,loaded:true,modVersion:'test',bridgeVersion:'test',compatibilityProvenance:'game-thread-client-observation',...extra});
 const report=async(parent:Credentials)=>{const result=await ok(parent,{inspectionList:{limit:1}});return result.inspections!;};
 return {service,options,host,envelope,send,rpc,ok,desktop,prepare,attach,observation,report,get room(){return room!;},get now(){return now;},advance(ms:number){now+=ms;},async close(){await service.app.close();const pool=new pg.Pool({connectionString:url});try{await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);}finally{await pool.end();}}};
}
test('routed Start exposes exact unique release only to selected game; excluded expiry cannot suspend it',async()=>{
 const f=await fixture();try{
  const a=await f.desktop('selected'),b=await f.desktop('excluded');await f.ok(f.host,{prepare:{}});const ga=await f.attach(a,'game-a'),gb=await f.attach(b,'game-b');
  await f.ok(ga,{gameObservation:f.observation(ga,'game-a','world-a')});await f.ok(gb,{gameObservation:f.observation(gb,'game-b','world-b')});
  const reports=[];let cursor='';do{const response=await f.ok(f.host,{inspectionList:{limit:1,afterDesktopId:cursor}});reports.push(...response.inspections!);cursor=response.inspectionNextDesktopId??'';}while(cursor);
  const report=reports.find(r=>r.desktopId===a.sessionId)!;assert.deepEqual(report.activeMods,['noita_together2','duplicate','duplicate']);
  const request=f.envelope(f.host,{finalStart:{participants:[{desktopId:a.sessionId!,attachmentId:ga.attachmentId!,worldToken:report.worldToken!,observationRevision:report.observationRevision!,contentDigest:report.contentDigest!}],excludedDesktopIds:[b.sessionId!]}});
  assert.equal((await f.send(f.host,request)).error,undefined);assert.equal((await f.send(f.host,request)).error,undefined);assert.equal((await f.send(f.host,{...request,requestId:randomUUID()})).error?.code,'CONFLICT');
  const selected=await f.ok(ga,{roomGet:{}});assert.ok(selected.release);const release=selected.release!;assert.notEqual(release.authorizationId,request.requestId);assert.equal(release.startRequestId,request.requestId);assert.equal(release.gameSessionId,ga.sessionId);assert.equal(release.gameSessionEpoch,ga.sessionEpoch);assert.equal(release.desktopSessionEpoch,a.sessionEpoch);assert.equal(release.worldToken,'world-a');assert.equal(release.contentDigest,report.contentDigest);assert.equal(release.observationRevision,1);assert.equal(release.phase,'authorized');
  for(const c of [f.host,a,b])assert.equal((await f.ok(c,{roomGet:{}})).release,undefined);
  const excluded=await f.ok(gb,{roomGet:{}});assert.equal(excluded.release,undefined);assert.equal(excluded.gameStatus?.reason,'excluded');
  f.advance(20000);await f.ok(ga,{gamePulse:{attachmentId:ga.attachmentId!,gameInstanceId:'game-a',worldToken:'world-a'}});f.advance(20000);await f.service.rooms.sweep();
  const after=await f.ok(ga,{roomGet:{}});assert.equal(after.room!.state,'running');assert.equal(after.release!.authorizationId,release.authorizationId);assert.equal((await f.ok(gb,{roomGet:{}})).release,undefined);
  const badEpoch=await f.rpc(ga,{runEpoch:randomUUID(),roomGet:{}});assert.equal(badEpoch.error?.code,'STALE_EPOCH');assert.equal(badEpoch.release,undefined);
  await f.ok(ga,{gameObservation:f.observation(ga,'game-a','world-a',{observationRevision:2,protected:false})});assert.equal((await f.ok(ga,{roomGet:{}})).release!.phase,'playing');
  await f.ok(ga,{gameObservation:f.observation(ga,'game-a','world-a',{observationRevision:3,protected:false,activeMods:['changed']})});const revoked=await f.ok(ga,{roomGet:{}});assert.equal(revoked.room!.state,'suspended');assert.equal(revoked.release,undefined);assert.equal(revoked.gameStatus?.phase,'revoked');
 }finally{await f.close();}
});
test('routed preparation has fixed absolute review and independent deadlines, exact retries, scoped cancellation and expiry',async()=>{
 const f=await fixture();try{
  const a=await f.desktop('first'),b=await f.desktop('later');const start=f.now;await f.ok(f.host,{prepare:{}});assert.equal(Date.parse(f.room.reviewExpiresAt!),start+1800000);assert.equal(f.room.preparationExpiresAt,'');
  const firstRequest=f.envelope(a,{prepare:{}}),first=await f.send(a,firstRequest);assert.equal(Date.parse(first.preparation!.expiresAt!),start+600000);
  f.advance(5000);await f.ok(a,{presence:{present:true}});await f.ok(b,{presence:{present:true}});const repeated=await f.send(a,firstRequest);assert.equal(repeated.preparation!.preparationId,first.preparation!.preparationId);assert.equal(repeated.preparation!.expiresAt,first.preparation!.expiresAt);
  const late=await f.prepare(b);assert.equal(Date.parse(late!.expiresAt!),start+605000);assert.equal(late!.reviewExpiresAt,first.preparation!.reviewExpiresAt);
  const game=await f.attach(a,'first-game');await f.ok(game,{gameObservation:f.observation(game,'first-game','world')});assert.equal(Date.parse(f.room.reviewExpiresAt!),start+1800000);
  assert.equal((await f.rpc(a,{preparationCancel:{preparationId:late!.preparationId!}})).error?.code,'FORBIDDEN');
  await f.ok(b,{preparationCancel:{preparationId:late!.preparationId!}});assert.equal((await f.rpc(b,{gameAttach:{preparationId:late!.preparationId!,gameInstanceId:'cancelled',gameVersion:'test',modVersion:'test',bridgeVersion:'test',capabilities:['core.seed']}})).error?.code,'EXPIRED');
  await f.ok(b,{presence:{present:true}});const replacement=await f.prepare(b);assert.notEqual(replacement!.preparationId,late!.preparationId);assert.equal(replacement!.reviewExpiresAt,late!.reviewExpiresAt);
  f.advance(600001);await f.service.rooms.sweep();const state=await f.service.db.transaction(tx=>tx.query('SELECT state FROM preparations WHERE id=$1',[replacement!.preparationId]));assert.equal(state.rows[0].state,'expired');
  const reportState=await f.service.db.transaction(tx=>tx.query('SELECT review_until,created_at FROM runs WHERE id=$1',[f.room.runId]));assert.equal(+reportState.rows[0].review_until-(+reportState.rows[0].created_at),1800000);
 }finally{await f.close();}
});
test('parent-authenticated renewal covers expired game access in protected review and running without replacing its context',async()=>{
 const f=await fixture();try{
  let parent=await f.desktop('renewable');await f.ok(f.host,{prepare:{}});let game=await f.attach(parent,'renew-game');await f.ok(game,{gameObservation:f.observation(game,'renew-game','renew-world')});
  // Real routed pulses keep native liveness current; a clock jump alone must not manufacture it.
  for(let i=0;i<44;i++){f.advance(20000);await f.ok(game,{gamePulse:{attachmentId:game.attachmentId!,gameInstanceId:'renew-game',worldToken:'renew-world'}});}
  const renewedParent=await f.ok(undefined,{refresh:{secret:parent.refreshSecret!,deviceId:parent.deviceId!}});parent=renewedParent.credentials!;
  f.advance(20100);assert.equal((await f.rpc(game,{roomGet:{}})).error?.code,'UNAUTHENTICATED');
  const renewal=f.envelope(parent,{gameRenew:{attachmentId:game.attachmentId!,gameInstanceId:'renew-game',worldToken:'renew-world',gameSessionId:game.sessionId!,gameSessionEpoch:game.sessionEpoch!}}),renewed=await f.send(parent,renewal);assert.equal(renewed.error,undefined);assert.equal(renewed.credentials!.sessionId,game.sessionId);assert.equal(renewed.credentials!.sessionEpoch,game.sessionEpoch);assert.equal(renewed.credentials!.attachmentId,game.attachmentId);assert.equal((await f.send(parent,renewal)).credentials!.accessToken,renewed.credentials!.accessToken);game=renewed.credentials!;
  assert.equal((await f.ok(game,{roomGet:{}})).gameStatus?.phase,'hold');assert.equal((await f.rpc(parent,{gameRenew:{...renewal.gameRenew!,worldToken:'other'}})).error?.code,'RESTART_REQUIRED');
  await f.ok(parent,{presence:{present:true}});const reports=await f.report(parent),report=reports[0]!;
  // The browser also renews its ordinary expired access before final Start.
  const host=(await f.ok(undefined,{refresh:{secret:f.host.refreshSecret!,deviceId:f.host.deviceId!}})).credentials!;
  await f.ok(host,{finalStart:{participants:[{desktopId:parent.sessionId!,attachmentId:game.attachmentId!,worldToken:report.worldToken!,observationRevision:report.observationRevision!,contentDigest:report.contentDigest!}],excludedDesktopIds:[]}});
  const authorized=(await f.ok(game,{roomGet:{}})).release!;await f.ok(game,{gameObservation:f.observation(game,'renew-game','renew-world',{observationRevision:2,protected:false})});
  for(let i=0;i<44;i++){f.advance(20000);await f.ok(game,{gamePulse:{attachmentId:game.attachmentId!,gameInstanceId:'renew-game',worldToken:'renew-world'}});}
  parent=(await f.ok(undefined,{refresh:{secret:parent.refreshSecret!,deviceId:parent.deviceId!}})).credentials!;f.advance(20100);assert.equal((await f.rpc(game,{roomGet:{}})).error?.code,'UNAUTHENTICATED');
  game=(await f.ok(parent,{gameRenew:{attachmentId:game.attachmentId!,gameInstanceId:'renew-game',worldToken:'renew-world',gameSessionId:game.sessionId!,gameSessionEpoch:game.sessionEpoch!}})).credentials!;const playing=await f.ok(game,{roomGet:{}});assert.equal(playing.release!.phase,'playing');assert.equal(playing.release!.authorizationId,authorized.authorizationId);assert.ok(playing.release!.enteredPlayingAt);
  const transient=f.envelope(game,{gameObservation:f.observation(game,'renew-game','renew-world',{observationRevision:3,protected:true})});await f.send(game,transient);await f.send(game,transient);await f.service.rooms.sweep();
  const restoring=(await f.ok(game,{roomGet:{}})).release!;assert.equal(f.room.state,'running');assert.equal(restoring.phase,'restoring');assert.equal(restoring.enteredPlayingAt,playing.release!.enteredPlayingAt);assert.equal(restoring.authorizationId,authorized.authorizationId);assert.equal(restoring.observationRevision,authorized.observationRevision);assert.equal(restoring.capturedAt,authorized.capturedAt);assert.equal(restoring.contentDigest,authorized.contentDigest);
  const renewalAfter=f.envelope(parent,{gameRenew:{attachmentId:game.attachmentId!,gameInstanceId:'renew-game',worldToken:'renew-world',gameSessionId:game.sessionId!,gameSessionEpoch:game.sessionEpoch!}});const renewedAgain=await f.send(parent,renewalAfter);assert.equal(renewedAgain.error,undefined);assert.equal((await f.send(parent,renewalAfter)).credentials!.accessToken,renewedAgain.credentials!.accessToken);game=renewedAgain.credentials!;
  await f.ok(game,{gameObservation:f.observation(game,'renew-game','renew-world',{observationRevision:4,protected:false})});assert.equal((await f.ok(game,{roomGet:{}})).release!.phase,'playing');
  f.advance(30001);await f.service.rooms.sweep();assert.equal((await f.ok(game,{roomGet:{}})).release,undefined);assert.equal(f.room.state,'suspended');assert.ok((await f.rpc(parent,{gameRenew:renewalAfter.gameRenew!})).error);assert.equal((await f.rpc(game,{gameObservation:f.observation(game,'renew-game','renew-world',{observationRevision:5,protected:false})})).error?.code,'INVALID_STATE');

  await f.ok(parent,{logout:{}});assert.equal((await f.rpc(parent,{gameRenew:renewal.gameRenew!})).error?.code,'UNAUTHENTICATED');assert.equal((await f.rpc(game,{roomGet:{}})).error?.code,'UNAUTHENTICATED');
 }finally{await f.close();}
});

test('unknown collector facts remain visible but cannot Start; selected cancellation immediately suspends authorization',async()=>{
 const f=await fixture();try{
  const parent=await f.desktop('collector');await f.ok(f.host,{prepare:{}});const game=await f.attach(parent,'collector-game');
  const unknown=f.observation(game,'collector-game','collector-world',{seedStatus:'unknown',inventoryComplete:false,collectorFailure:'seed_unavailable'});delete unknown.actualSeed;
  await f.ok(game,{gameObservation:unknown});let report=(await f.report(parent))[0]!;assert.equal(report.seedStatus,'unknown');assert.equal(report.actualSeed,undefined);assert.equal(report.collectorFailure,'seed_unavailable');assert.equal(report.compatibility,'seed_unknown');
  const start=()=>({participants:[{desktopId:parent.sessionId!,attachmentId:game.attachmentId!,worldToken:report.worldToken!,observationRevision:report.observationRevision!,contentDigest:report.contentDigest!}],excludedDesktopIds:[]});
  assert.ok((await f.rpc(f.host,{finalStart:start()})).error);assert.equal(f.room.state,'staging');
  await f.ok(game,{gameObservation:f.observation(game,'collector-game','collector-world',{observationRevision:2})});report=(await f.report(parent))[0]!;await f.ok(f.host,{finalStart:start()});assert.ok((await f.ok(game,{roomGet:{}})).release);
  const own=await f.service.db.transaction(tx=>f.service.rooms.inspection.ownPreparation(tx,{kind:'desktop',sessionId:parent.sessionId!,sessionEpoch:parent.sessionEpoch!} as any,f.room.runId!));
  await f.ok(parent,{preparationCancel:{preparationId:own!.preparationId!}});assert.equal(f.room.state,'suspended');assert.equal((await f.rpc(game,{roomGet:{}})).error?.code,'UNAUTHENTICATED');
 }finally{await f.close();}
});

test('never-entered authorization cannot first play after absolute review deadline even with live native pulses',async()=>{
 const f=await fixture();try{
  const parent=await f.desktop('never-played');await f.ok(f.host,{prepare:{reviewMinutes:5}});const game=await f.attach(parent,'late-game');await f.ok(game,{gameObservation:f.observation(game,'late-game','late-world')});const report=(await f.report(parent))[0]!;
  await f.ok(f.host,{finalStart:{participants:[{desktopId:parent.sessionId!,attachmentId:game.attachmentId!,worldToken:report.worldToken!,observationRevision:report.observationRevision!,contentDigest:report.contentDigest!}],excludedDesktopIds:[]}});assert.equal((await f.ok(game,{roomGet:{}})).release!.enteredPlayingAt,undefined);
  for(let i=0;i<14;i++){f.advance(20000);await f.ok(game,{gamePulse:{attachmentId:game.attachmentId!,gameInstanceId:'late-game',worldToken:'late-world'}});}f.advance(20200);
  await f.rpc(game,{gameObservation:f.observation(game,'late-game','late-world',{observationRevision:2,protected:false})});await f.service.rooms.sweep();const state=await f.ok(game,{roomGet:{}});assert.equal(state.release,undefined);assert.equal(state.room!.state,'suspended');
  const entries=await f.service.db.transaction(tx=>tx.query('SELECT * FROM authorization_play_entries WHERE attachment_id=$1',[game.attachmentId]));assert.equal(entries.rowCount,0);
 }finally{await f.close();}
});

test('persisted playing entry survives in audit but cannot authorize a replacement service epoch',async()=>{
 const f=await fixture();let restarted:Awaited<ReturnType<typeof createCandidateApp>>|undefined;try{
  const parent=await f.desktop('restart-player');await f.ok(f.host,{prepare:{}});const game=await f.attach(parent,'restart-game');await f.ok(game,{gameObservation:f.observation(game,'restart-game','restart-world')});const report=(await f.report(parent))[0]!;
  await f.ok(f.host,{finalStart:{participants:[{desktopId:parent.sessionId!,attachmentId:game.attachmentId!,worldToken:report.worldToken!,observationRevision:report.observationRevision!,contentDigest:report.contentDigest!}],excludedDesktopIds:[]}});await f.ok(game,{gameObservation:f.observation(game,'restart-game','restart-world',{observationRevision:2,protected:false})});const before=(await f.ok(game,{roomGet:{}})).release!;assert.ok(before.enteredPlayingAt);
  const request=f.envelope(game,{roomGet:{}});await f.service.app.close();restarted=await createCandidateApp(f.options);assert.notEqual(restarted.db.epoch,request.serviceEpoch);
  const response=(await restarted.app.inject({method:'POST',url:'/v1/rpc',headers:{authorization:'Bearer '+game.accessToken},payload:request})).json() as Response;assert.ok(response.error);assert.equal(response.release,undefined);
  const audit=await restarted.db.transaction(tx=>tx.query('SELECT e.entered_playing_at,x.revoked_at,x.reason FROM authorization_play_entries e JOIN inspected_releases x USING(authorization_uuid) WHERE e.authorization_uuid=$1',[before.authorizationId]));assert.equal(audit.rowCount,1);assert.ok(audit.rows[0].revoked_at);assert.equal(audit.rows[0].reason,'service_restart');
 }finally{if(restarted)await restarted.app.close();await f.close();}
});
