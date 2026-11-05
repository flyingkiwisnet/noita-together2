import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID,randomBytes} from 'node:crypto';
import {startG3Fixture} from '../scripts/g3-fixture.js';
import {contentDigest} from '../src/g2-next/observation.js';
import type {Credentials,Request,Response,Room,GameObservation} from '../../../packages/contracts/g2-next/g2-next.js';
async function fixture(seedSupport=false){
 let now=Date.now();const f=await startG3Fixture({live:true,tls:true,now:()=>now});let room:Room;
 try{
 if(seedSupport)f.profile.compatibility.capabilities.push('core.seed-policy.v1','core.randomize-loot.v1');
 const request=(c:Credentials|undefined,payload:Partial<Request>):Request=>({major:1,revision:'g2-draft-5',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId:randomUUID(),...(c?{sessionEpoch:c.sessionEpoch!}:{}),...(room?{roomId:room.roomId!,runId:room.runId!,runEpoch:room.runEpoch!,configRevision:room.configRevision!,expectedRevision:room.revision!}:{}),...payload});
 const send=async(c:Credentials|undefined,input:Request)=>{now+=500;const response=await f.call('/v1/rpc',input,c?.accessToken);assert.equal(response.status,200,response.body);const value=JSON.parse(response.body) as Response;if(value.room)room=value.room;return value;};
 const rpc=async(c:Credentials|undefined,payload:Partial<Request>)=>{const value=await send(c,request(c,payload));assert.equal(value.error,undefined,JSON.stringify(value.error));return value;};
 const host=JSON.parse((await f.call('/v1/mock/browser-login',{subject:'website-local-alex',deviceId:'flow-host',requestId:randomUUID()})).body).credentials as Credentials;
 const desktops:Credentials[]=[];for(let i=0;i<3;i++)desktops.push((await rpc(undefined,{guestIssue:{deviceId:'flow-desktop-'+i,issuanceProof:randomBytes(32).toString('base64url'),displayName:'Flow player '+i}})).credentials!);
 await rpc(host,{roomCreate:{name:'Automatic connection flow',allowGuests:true,config:{seed:424242,mode:'core',requiredCapabilities:f.profile.compatibility.capabilities}}});
 const reports=new Map<string,GameObservation>(),games=new Map<string,Credentials>();
 const gameMetadata={gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities};
 const join=async(i:number)=>rpc(desktops[i],{roomJoin:{}});
 const hold=async(i:number,override:Partial<GameObservation>={})=>{
  const d=desktops[i]!,p=(await rpc(d,{roomGet:{}})).preparation!;
  const attached=await rpc(d,{gameAttach:{preparationId:p.preparationId!,gameInstanceId:'flow-game-'+i,...gameMetadata}});
  const g=attached.credentials!;games.set(d.sessionId!,g);
  const report:GameObservation={attachmentId:g.attachmentId!,gameInstanceId:'flow-game-'+i,worldToken:'flow-world-'+i,observationRevision:1,actualSeed:String(room.config!.seed),seedStatus:'known',collectorFailure:'',modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,compatibilityProvenance:'game-thread-client-observation',activeMods:['noita_together2','advisory-other-mod','advisory-other-mod'],inventoryComplete:true,beta:'unknown',protected:true,loaded:true,...override};
  await rpc(g,{gameObservation:report,expectedRevision:0});reports.set(d.sessionId!,report);return {g,report,attachRequest:attached};
 };
 const selection=(indices:number[],excluded:number[]=[])=>({participants:indices.map(i=>{const d=desktops[i]!,p=reports.get(d.sessionId!)!;return {desktopId:d.sessionId!,attachmentId:p.attachmentId!,worldToken:p.worldToken!,observationRevision:p.observationRevision!,contentDigest:contentDigest(p)};}),excludedDesktopIds:excluded.map(i=>desktops[i]!.sessionId!)});
 const presence=async()=>{for(const d of desktops){if(room.members?.some(m=>m.desktopId===d.sessionId))await rpc(d,{presence:{present:true},expectedRevision:0});}};
 return {f,host,desktops,reports,games,gameMetadata,request,send,rpc,join,hold,selection,get room(){return room;},advance(ms:number){now+=ms;},presence,async waitLive(ms:number){for(let elapsed=0;elapsed<ms;elapsed+=20000){now+=Math.min(20000,ms-elapsed);await presence();for(const [id,g] of games){const p=reports.get(id)!;await rpc(g,{gamePulse:{attachmentId:g.attachmentId!,gameInstanceId:p.gameInstanceId!,worldToken:p.worldToken!},expectedRevision:0});}}},async refresh(){for(const c of [host,...desktops]){const value=await rpc(undefined,{refresh:{secret:c.refreshSecret!,deviceId:c.deviceId!}});Object.assign(c,value.credentials);}},close:()=>f.close()};
 }catch(e){await f.close();throw e;}
}
test('U1 automatic create/concurrent joins, same preparation recovery, actual reports and default complete atomic roster',async()=>{
 const s=await fixture();try{
  assert.equal(JSON.parse((await s.f.call('/v1/info')).body).connectionFlowRevision,'g3-flow-1');assert.equal(s.room.state,'staging');assert.ok(s.room.runId);
  const first=s.request(s.desktops[0],{roomJoin:{}}),second=s.request(s.desktops[1],{roomJoin:{}});
  const raced=await Promise.all([s.send(s.desktops[0],first),s.send(s.desktops[1],second)]);assert.equal(raced.filter(v=>!v.error).length,1);assert.equal(raced.find(v=>v.error)?.error?.code,'STALE_REVISION');
  const winner=raced[0]!.error?1:0,loser=1-winner;const own=raced[winner]!.preparation!;assert.equal(own.state,'issued');assert.equal(s.room.members![0]!.ready,false);
  assert.equal((await s.send(s.desktops[winner],winner===0?first:second)).preparation!.preparationId,own.preparationId);
  await s.join(loser);await s.join(2);const id=(await s.rpc(s.desktops[0],{roomGet:{}})).preparation!.preparationId;
  await s.rpc(s.desktops[0],{ready:{ready:true}});assert.ok(s.room.members!.every(m=>!m.ready&&m.included&&m.readiness==='waiting_noita'));
  const a=await s.hold(0,{loaded:false});assert.equal(s.room.members!.find(m=>m.desktopId===s.desktops[0]!.sessionId)!.ready,false);
  const changed={...a.report,loaded:true,actualSeed:'424243',observationRevision:2};await s.rpc(a.g,{gameObservation:changed,expectedRevision:0});assert.equal(s.room.members!.find(m=>m.desktopId===s.desktops[0]!.sessionId)!.readinessReason,'seed_mismatch');
  const matched={...changed,actualSeed:'424242',observationRevision:3};await s.rpc(a.g,{gameObservation:matched,expectedRevision:0});s.reports.set(s.desktops[0]!.sessionId!,matched);const incomplete=await s.hold(1,{inventoryComplete:false,collectorFailure:'mods_unavailable'});
  assert.equal(s.room.members!.find(m=>m.desktopId===s.desktops[1]!.sessionId)!.readinessReason,'inventory_incomplete');assert.equal((await s.send(s.host,s.request(s.host,{finalStart:s.selection([0,1],[2])}))).error?.code,'NOT_READY');
  const full={...incomplete.report,inventoryComplete:true,collectorFailure:'',observationRevision:2};await s.rpc(incomplete.g,{gameObservation:full,expectedRevision:0});s.reports.set(s.desktops[1]!.sessionId!,full);
  let cursor='';const observed=[];do{const page=await s.rpc(s.host,{inspectionList:{limit:1,afterDesktopId:cursor},expectedRevision:0});observed.push(...page.inspections!);cursor=page.inspectionNextDesktopId??'';}while(cursor);assert.ok(observed.every(p=>p.activeMods?.join(',')==='noita_together2,advisory-other-mod,advisory-other-mod'));
  assert.equal((await s.rpc(s.desktops[0],{roomGet:{}})).preparation!.preparationId,id);assert.equal(s.room.members!.filter(m=>m.ready).length,2);
  const omitted=await s.send(s.host,s.request(s.host,{finalStart:s.selection([0,1])}));assert.equal(omitted.error?.code,'ROSTER_MISMATCH');
  const pending=s.request(s.host,{finalStart:s.selection([0,1],[2])});const newReport={...matched,observationRevision:4};await s.rpc(a.g,{gameObservation:newReport,expectedRevision:0});assert.equal((await s.send(s.host,pending)).error?.code,'STALE_REVISION');
  assert.equal((await s.send(s.host,s.request(s.host,{finalStart:s.selection([0,1],[2])}))).error?.code,'REPORT_CHANGED');s.reports.set(s.desktops[0]!.sessionId!,newReport);
  await s.rpc(s.desktops[1],{presence:{present:false}});assert.equal((await s.send(s.host,s.request(s.host,{finalStart:s.selection([0,1],[2])}))).error?.code,'NOT_READY');await s.presence();
  const start=s.request(s.host,{finalStart:s.selection([0,1],[2])});assert.equal((await s.send(s.host,start)).error,undefined);assert.equal(s.room.state,'running');assert.equal(s.room.members!.filter(m=>m.selected).length,2);assert.equal(s.room.members!.find(m=>m.desktopId===s.desktops[2]!.sessionId)!.readiness,'excluded');
  const release=(await s.rpc(a.g,{roomGet:{},expectedRevision:0})).release!;assert.ok(release.authorizationId);assert.equal(release.actualSeed,undefined);assert.equal(release.seedPolicy,undefined);assert.equal(release.randomizeLoot,undefined);await s.send(s.host,start);assert.equal((await s.rpc(a.g,{roomGet:{}})).release!.authorizationId,release.authorizationId);
 }finally{await s.close();}
});
test('U1 configuration changes invalidate consent/unconsumed preparation; consumed seed requires explicit cancel',async()=>{
 const s=await fixture();try{
  await s.join(0);const old=(await s.rpc(s.desktops[0],{roomGet:{}})).preparation!;const oldAttach=s.request(s.desktops[0],{gameAttach:{preparationId:old.preparationId!,gameInstanceId:'late',...s.gameMetadata}});
  await s.rpc(s.host,{configure:{...s.room.config!,seed:12345}});assert.notEqual(s.room.runId,old.runId);assert.equal(s.room.members![0]!.readiness,'configuration_changed');assert.equal((await s.rpc(s.desktops[0],{roomGet:{}})).preparation,undefined);
  assert.equal((await s.send(s.desktops[0],oldAttach)).error?.code,'STALE_REVISION');await s.join(0);const p=(await s.rpc(s.desktops[0],{roomGet:{}})).preparation!;
  const attach=s.request(s.desktops[0],{gameAttach:{preparationId:p.preparationId!,gameInstanceId:'consumed-original',...s.gameMetadata}}),attached=await s.send(s.desktops[0],attach);assert.equal(attached.error,undefined);
  const recovered=await s.send(s.desktops[0],attach);assert.equal(recovered.credentials!.accessToken,attached.credentials!.accessToken);assert.equal(recovered.credentials!.attachmentId,attached.credentials!.attachmentId);
  assert.equal((await s.send(s.host,s.request(s.host,{configure:{...s.room.config!,seed:54321}}))).error?.code,'INVALID_STATE');assert.equal(s.room.config!.seed,12345);
  assert.equal((await s.send(s.desktops[0],{...attach,gameAttach:{...attach.gameAttach!,gameInstanceId:'replacement'}})).error?.code,'CONFLICT');
  await s.rpc(s.host,{cancelRun:{}});assert.equal((await s.rpc(s.desktops[0],{roomGet:{}})).preparation!.state,'cancelled');assert.equal((await s.send(s.desktops[0],attach)).error?.code,'RESTART_REQUIRED');
  assert.equal((await s.send(attached.credentials,s.request(attached.credentials,{roomGet:{}}))).error?.code,'UNAUTHENTICATED');
 }finally{await s.close();}
});
test('U1 waiting beyond five/ten minutes renews only live issued identity within fixed review deadline; terminal state never resurrects',async()=>{
 const s=await fixture();try{
  await s.join(0);const initial=(await s.rpc(s.desktops[0],{roomGet:{}})).preparation!,review=initial.reviewExpiresAt;
  s.advance(9*60000);await s.presence();const renew=s.request(s.desktops[0],{preparationRenew:{preparationId:initial.preparationId!},expectedRevision:0}),renewed=(await s.send(s.desktops[0],renew)).preparation!;
  assert.equal(renewed.preparationId,initial.preparationId);assert.equal(renewed.reviewExpiresAt,review);assert.ok(Date.parse(renewed.expiresAt!)>Date.parse(initial.expiresAt!));assert.equal((await s.send(s.desktops[0],renew)).preparation!.preparationId,initial.preparationId);
  s.advance(2*60000);await s.presence();const held=await s.hold(0);assert.equal((await s.rpc(s.desktops[0],{roomGet:{}})).preparation!.state,'loaded');
  assert.equal((await s.send(s.desktops[0],s.request(s.desktops[0],{preparationRenew:{preparationId:initial.preparationId!}}))).error?.code,'EXPIRED');
  await s.rpc(s.desktops[0],{gameRenew:{attachmentId:held.g.attachmentId!,gameSessionId:held.g.sessionId!,gameSessionEpoch:held.g.sessionEpoch!,gameInstanceId:held.report.gameInstanceId!,worldToken:held.report.worldToken!}});
  await s.waitLive(3*60000);await s.refresh();await s.waitLive(10*60000);assert.equal((await s.rpc(s.desktops[0],{roomGet:{}})).preparation!.state,'loaded');
  await s.join(1);const later=(await s.rpc(s.desktops[1],{roomGet:{}})).preparation!;assert.equal(later.expiresAt,review);
  s.advance(6*60000);await s.refresh();await s.f.service.rooms.sweep();assert.equal((await s.rpc(s.desktops[0],{roomGet:{}})).room!.state,'waiting');
  const result=await s.send(s.desktops[1],s.request(s.desktops[1],{preparationRenew:{preparationId:later.preparationId!},runId:later.runId!,runEpoch:later.runEpoch!}));assert.ok(result.error);assert.notEqual(result.preparation?.state,'issued');
 }finally{await s.close();}
});
test('U1 expired/consumed waiting worlds cannot be replaced; ambiguous join resolves original identity through room read',async()=>{
 const s=await fixture();try{
  const join=s.request(s.desktops[0],{roomJoin:{}}),joined=await s.send(s.desktops[0],join),id=joined.preparation!.preparationId;
  s.advance(31000);assert.equal((await s.send(s.desktops[0],join)).error?.code,'EXPIRED');await s.presence();assert.equal((await s.rpc(s.desktops[0],{roomGet:{}})).preparation!.preparationId,id);
  s.advance(10*60000);await s.presence();await s.f.service.rooms.sweep();assert.equal((await s.rpc(s.desktops[0],{roomGet:{}})).preparation!.state,'expired');
  assert.equal((await s.send(s.desktops[0],s.request(s.desktops[0],{prepare:{}}))).error?.code,'RESTART_REQUIRED');assert.equal((await s.send(s.desktops[0],s.request(s.desktops[0],{roomJoin:{}}))).error?.code,'RESTART_REQUIRED');
  const count=await s.f.service.db.transaction(tx=>tx.query('SELECT count(*) FROM preparations WHERE run_id=$1 AND desktop_id=$2',[s.room.runId,s.desktops[0]!.sessionId]));assert.equal(+count.rows[0].count,1);
 }finally{await s.close();}
});
test('U1 report change/End/revocation retire committed gameplay and watched live projection',async()=>{
 const s=await fixture();try{
  await s.join(0);await s.join(1);const a=await s.hold(0),b=await s.hold(1);await s.rpc(s.host,{finalStart:s.selection([0,1])});
  s.f.service.live!.projection.watch(s.room.roomId!);
  for(const [i,p] of [[0,a],[1,b]] as const){const playing={...p.report,observationRevision:2,protected:false};await s.rpc(p.g,{gameObservation:playing,expectedRevision:0});s.reports.set(s.desktops[i]!.sessionId!,playing);}
  assert.equal(s.f.service.live!.projection.rooms.get(s.room.roomId!)!.members.size,2);
  await s.rpc(a.g,{gameObservation:{...a.report,observationRevision:3,protected:false,activeMods:['changed-after-start']},expectedRevision:0});assert.equal(s.room.state,'suspended');const retired=s.f.service.live!.projection.rooms.get(s.room.roomId!);assert.ok(!retired||retired.terminal&&retired.members.size===0);
  await s.rpc(s.host,{endRun:{}});assert.equal(s.room.state,'ended');assert.equal((await s.send(b.g,s.request(b.g,{roomGet:{}}))).error?.code,'UNAUTHENTICATED');
 }finally{await s.close();}
});
test('U1 explicit family revocation retires after-commit live scope and cannot be repaired by setup retry',async()=>{
 const s=await fixture();try{
  await s.join(0);await s.join(1);const a=await s.hold(0),b=await s.hold(1);await s.rpc(s.host,{finalStart:s.selection([0,1])});s.f.service.live!.projection.watch(s.room.roomId!);
  for(const p of [a,b])await s.rpc(p.g,{gameObservation:{...p.report,observationRevision:2,protected:false},expectedRevision:0});assert.equal(s.f.service.live!.projection.rooms.get(s.room.roomId!)!.members.size,2);
  await s.rpc(s.desktops[0],{logout:{}});assert.equal(s.f.service.live!.projection.rooms.has(s.room.roomId!),false);assert.equal((await s.send(a.g,s.request(a.g,{roomGet:{}}))).error?.code,'UNAUTHENTICATED');assert.equal((await s.send(s.desktops[0],s.request(s.desktops[0],{roomJoin:{}}))).error?.code,'UNAUTHENTICATED');assert.equal((await s.rpc(s.host,{roomGet:{}})).room!.state,'suspended');
 }finally{await s.close();}
});
test('U1 waiting renewal cannot revive expired loaded game contact or replace its consumed world',async()=>{
 const s=await fixture();try{
  await s.join(0);const a=await s.hold(0),original=(await s.rpc(s.desktops[0],{roomGet:{}})).preparation!;s.advance(31000);await s.presence();
  assert.equal((await s.rpc(a.g,{roomGet:{}})).gameStatus!.phase,'revoked');
  assert.equal((await s.send(a.g,s.request(a.g,{gamePulse:{attachmentId:a.g.attachmentId!,gameInstanceId:a.report.gameInstanceId!,worldToken:a.report.worldToken!}}))).error?.code,'RESTART_REQUIRED');
  assert.equal((await s.send(a.g,s.request(a.g,{gameObservation:{...a.report,observationRevision:2}}))).error?.code,'RESTART_REQUIRED');
  assert.equal((await s.send(s.desktops[0],s.request(s.desktops[0],{gameRenew:{attachmentId:a.g.attachmentId!,gameSessionId:a.g.sessionId!,gameSessionEpoch:a.g.sessionEpoch!,gameInstanceId:a.report.gameInstanceId!,worldToken:a.report.worldToken!}}))).error?.code,'RESTART_REQUIRED');
  assert.equal((await s.rpc(s.desktops[0],{prepare:{}})).preparation!.preparationId,original.preparationId);assert.equal(s.room.members![0]!.ready,false);assert.equal((await s.send(s.desktops[0],s.request(s.desktops[0],{gameAttach:{preparationId:original.preparationId!,gameInstanceId:'replacement',...s.gameMetadata}}))).error?.code,'EXPIRED');
 }finally{await s.close();}
});
function previewRequest(s:Awaited<ReturnType<typeof fixture>>,c:Credentials):Request{return {major:1,revision:'g2-draft-5',serviceId:s.f.service.db.serviceId,serviceEpoch:s.f.service.db.epoch,sessionEpoch:c.sessionEpoch!,requestId:randomUUID(),roomId:s.room.roomId!,roomPreview:{}};}
async function previewAuthorityState(s:Awaited<ReturnType<typeof fixture>>){return s.f.service.db.transaction(async tx=>(await tx.query(`SELECT to_jsonb(r) AS room,
 (SELECT to_jsonb(n) FROM runs n WHERE n.id=r.current_run_id) AS run,
 (SELECT COALESCE(jsonb_agg(to_jsonb(m) ORDER BY m.desktop_id),'[]') FROM memberships m WHERE m.room_id=r.id) AS membership,
 (SELECT COALESCE(jsonb_agg(to_jsonb(p) ORDER BY p.id),'[]') FROM preparations p WHERE p.run_id=r.current_run_id) AS preparation,
 (SELECT COALESCE(jsonb_agg(to_jsonb(a) ORDER BY a.id),'[]') FROM attachments a WHERE a.run_id=r.current_run_id) AS attachment,
 (SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.run_id),'[]') FROM inspected_releases x WHERE x.run_id=r.current_run_id) AS release,
 (SELECT count(*) FROM command_results) AS recorded_commands FROM rooms r WHERE r.id=$1`,[s.room.roomId])).rows[0]);}
test('FLOW-PREVIEW01 public actual-handler configuration round-trip is limited/coherent/read-only before displayed Join',async()=>{
 const s=await fixture();try{
  // Inject only a supported setting in this disposable simulated registry;
  // the actual config validator/handler persists and projects its exact value.
  s.f.service.rooms.compatibility.settingKeys.push('flowPreviewRule');
  await s.rpc(s.host,{configure:{...s.room.config!,settings:[{key:'flowPreviewRule',value:'visible-choice'}]}});await s.join(0);await s.hold(0);
  await s.rpc(s.host,{roomPolicy:{capacity:32,password:'fixture-preview-secret'}});const before=await previewAuthorityState(s),request=previewRequest(s,s.desktops[1]!);
  const shown=await s.send(s.desktops[1],request);assert.equal(shown.error,undefined);const publicRoom=shown.room!,admission=shown.admission!;
  assert.deepEqual(Object.keys(publicRoom).sort(),['config','configRevision','name','revision','roomId','state']);
  for(const key of ['roomId','revision','configRevision','name','state'] as const)assert.equal(publicRoom[key],admission[key]);
  assert.deepEqual(publicRoom.config,before.room.config);assert.deepEqual(publicRoom.config!.settings,[{key:'flowPreviewRule',value:'visible-choice'}]);assert.equal(admission.passwordRequired,true);
  for(const field of ['preparation','credentials','release','inspections','gameStatus'] as const)assert.equal(shown[field],undefined);
  const body=JSON.stringify(shown);for(const privateValue of ['fixture-preview-secret',s.desktops[0]!.accountId!,s.desktops[0]!.sessionId!,s.games.get(s.desktops[0]!.sessionId!)!.attachmentId!])assert.ok(!body.includes(privateValue));
  assert.deepEqual(await previewAuthorityState(s),before);assert.equal((await s.send(s.desktops[1],request)).preparation,undefined);assert.deepEqual(await previewAuthorityState(s),before);
  const {roomPreview,...joinEnvelope}=previewRequest(s,s.desktops[1]!);
  assert.equal((await s.send(s.desktops[1],{...joinEnvelope,roomJoin:{},expectedRevision:publicRoom.revision!,configRevision:publicRoom.configRevision!})).error?.code,'PASSWORD_REQUIRED');assert.deepEqual(await previewAuthorityState(s),before);
  const joined=await s.send(s.desktops[1],{...joinEnvelope,requestId:randomUUID(),roomJoin:{password:'fixture-preview-secret'},expectedRevision:publicRoom.revision!,configRevision:publicRoom.configRevision!});
  assert.equal(joined.error,undefined);assert.ok(joined.preparation?.preparationId);assert.equal(joined.preparation!.configRevision,publicRoom.configRevision);assert.deepEqual(joined.room!.config,publicRoom.config);assert.equal(joined.room!.members!.find(m=>m.desktopId===s.desktops[1]!.sessionId)!.ready,false);
 }finally{await s.close();}
});
test('FLOW-PREVIEW01 changed displayed configuration/revision refuses Join before authority mutation; new preview permits Join',async()=>{
 const s=await fixture();try{
  const shown=await s.send(s.desktops[0],previewRequest(s,s.desktops[0]!)),publicRoom=shown.room!;
  await s.rpc(s.host,{configure:{...publicRoom.config!,seed:12345}});const changed=await previewAuthorityState(s);
  const join:Request={major:1,revision:'g2-draft-5',serviceId:s.f.service.db.serviceId,serviceEpoch:s.f.service.db.epoch,sessionEpoch:s.desktops[0]!.sessionEpoch!,requestId:randomUUID(),roomId:publicRoom.roomId!,expectedRevision:publicRoom.revision!,configRevision:publicRoom.configRevision!,roomJoin:{}};
  assert.equal((await s.send(s.desktops[0],join)).error?.code,'STALE_REVISION');assert.deepEqual(await previewAuthorityState(s),changed);
  const refreshed=(await s.send(s.desktops[0],previewRequest(s,s.desktops[0]!))).room!;assert.equal(refreshed.config!.seed,12345);assert.notEqual(refreshed.configRevision,publicRoom.configRevision);
  await s.join(1);const concurrent=await previewAuthorityState(s);assert.equal(s.room.configRevision,refreshed.configRevision);
  assert.equal((await s.send(s.desktops[0],{...join,requestId:randomUUID(),expectedRevision:refreshed.revision!,configRevision:refreshed.configRevision!})).error?.code,'STALE_REVISION');assert.deepEqual(await previewAuthorityState(s),concurrent);
  const latest=(await s.send(s.desktops[0],previewRequest(s,s.desktops[0]!))).room!;
  const joined=await s.send(s.desktops[0],{...join,requestId:randomUUID(),expectedRevision:latest.revision!,configRevision:latest.configRevision!});assert.equal(joined.error,undefined);assert.equal(joined.preparation!.configRevision,latest.configRevision);
 }finally{await s.close();}
});
test('FLOW-PREVIEW01 preserves private/admission/ban/game-kind boundaries and never issues or renews preparation',async()=>{
 const s=await fixture();try{
  await s.rpc(s.host,{roomAccess:{visibility:'private',admittedAccountIds:[]}});let before=await previewAuthorityState(s);
  const hidden=await s.send(s.desktops[0],previewRequest(s,s.desktops[0]!));assert.equal(hidden.error?.code,'NOT_FOUND');assert.equal(hidden.room,undefined);assert.equal(hidden.admission,undefined);assert.deepEqual(await previewAuthorityState(s),before);
  await s.rpc(s.host,{roomAccess:{visibility:'private',admittedAccountIds:[s.desktops[0]!.accountId!]}});const allowed=await s.send(s.desktops[0],previewRequest(s,s.desktops[0]!));assert.equal(allowed.error,undefined);assert.equal(allowed.admission!.visibility,'private');assert.ok(allowed.room!.config);assert.equal(allowed.preparation,undefined);
  await s.rpc(s.host,{roomModerate:{accountId:s.desktops[0]!.accountId!,action:'ban'}});before=await previewAuthorityState(s);const banned=await s.send(s.desktops[0],previewRequest(s,s.desktops[0]!));assert.equal(banned.error?.code,'NOT_FOUND');assert.equal(banned.room,undefined);assert.deepEqual(await previewAuthorityState(s),before);
  await s.rpc(s.host,{roomAccess:{visibility:'public',admittedAccountIds:[]}});assert.equal((await s.send(s.desktops[0],previewRequest(s,s.desktops[0]!))).error?.code,'NOT_FOUND');
  await s.join(1);const held=await s.hold(1);before=await previewAuthorityState(s);assert.equal((await s.send(held.g,previewRequest(s,held.g))).error?.code,'FORBIDDEN');assert.deepEqual(await previewAuthorityState(s),before);
 }finally{await s.close();}
});

test('U1-S-SEED shared random create/configure resolves once, replay/preview/read/Join/Start retain resolved seed',async()=>{
 const s=await fixture(true);try{
  const config={seed:0,seedPolicy:'shared-random',randomizeLoot:true,mode:'core',requiredCapabilities:s.gameMetadata.capabilities};
  const create=s.request(s.host,{roomCreate:{name:'Seed policy lobby',allowGuests:true,config}}),created=await s.send(s.host,create);assert.equal(created.error,undefined);
  const id=s.room.roomId,seed=s.room.config!.seed!;assert.ok(seed>=1&&seed<=4294967295);assert.equal(s.room.config!.randomizeLoot,true);
  const replay=await s.send(s.host,create);assert.equal(replay.room!.roomId,id);assert.equal(replay.room!.config!.seed,seed);assert.equal(config.seed,0);
  await s.rpc(s.desktops[0],{roomPreview:{}});assert.equal(s.room.config!.seed,seed);await s.join(0);await s.rpc(s.desktops[0],{roomGet:{}});assert.equal(s.room.config!.seed,seed);
  const apply=s.request(s.host,{configure:config}),changed=await s.send(s.host,apply);assert.equal(changed.error,undefined);const chosen=s.room.config!.seed!;assert.ok(chosen>=1&&chosen<=4294967295);assert.equal((await s.send(s.host,apply)).room!.config!.seed,chosen);
  await s.join(0);const held=await s.hold(0,{actualSeed:String(chosen+1)});assert.equal(s.room.members![0]!.readinessReason,'seed_mismatch');assert.equal((await s.send(s.host,s.request(s.host,{finalStart:s.selection([0])}))).error!.code,'NOT_READY');
  const correct={...held.report,actualSeed:String(chosen),observationRevision:2};await s.rpc(held.g,{gameObservation:correct});s.reports.set(s.desktops[0]!.sessionId!,correct);
  assert.equal((await s.send(s.host,s.request(s.host,{configure:config}))).error!.code,'INVALID_STATE');await s.rpc(s.host,{finalStart:s.selection([0])});const release=(await s.rpc(held.g,{roomGet:{}})).release!;
  assert.equal(s.room.config!.seed,chosen);assert.equal(release.actualSeed,String(chosen));assert.equal(release.seedPolicy,'shared-random');assert.equal(release.randomizeLoot,true);
 }finally{await s.close();}
});
test('U1-S-SEED individual worlds retain complete inspection, freeze distinct actual seeds and invalidate changed content',async()=>{
 const s=await fixture(true);try{
  await s.rpc(s.host,{configure:{seed:0,seedPolicy:'individual-random',randomizeLoot:false,mode:'core',requiredCapabilities:['core.seed','core.seed-policy.v1']}});await s.join(0);await s.join(1);
  const a=await s.hold(0,{actualSeed:'123'}),b=await s.hold(1,{actualSeed:'987',inventoryComplete:false,collectorFailure:'mods_unavailable'});assert.equal(s.room.members!.find(m=>m.desktopId===s.desktops[1]!.sessionId)!.readinessReason,'inventory_incomplete');
  assert.equal((await s.send(s.host,s.request(s.host,{finalStart:s.selection([0,1])}))).error!.code,'NOT_READY');
  const {actualSeed:omitted,...unknown}=b.report;await s.rpc(b.g,{gameObservation:{...unknown,seedStatus:'unknown',collectorFailure:'seed_unavailable',observationRevision:2}});assert.equal(s.room.members!.find(m=>m.desktopId===s.desktops[1]!.sessionId)!.readinessReason,'seed_unknown');
  const full={...b.report,inventoryComplete:true,collectorFailure:'',observationRevision:3};await s.rpc(b.g,{gameObservation:full});s.reports.set(s.desktops[1]!.sessionId!,full);assert.ok(s.room.members!.every(m=>m.ready&&m.seedApplied));
  await s.rpc(s.host,{finalStart:s.selection([0,1])});for(const [g,seed] of [[a.g,'123'],[b.g,'987']] as const){const release=(await s.rpc(g,{roomGet:{}})).release!;assert.equal(release.actualSeed,seed);assert.equal(release.seedPolicy,'individual-random');assert.equal(release.randomizeLoot,false);}
  const row=await s.f.service.db.transaction(tx=>tx.query('SELECT selected_reports FROM inspected_releases WHERE run_id=$1',[s.room.runId]));assert.deepEqual(row.rows[0].selected_reports.participants.map((p:any)=>p.actualSeed),['123','987']);
  await s.rpc(a.g,{gameObservation:{...a.report,actualSeed:'124',observationRevision:2}});assert.equal(s.room.state,'suspended');
 }finally{await s.close();}
});
test('U1-S-SEED typed shared loot policy requires advertised and attached capabilities; fixed mode remains exact',async()=>{
 const s=await fixture();try{
  const config={seed:55,seedPolicy:'shared-fixed',randomizeLoot:true,mode:'core',requiredCapabilities:['core.seed','core.seed-policy.v1','core.randomize-loot.v1']};
  assert.equal((await s.send(s.host,s.request(s.host,{configure:config}))).error!.code,'INCOMPATIBLE_GAME');s.f.profile.compatibility.capabilities.push('core.seed-policy.v1');
  assert.equal((await s.send(s.host,s.request(s.host,{configure:config}))).error!.code,'INCOMPATIBLE_GAME');
  s.f.profile.compatibility.capabilities.push('core.randomize-loot.v1');assert.equal((await s.send(s.host,s.request(s.host,{configure:{...config,requiredCapabilities:['core.seed']}}))).error!.code,'INCOMPATIBLE_GAME');
  await s.rpc(s.host,{configure:config});await s.join(0);const prep=(await s.rpc(s.desktops[0],{roomGet:{}})).preparation!;
  assert.equal((await s.send(s.desktops[0],s.request(s.desktops[0],{gameAttach:{preparationId:prep.preparationId!,gameInstanceId:'old-consumer',...s.gameMetadata,capabilities:['core.seed','core.seed-policy.v1']}}))).error!.code,'INCOMPATIBLE_GAME');
  const held=await s.hold(0,{actualSeed:'56'});assert.equal(s.room.members![0]!.readinessReason,'seed_mismatch');
  await s.rpc(s.host,{cancelRun:{}});await s.rpc(s.host,{configure:{...config,randomizeLoot:false,requiredCapabilities:['core.seed','core.seed-policy.v1']}});await s.join(0);await s.hold(0,{actualSeed:'55'});assert.ok(s.room.members![0]!.ready);await s.rpc(s.host,{finalStart:s.selection([0])});
 }finally{await s.close();}
});
