import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,randomBytes} from 'node:crypto';
import {generateKeyPair,exportPKCS8,exportSPKI,importPKCS8,SignJWT} from 'jose';
import pg from 'pg';
import {createG1App,type Options} from '../src/g1/app.js';
import {G1Consumer,g1Compatibility} from '../../../packages/test-kit/src/g1.js';
import type {Room,Response,Request,Credentials} from '../../../packages/contracts/g1/g1.js';
import {hash} from '../src/g1/security.js';
import {PolicyError} from '../src/g1/security.js';
import {setTimeout as delay} from 'node:timers/promises';
import {request as httpRequest} from 'node:http';
import {createServer} from 'node:net';
import {Database} from '../src/g1/database.js';
import {readFile} from 'node:fs/promises';
const url=process.env.NT2_G1_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
function room(result:Response):Room{assert.equal(result.error,undefined,JSON.stringify(result.error));assert.ok(result.room);return result.room;}
function credentials(result:Response):Credentials{assert.equal(result.error,undefined,JSON.stringify(result.error));assert.ok(result.credentials);return result.credentials;}
async function setup(overrides:Partial<Options>={},beforeStart?:(options:Options)=>Promise<void>){
 const pair=await generateKeyPair('EdDSA',{extractable:true});let now=Date.now(),available=true;
 const namespace='g1_test_'+randomUUID().replaceAll('-','');const options:Options={databaseUrl:url,namespace,serviceId:'nt2-g1-test',keys:{privateKey:await exportPKCS8(pair.privateKey),publicKey:await exportSPKI(pair.publicKey),recoveryKey:randomBytes(32).toString('base64url'),issuer:'http://127.0.0.1:4321'},mockEnabled:true,now:()=>now,providerAvailable:()=>available,compatibility:g1Compatibility,...overrides};
 if(beforeStart)await beforeStart(options);let service=await createG1App(options);const consumers:G1Consumer[]=[];
 const consumer=()=>{const c=new G1Consumer(service.app,options.serviceId,service.db.epoch);consumers.push(c);return c;};
 return {get service(){return service;},options,consumer,advance(ms:number){now+=ms;},outage(value:boolean){available=!value;},async restart(){await service.app.close();service=await createG1App(options);for(const c of consumers){c.app=service.app;c.serviceEpoch=service.db.epoch;}},async close(){await service.app.close();const pool=new pg.Pool({connectionString:url});await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);await pool.end();}};
}
const config={seed:424242,mode:'core',settings:[{key:'bankEnabled',value:'true'}],requiredCapabilities:['core.seed','core.items','core.emotes','integration.nemesis']};
test('G1 empty database, migration idempotence, namespaces, singleton and provider outage',async()=>{
 const f=await setup();try{
  await f.service.db.migrate();assert.deepEqual((await f.service.db.pool.query('SELECT version FROM schema_migrations ORDER BY version')).rows,[{version:1},{version:2},{version:3},{version:4}]);
  await assert.rejects(createG1App(f.options),/Another service owns/);
  const browser=await f.consumer().browser('host','tab-one');await f.consumer().browser('host','tab-two');
  assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM accounts')).rows[0].count,1);
  const other=await f.service.db.transaction(tx=>f.service.auth.account(tx,'other-provider','host'));assert.notEqual(other,browser.credentials!.accountId);
  f.outage(true);const response=await f.service.app.inject({method:'POST',url:'/v1/mock/browser-login',payload:{subject:'outage',deviceId:'tab',requestId:'outage'}});assert.equal(response.json().error.code,'PROVIDER_UNAVAILABLE');assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM accounts')).rows[0].count,2);
 }finally{await f.close();}
});
test('G1 pairing approval, device/proof ownership, cancellation, expiry and durable exchange retry',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');const guest=await f.consumer().browser('guest','browser');const d=f.consumer();
  const p=(await d.rpc({pairBegin:{deviceId:'desktop'}})).pairing!;
  assert.equal((await browser.rpc({pairApprove:{attemptId:p.attemptId!,code:'wrong'}})).error?.code,'FORBIDDEN');
  assert.equal((await browser.rpc({pairApprove:{attemptId:p.attemptId!,code:p.code!}})).pairing?.state,'approved');
  assert.equal((await guest.rpc({pairApprove:{attemptId:p.attemptId!,code:p.code!}})).error?.code,'CONFLICT');
  const proof={attemptId:p.attemptId!,deviceId:'desktop',proof:p.proof!};
  assert.equal((await d.rpc({pairExchange:{...proof,deviceId:'wrong'}})).error?.code,'FORBIDDEN');
  const exact=d.request({pairExchange:proof});const first=credentials(await d.send(exact));await f.restart();assert.deepEqual(credentials(await d.send(exact)),first);
  assert.equal((await d.rpc({pairExchange:proof})).error?.code,'ALREADY_EXCHANGED');f.advance(30001);assert.equal((await d.send(exact)).error?.code,'ALREADY_EXCHANGED');
  const p2=(await d.rpc({pairBegin:{deviceId:'other'}})).pairing!;const p2proof={attemptId:p2.attemptId!,proof:p2.proof!,deviceId:'other'};await d.rpc({pairCancel:p2proof});assert.equal((await d.rpc({pairExchange:p2proof})).error?.code,'CANCELLED');
  const p3=(await d.rpc({pairBegin:{deviceId:'expires'}})).pairing!;f.advance(60001);const poll=await d.rpc({pairPoll:{attemptId:p3.attemptId!,proof:p3.proof!,deviceId:'expires'}});assert.equal(poll.pairing?.state,'expired');assert.equal(poll.pairing?.proof,undefined);
 }finally{await f.close();}
});
test('G1 transactional refresh, concurrent exact retry, restart recovery, reused secret revocation and logout',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');const d=await f.consumer().desktop(browser,'desktop');const original=d.credentials!;
  const req=d.request({refresh:{secret:original.refreshSecret!,deviceId:'desktop'}});const [a,b]=await Promise.all([d.send(req),d.send(req)]);assert.deepEqual(credentials(a),credentials(b));assert.notEqual(a.credentials!.refreshSecret,original.refreshSecret);
  assert.equal((await d.rpc({roomList:{}})).error?.code,'UNAUTHENTICATED');d.credentials=a.credentials!;await f.restart();assert.deepEqual(credentials(await d.send(req)),a.credentials);
  const row=(await f.service.db.pool.query('SELECT recovery_ciphertext FROM refresh_tokens WHERE hash=$1',[hash(original.refreshSecret!)])).rows[0];assert.ok(!row.recovery_ciphertext.includes(a.credentials!.refreshSecret));
  const reuse=await d.rpc({refresh:{secret:original.refreshSecret!,deviceId:'desktop'}});assert.equal(reuse.error?.code,'REFRESH_REUSED');assert.equal((await d.rpc({roomList:{}})).error?.code,'UNAUTHENTICATED');
  browser.serviceEpoch=f.service.db.epoch;const logout=browser.request({logout:{}});assert.equal((await browser.send(logout)).error,undefined);await f.restart();assert.equal((await browser.send(logout)).error,undefined);assert.equal((await browser.rpc({roomList:{}})).error?.code,'UNAUTHENTICATED');
 }finally{await f.close();}
});
test('G1 browser, two desktop and two attached games: revision fencing, frozen roster, multiple rooms and real game-only loading',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');const guestBrowser=await f.consumer().browser('guest','browser');const a=await f.consumer().desktop(browser,'a'),b=await f.consumer().desktop(guestBrowser,'b');
  let r=room(await browser.rpc({roomCreate:{name:'First',config}},undefined,true));const other=room(await browser.rpc({roomCreate:{name:'Second',config}}));
  assert.equal((await guestBrowser.rpc({roomGet:{}},other)).error?.code,'FORBIDDEN');
  r=room(await a.rpc({roomJoin:{}},r));r=room(await b.rpc({roomJoin:{}},r));assert.equal((await a.rpc({roomJoin:{}},other)).error?.code,'CONFLICT');
  assert.equal((await browser.rpc({ready:{ready:true}},r)).error?.code,'FORBIDDEN');
  const stale=r;r=room(await a.rpc({ready:{ready:true}},r));assert.equal((await b.rpc({ready:{ready:true}},stale)).error?.code,'STALE_REVISION');r=room(await b.rpc({ready:{ready:true}},r));
  assert.equal((await browser.rpc({start:{participantDesktopIds:[a.credentials!.sessionId!],excludedDesktopIds:[]}},r)).error?.code,'ROSTER_MISMATCH');
  r=room(await browser.rpc({start:{participantDesktopIds:[a.credentials!.sessionId!,b.credentials!.sessionId!],excludedDesktopIds:[]}},r));assert.equal(r.state,'preparing');
  assert.equal((await browser.rpc({configure:config},r)).error?.code,'INVALID_STATE');const late=await f.consumer().desktop(guestBrowser,'late');assert.equal((await late.rpc({roomJoin:{}},r)).error?.code,'INVALID_STATE');
  const attach=(instance:string)=>({gameInstanceId:instance,gameVersion:g1Compatibility.gameVersion,modVersion:g1Compatibility.modVersion,bridgeVersion:g1Compatibility.bridgeVersion,capabilities:g1Compatibility.capabilities});
  assert.equal((await a.rpc({gameAttach:{...attach('a-game'),modVersion:'bad'}},r)).error?.code,'INCOMPATIBLE_GAME');
  const ar=await a.rpc({gameAttach:attach('a-game')},r);r=room(ar);const ga=f.consumer();ga.credentials=credentials(ar);
  const br=await b.rpc({gameAttach:attach('b-game')},r);r=room(br);const gb=f.consumer();gb.credentials=credentials(br);
  assert.equal((await ga.rpc({roomGet:{}},other)).error?.code,'FORBIDDEN');
  const af={attachmentId:ga.credentials!.attachmentId!,gameInstanceId:'a-game'},bf={attachmentId:gb.credentials!.attachmentId!,gameInstanceId:'b-game'};
  assert.equal((await a.rpc({gameSeedApplied:af},r)).error?.code,'FORBIDDEN');assert.equal((await ga.rpc({gameLoaded:af},r)).error?.code,'INVALID_STATE');
  assert.equal((await ga.rpc({gameSeedApplied:bf},r)).error?.code,'RESTART_REQUIRED');assert.equal((await ga.rpc({gameSeedApplied:af},{...r,runEpoch:randomUUID()})).error?.code,'STALE_EPOCH');
  r=room(await ga.rpc({gameSeedApplied:af},r, true));assert.equal(r.state,'preparing');r=room(await gb.rpc({gameSeedApplied:bf},r));assert.equal(r.state,'loading');
  r=room(await ga.rpc({gameLoaded:af},r));assert.equal(r.state,'loading');r=room(await gb.rpc({gameLoaded:bf},r));assert.equal(r.state,'running');assert.equal(r.config!.seed,424242);
  const resume=ga.request({gameResume:af},r);const resumed=await ga.send(resume);assert.ok(resumed.credentials);assert.deepEqual((await ga.send(resume)).credentials,resumed.credentials);ga.credentials=resumed.credentials!;
  const oldEpoch=f.service.db.epoch;await f.restart();assert.notEqual(f.service.db.epoch,oldEpoch);r=room(await browser.rpc({roomGet:{}},r));assert.equal(r.state,'suspended');assert.ok(r.members!.every(m=>!m.ready&&!m.present&&!m.attached&&!m.loaded));assert.equal((await ga.rpc({gameLoaded:af},r)).error?.code,'UNAUTHENTICATED');
  assert.equal(room(await browser.rpc({roomGet:{}},other)).state,'waiting');
  r=room(await browser.rpc({endRun:{}},r));assert.equal(r.state,'ended');assert.equal(r.config!.seed,424242);
 }finally{await f.close();}
});
test('G1 preparation expiry/restart cancellation and desktop supersession invalidate attachments',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');const a=await f.consumer().desktop(browser,'a');let r=room(await browser.rpc({roomCreate:{name:'Expiry',config}}));r=room(await a.rpc({roomJoin:{}},r));
  const start=async()=>{r=room(await a.rpc({presence:{present:true}},r));r=room(await a.rpc({ready:{ready:true}},r));r=room(await browser.rpc({start:{participantDesktopIds:[a.credentials!.sessionId!],excludedDesktopIds:[]}},r));};
  await start();await f.restart();r=room(await browser.rpc({roomGet:{}},r));assert.equal(r.state,'waiting');assert.equal(r.members![0]!.ready,false);
  await start();f.advance(120001);r=room(await browser.rpc({roomGet:{}},r));assert.equal(r.state,'waiting');assert.equal((await f.service.db.pool.query('SELECT reason FROM runs WHERE id=$1',[r.runId])).rows[0].reason,'preparation_expired');
  await start();const ga=f.consumer();const attached=await a.rpc({gameAttach:{gameInstanceId:'game',gameVersion:g1Compatibility.gameVersion,modVersion:g1Compatibility.modVersion,bridgeVersion:g1Compatibility.bridgeVersion,capabilities:g1Compatibility.capabilities}},r);ga.credentials=credentials(attached);r=room(attached);
  await f.consumer().desktop(browser,'a');assert.equal((await a.rpc({roomGet:{}},r)).error?.code,'UNAUTHENTICATED');assert.equal((await ga.rpc({gameSeedApplied:{attachmentId:ga.credentials!.attachmentId!,gameInstanceId:'game'}},r)).error?.code,'UNAUTHENTICATED');r=room(await browser.rpc({roomGet:{}},r));assert.equal(r.state,'waiting');
 }finally{await f.close();}
});
test('G1 malformed, major/service/scope mismatch and expired access fail without mutation',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');
  for(const [payload,code] of [[{major:2},'VERSION_MISMATCH'],[{serviceId:'other'},'SERVICE_MISMATCH'],[{serviceEpoch:'old'},'STALE_EPOCH'],[{sessionEpoch:'old'},'STALE_EPOCH']] as const){assert.equal((await browser.rpc({roomList:{},...payload})).error?.code,code);}
  const response=await f.service.app.inject({method:'POST',url:'/v1/rpc',payload:{...browser.request({roomList:{}}),sender:'host'}});assert.equal(response.json().error.code,'INVALID_MESSAGE');
  f.advance(900001);assert.equal((await browser.rpc({roomList:{}})).error?.code,'UNAUTHENTICATED');assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM rooms')).rows[0].count,0);
 }finally{await f.close();}
});
test('G1 competing Start, durable exclusions, exact retry and whole-run cancellation',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');const a=await f.consumer().desktop(browser,'a'),b=await f.consumer().desktop(browser,'b');let r=room(await browser.rpc({roomCreate:{name:'Race',config}}));r=room(await a.rpc({roomJoin:{}},r));r=room(await b.rpc({roomJoin:{}},r));r=room(await a.rpc({ready:{ready:true}},r));
  const payload={start:{participantDesktopIds:[a.credentials!.sessionId!],excludedDesktopIds:[b.credentials!.sessionId!]}};
  const exact=browser.request(payload,r),competing=browser.request(payload,r);const results=await Promise.all([browser.send(exact),browser.send(competing)]);assert.equal(results.filter(x=>x.room).length,1);assert.equal(results.filter(x=>x.error?.code==='STALE_REVISION').length,1);r=room(results.find(x=>x.room)!);
  const winner=results[0]!.room?exact:competing;assert.deepEqual((await browser.send(winner)).room,r);assert.equal((await browser.send({...winner,start:{participantDesktopIds:[],excludedDesktopIds:[]}})).error?.code,'CONFLICT');
  assert.deepEqual((await f.service.db.pool.query('SELECT excluded_roster FROM runs WHERE id=$1',[r.runId])).rows[0].excluded_roster,[b.credentials!.sessionId]);
  assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM runs')).rows[0].count,1);const old=r;r=room(await browser.rpc({cancelRun:{}},r));assert.equal(r.state,'waiting');assert.ok(r.members!.every(m=>!m.ready&&!m.selected));assert.equal((await browser.send(winner)).error?.code,'CANCELLED');
  assert.equal((await a.rpc({gameAttach:{gameInstanceId:'late',gameVersion:g1Compatibility.gameVersion,modVersion:g1Compatibility.modVersion,bridgeVersion:g1Compatibility.bridgeVersion,capabilities:g1Compatibility.capabilities}},old)).error?.code,'STALE_REVISION');
 }finally{await f.close();}
});
test('G1 JWT purpose/issuer/audience/algorithm and durable account disablement',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');const c=browser.credentials!;const original=c.accessToken!;const key=await importPKCS8(f.options.keys.privateKey,'EdDSA');const parsed=await f.service.auth.security.verify(original,f.service.auth.now());
  for(const change of [{purpose:'refresh'},{iss:'http://wrong-service'},{aud:'wrong-service'},{kind:'game'},{sessionEpoch:randomUUID()}]){
   c.accessToken=await new SignJWT({...parsed,...change}).setProtectedHeader({alg:'EdDSA',typ:'JWT'}).sign(key);assert.equal((await browser.rpc({roomList:{}})).error?.code,'UNAUTHENTICATED');
  }
  c.accessToken=await new SignJWT(parsed).setProtectedHeader({alg:'HS256',typ:'JWT'}).sign(randomBytes(32));assert.equal((await browser.rpc({roomList:{}})).error?.code,'UNAUTHENTICATED');c.accessToken=original;
  await f.service.db.pool.query('UPDATE accounts SET disabled=true WHERE id=$1',[c.accountId]);await f.restart();assert.equal((await browser.rpc({roomList:{}})).error?.code,'UNAUTHENTICATED');assert.equal((await browser.rpc({refresh:{secret:c.refreshSecret!,deviceId:c.deviceId!}})).error?.code,'UNAUTHENTICATED');
  const badOrigin=await f.service.app.inject({method:'GET',url:'/v1/info',headers:{origin:'http://evil.invalid'}});assert.equal(badOrigin.statusCode,403);assert.equal((await f.service.app.inject({method:'GET',url:'/v1/info?token=forbidden'})).statusCode,403);
 }finally{await f.close();}
});
test('G1 lost refresh response outside recovery window revokes rather than guessing a new rotation',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');const old=browser.credentials!;const exact=browser.request({refresh:{secret:old.refreshSecret!,deviceId:old.deviceId!}});const result=credentials(await browser.send(exact));browser.credentials=result;f.advance(30001);assert.equal((await browser.send(exact)).error?.code,'REFRESH_REUSED');await f.restart();assert.equal((await browser.rpc({roomList:{}})).error?.code,'UNAUTHENTICATED');
 }finally{await f.close();}
});
test('G1 same-context game liveness resumes within grace, replaces epoch and never resumes a new/expired world',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser'),desktop=await f.consumer().desktop(browser,'desktop');let r=room(await browser.rpc({roomCreate:{name:'Grace',config}}));r=room(await desktop.rpc({roomJoin:{}},r));r=room(await desktop.rpc({ready:{ready:true}},r));r=room(await browser.rpc({start:{participantDesktopIds:[desktop.credentials!.sessionId!],excludedDesktopIds:[]}},r));
  const result=await desktop.rpc({gameAttach:{gameInstanceId:'same-context',gameVersion:g1Compatibility.gameVersion,modVersion:g1Compatibility.modVersion,bridgeVersion:g1Compatibility.bridgeVersion,capabilities:g1Compatibility.capabilities}},r);const game=f.consumer();game.credentials=credentials(result);r=room(result);const fact={attachmentId:game.credentials.attachmentId!,gameInstanceId:'same-context'};
  r=room(await game.rpc({gameSeedApplied:fact},r));r=room(await game.rpc({gameLoaded:fact},r));assert.equal(r.state,'running');f.advance(31000);r=room(await browser.rpc({roomGet:{}},r));assert.equal(r.state,'suspended');
  assert.equal((await game.rpc({gameResume:{...fact,gameInstanceId:'new-world'}},r)).error?.code,'RESTART_REQUIRED');const oldEpoch=game.credentials.sessionEpoch;const resume=await game.rpc({gameResume:fact},r);game.credentials=credentials(resume);assert.notEqual(game.credentials.sessionEpoch,oldEpoch);r=room(resume);assert.equal(r.state,'running');assert.equal(r.members![0]!.loaded,true);
  f.advance(60001);r=room(await browser.rpc({roomGet:{}},r));assert.equal(r.state,'suspended');assert.equal((await game.rpc({gameResume:fact},r)).error?.code,'RESTART_REQUIRED');
 }finally{await f.close();}
});
test('G1 loss of leader connection refuses mutations until a new service acquires ownership',async()=>{
 const f=await setup();try{
  const browser=await f.consumer().browser('host','browser');
  const terminated=await f.service.db.pool.query("SELECT pg_terminate_backend(pid) AS terminated FROM pg_locks WHERE locktype='advisory' AND granted AND objid=((hashtext($1)::bigint & 4294967295)::oid)",['nt2-g1-service:'+f.options.namespace]);assert.equal(terminated.rowCount,1);
  await new Promise(resolve=>setTimeout(resolve,20));assert.equal(f.service.db.ready,false);assert.equal((await browser.rpc({roomList:{}})).error?.code,'SERVICE_UNAVAILABLE');assert.equal((await f.service.app.inject({method:'GET',url:'/v1/info'})).statusCode,503);await f.restart();assert.equal((await browser.rpc({roomList:{}})).error,undefined);
 }finally{await f.close();}
});
test('G1 independent-account discovery/admission never needs a host or private member snapshot',async()=>{
 const f=await setup();try{
  const host=await f.consumer().browser('host','host-tab'),guestBrowser=await f.consumer().browser('independent-guest','guest-tab'),guest=await f.consumer().desktop(guestBrowser,'guest-device');assert.notEqual(host.credentials!.accountId,guest.credentials!.accountId);
  let publicRoom=room(await host.rpc({roomCreate:{name:'Discoverable',config,visibility:'public'}}));const hidden=room(await host.rpc({roomCreate:{name:'Hidden',config,visibility:'private'}}));await host.rpc({roomCreate:{name:'Second public',config}});
  const first=await guest.rpc({roomList:{limit:1}});assert.equal(first.rooms!.length,1);assert.ok(first.nextRoomId);const second=await guest.rpc({roomList:{limit:1,afterRoomId:first.nextRoomId!}});assert.equal(second.rooms!.length,1);assert.equal(second.nextRoomId,'');assert.ok([...first.rooms!,...second.rooms!].every(s=>s.roomId!==hidden.roomId));
  assert.equal((await guest.rpc({roomId:publicRoom.roomId!,roomGet:{}})).error?.code,'FORBIDDEN');assert.equal((await guest.rpc({roomId:hidden.roomId!,roomPreview:{}})).error?.code,'NOT_FOUND');assert.equal((await guest.rpc({roomJoin:{}},hidden)).error?.code,'NOT_FOUND');
  const preview=async(id:string)=>{const result=await guest.rpc({roomId:id,roomPreview:{}});assert.equal(result.error,undefined);assert.ok(result.admission);assert.equal(result.room,undefined);assert.deepEqual(Object.keys(result.admission).sort(),['configRevision','joinAllowed','name','revision','roomId','state','visibility'].sort());return result.admission;};
  const p=await preview(publicRoom.roomId!);assert.equal(p.joinAllowed,true);publicRoom=room(await host.rpc({configure:{...config,seed:123}},publicRoom));
  const join=(s:NonNullable<Response['admission']>)=>guest.rpc({roomId:s.roomId!,expectedRevision:s.revision!,configRevision:s.configRevision!,roomJoin:{}});
  assert.equal((await join(p)).error?.code,'STALE_REVISION');assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM memberships')).rows[0].count,0);
  const current=await preview(publicRoom.roomId!);const joined=room(await join(current));assert.equal(joined.config!.seed,123);assert.equal(joined.members![0]!.accountId,guest.credentials!.accountId);assert.equal(joined.members![0]!.desktopId,guest.credentials!.sessionId);
  assert.equal((await guest.rpc({roomId:publicRoom.roomId!,roomGet:{}})).error,undefined);assert.equal((await guest.rpc({roomList:{limit:33}})).error?.code,'INVALID_MESSAGE');assert.equal((await guest.rpc({roomList:{afterRoomId:'invalid'}})).error?.code,'INVALID_MESSAGE');
  assert.equal((await guestBrowser.rpc({roomJoin:{}},publicRoom)).error?.code,'FORBIDDEN');
 }finally{await f.close();}
});
test('G1 private account grants are authoritative across preview/revoke races and closed/full-room admission',async()=>{
 const f=await setup();try{
  const host=await f.consumer().browser('host','tab'),browser=await f.consumer().browser('guest','tab'),guest=await f.consumer().desktop(browser,'guest');let r=room(await host.rpc({roomCreate:{name:'Private',config,visibility:'private'}}));
  r=room(await host.rpc({roomAccess:{visibility:'private',admittedAccountIds:[guest.credentials!.accountId!]}},r));const p=(await guest.rpc({roomId:r.roomId!,roomPreview:{}})).admission!;assert.equal(p.joinAllowed,true);
  r=room(await host.rpc({roomAccess:{visibility:'private',admittedAccountIds:[]}},r));assert.equal((await guest.rpc({roomId:p.roomId!,expectedRevision:p.revision!,configRevision:p.configRevision!,roomJoin:{}})).error?.code,'NOT_FOUND');
  assert.equal((await guest.rpc({roomList:{}})).rooms!.length,0);assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM memberships')).rows[0].count,0);
  r=room(await host.rpc({roomAccess:{visibility:'private',admittedAccountIds:[guest.credentials!.accountId!]}},r));const fresh=(await guest.rpc({roomId:r.roomId!,roomPreview:{}})).admission!;r=room(await guest.rpc({roomId:fresh.roomId!,expectedRevision:fresh.revision!,configRevision:fresh.configRevision!,roomJoin:{}}));assert.equal(r.members!.length,1);
  const other=room(await host.rpc({roomCreate:{name:'Other',config}}));assert.equal((await guest.rpc({roomId:other.roomId!,roomPreview:{}})).admission!.joinAllowed,false);assert.equal((await guest.rpc({roomJoin:{}},other)).error?.code,'CONFLICT');
  r=room(await guest.rpc({ready:{ready:true}},r));r=room(await host.rpc({start:{participantDesktopIds:[guest.credentials!.sessionId!],excludedDesktopIds:[]}},r));const late=await f.consumer().desktop(browser,'late');assert.equal((await late.rpc({roomId:r.roomId!,roomPreview:{}})).admission!.joinAllowed,false);assert.equal((await late.rpc({roomJoin:{}},r)).error?.code,'INVALID_STATE');
  const full=room(await host.rpc({roomCreate:{name:'Full',config}}));await f.service.db.pool.query("INSERT INTO devices(account_id,device_id) VALUES($1,'capacity-fixture') ON CONFLICT DO NOTHING",[host.credentials!.accountId]);
  await f.service.db.transaction(async tx=>{for(let i=0;i<32;i++){const session=randomUUID();await tx.query("INSERT INTO sessions(id,family_id,kind,epoch) VALUES($1,$2,'desktop',$3)",[session,host.credentials!.sessionId? (await tx.query('SELECT family_id FROM sessions WHERE id=$1',[host.credentials!.sessionId])).rows[0].family_id:null,randomUUID()]);await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id) VALUES($1,$2,$3)',[session,full.roomId,host.credentials!.accountId]);}});
  assert.equal((await late.rpc({roomId:full.roomId!,roomPreview:{}})).admission!.joinAllowed,false);assert.equal((await late.rpc({roomJoin:{}},full)).error?.code,'CAPACITY');
 }finally{await f.close();}
});
test('G1 namespace lock stall bounds 40 requests, finite sweep work, timeout rollback and recovery',async()=>{
 const f=await setup();const locker=new pg.Client({connectionString:url});await locker.connect();
 try{
  await locker.query('BEGIN');await locker.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g1-mutation:'+f.options.namespace]);
  const c=f.consumer(),start=Date.now();const requests=Array.from({length:40},(_,i)=>c.rpc({pairBegin:{deviceId:'queued_'+i}}));
  await delay(60);assert.ok(f.service.db.workStats.active<=7);assert.ok(f.service.requestGate.stats.active<=16);assert.ok(f.service.db.workStats.poolWaiting<=7);
  const results=await Promise.all(requests);assert.ok(Date.now()-start<4000);assert.ok(results.some(r=>r.error?.code==='CAPACITY'));assert.ok(results.some(r=>r.error?.code==='SERVICE_UNAVAILABLE'));assert.ok(results.every(r=>r.error?.retryAfterMs===250));
  const before=f.service.rooms.sweepStats.started;const sweeps=Array.from({length:20},()=>f.service.rooms.sweep());assert.ok(sweeps.every(p=>p===sweeps[0]));await delay(2200);await Promise.allSettled(sweeps);assert.ok(f.service.rooms.sweepStats.started-before<=4);assert.ok(f.service.rooms.sweepStats.running<=1);assert.ok(f.service.db.workStats.peak<=7);assert.equal(f.service.db.workStats.active,0);assert.equal(f.service.db.workStats.poolWaiting,0);assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM pairing_attempts')).rows[0].count,0);
  await locker.query('ROLLBACK');const recovered=await c.rpc({pairBegin:{deviceId:'recovered'}});assert.ok(recovered.pairing);assert.equal(f.service.requestGate.stats.active,0);
 }finally{await locker.query('ROLLBACK').catch(()=>{});await locker.end();await f.close();}
});
test('G1 connection acquisition deadline drains waiting entries and prevents late orphaned transactions',async()=>{
 const f=await setup({databaseBudget:{maxWork:2,acquireMs:100,transactionMs:500}});const held:pg.PoolClient[]=[];
 try{
  for(let i=0;i<7;i++)held.push(await f.service.db.pool.connect());
  const calls=Array.from({length:10},()=>f.service.db.transaction(async tx=>{await tx.query("INSERT INTO accounts(id,provider_namespace,provider_subject) VALUES($1,'late-acquire',$2)",[randomUUID(),randomUUID()]);}));const results=await Promise.allSettled(calls);assert.equal(results.filter(r=>r.status==='fulfilled').length,0);assert.ok(results.some(r=>r.status==='rejected'&&r.reason instanceof PolicyError&&r.reason.code==='CAPACITY'));assert.ok(f.service.db.workStats.peak<=2);assert.equal(f.service.db.workStats.poolWaiting,0);assert.equal(f.service.db.workStats.active,0);
  for(const client of held.splice(0))client.release();await delay(150);assert.equal(+(await f.service.db.pool.query("SELECT count(*) FROM accounts WHERE provider_namespace='late-acquire'")).rows[0].count,0);await f.service.db.transaction(tx=>tx.query('SELECT 1'));
 }finally{for(const client of held)client.release();await f.close();}
});
test('G1 statement/transaction deadlines roll back stalled mutations and release work',async()=>{
 const f=await setup({databaseBudget:{statementMs:100,lockMs:100,transactionMs:350}});try{
  await assert.rejects(f.service.db.transaction(async tx=>{await tx.query("INSERT INTO accounts VALUES($1,'timeout','statement',false)",[randomUUID()]);await tx.query('SELECT pg_sleep(2)');}),e=>e instanceof PolicyError&&e.code==='SERVICE_UNAVAILABLE');
  await assert.rejects(f.service.db.transaction(async tx=>{await tx.query("INSERT INTO accounts VALUES($1,'timeout','transaction',false)",[randomUUID()]);await delay(500);await tx.query("INSERT INTO accounts VALUES($1,'timeout','late-write',false)",[randomUUID()]);}),e=>e instanceof PolicyError&&e.code==='SERVICE_UNAVAILABLE');
  await delay(300);assert.equal(+(await f.service.db.pool.query("SELECT count(*) FROM accounts WHERE provider_namespace='timeout'")).rows[0].count,0);assert.equal(f.service.db.workStats.active,0);assert.equal(f.service.db.workStats.poolWaiting,0);await f.service.db.transaction(tx=>tx.query('SELECT 1'));
 }finally{await f.close();}
});
test('G1 disconnected HTTP caller and shutdown abort work without a late pairing mutation',async()=>{
 const f=await setup();const locker=new pg.Client({connectionString:url});await locker.connect();
 try{
  await f.service.app.listen({host:'127.0.0.1',port:0});const address=f.service.app.server.address();assert.ok(address&&typeof address!=='string');
  await locker.query('BEGIN');await locker.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g1-mutation:'+f.options.namespace]);
  const payload=JSON.stringify(f.consumer().request({pairBegin:{deviceId:'abandoned'}}));const call=httpRequest({host:'127.0.0.1',port:address.port,path:'/v1/rpc',method:'POST',headers:{'content-type':'application/json','content-length':Buffer.byteLength(payload)}});call.on('error',()=>{});call.end(payload);await delay(50);call.destroy();await delay(100);assert.equal(f.service.db.workStats.active,0);assert.equal(f.service.requestGate.stats.active,0);
  const pending=f.consumer().rpc({pairBegin:{deviceId:'shutdown'}});await delay(30);const start=Date.now();await f.service.app.close();assert.ok(Date.now()-start<1500);assert.equal((await pending).error?.code,'SERVICE_UNAVAILABLE');assert.equal(f.service.db.workStats.active,0);assert.equal(f.service.db.workStats.poolWaiting,0);assert.equal(f.service.db.workStats.poolTotal,0);
  await locker.query('ROLLBACK');assert.equal(+(await locker.query(`SELECT count(*) FROM "${f.options.namespace}".pairing_attempts`)).rows[0].count,0);
 }finally{await locker.query('ROLLBACK').catch(()=>{});await locker.end();await f.close();}
});
test('G1 request deadline and finite request admission cancel work before a long database lock timeout',async()=>{
 const f=await setup({requestBudget:1,requestDeadlineMs:100,databaseBudget:{lockMs:2000,statementMs:2200,transactionMs:2500}});const locker=new pg.Client({connectionString:url});await locker.connect();
 try{
  await locker.query('BEGIN');await locker.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g1-mutation:'+f.options.namespace]);
  const c=f.consumer();const start=Date.now(),pending=c.rpc({pairBegin:{deviceId:'http-deadline'}});await delay(30);const overflow=await c.rpc({pairBegin:{deviceId:'overflow'}});assert.equal(overflow.error?.code,'CAPACITY');const expired=await pending;assert.equal(expired.error?.code,'SERVICE_UNAVAILABLE');assert.ok(Date.now()-start<1500);assert.equal(f.service.requestGate.stats.peak,1);assert.equal(f.service.requestGate.stats.active,0);assert.equal(f.service.db.workStats.active,0);
  await locker.query('ROLLBACK');assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM pairing_attempts')).rows[0].count,0);assert.ok((await c.rpc({pairBegin:{deviceId:'after-deadline'}})).pairing);
 }finally{await locker.query('ROLLBACK').catch(()=>{});await locker.end();await f.close();}
});
test('G1 unresponsive database handshake has a finite acquisition deadline and clean shutdown',async()=>{
 const sockets=new Set<import('node:net').Socket>();const server=createServer(socket=>{sockets.add(socket);socket.on('close',()=>sockets.delete(socket));});await new Promise<void>(resolve=>server.listen(0,'127.0.0.1',resolve));const address=server.address();assert.ok(address&&typeof address!=='string');
 const db=new Database(`postgres://unresponsive:fixture@127.0.0.1:${address.port}/fixture`,'handshake-fixture','public',{acquireMs:100});
 try{const start=Date.now();await assert.rejects(db.migrate());assert.ok(Date.now()-start<1500);await db.close();assert.equal(db.workStats.poolWaiting,0);assert.equal(db.workStats.poolTotal,0);}finally{await db.close();for(const socket of sockets)socket.destroy();await new Promise<void>(resolve=>server.close(()=>resolve()));}
});
test('G1 migration004 preserves prior draft rooms privately until their host explicitly publishes them',async()=>{
 const priorId=randomUUID(),accountId=randomUUID();const f=await setup({},async options=>{
  const tx=new pg.Client({connectionString:url});await tx.connect();try{await tx.query(`CREATE SCHEMA "${options.namespace}"`);await tx.query(`SET search_path TO "${options.namespace}",pg_catalog`);
   for(const name of ['001_g1_identity.sql','002_g1_rooms.sql','003_g1_frozen_exclusions.sql'])await tx.query(await readFile(new URL('../../../db/migrations/'+name,import.meta.url),'utf8'));
   await tx.query("INSERT INTO accounts VALUES($1,'nt2-g1-mock','prior-host',false)",[accountId]);await tx.query("INSERT INTO rooms(id,name,host_account_id,state,config) VALUES($1,'Unpublished prior room',$2,'waiting',$3)",[priorId,accountId,config]);
  }finally{await tx.end();}
 });try{
  const host=await f.consumer().browser('prior-host','host'),browser=await f.consumer().browser('new-guest','guest'),guest=await f.consumer().desktop(browser,'guest');assert.equal(host.credentials!.accountId,accountId);
  const prior=room(await host.rpc({roomId:priorId,roomGet:{}}));assert.deepEqual(prior.config,config);assert.equal((await guest.rpc({roomList:{}})).rooms!.length,0);assert.equal((await guest.rpc({roomId:priorId,roomPreview:{}})).error?.code,'NOT_FOUND');
  await host.rpc({roomAccess:{visibility:'public',admittedAccountIds:[]}},prior);assert.equal((await guest.rpc({roomList:{}})).rooms![0]!.roomId,priorId);assert.equal((await guest.rpc({roomId:priorId,roomPreview:{}})).admission!.joinAllowed,true);
 }finally{await f.close();}
});
test('G1 cancelled acquisition retains its permit until the waiter is removed and never invokes stale work',async()=>{
 const f=await setup({databaseBudget:{maxWork:1,acquireMs:500}});const held:pg.PoolClient[]=[];
 try{
  for(let i=0;i<7;i++)held.push(await f.service.db.pool.connect());const controller=new AbortController();let invoked=false;
  const pending=f.service.db.transaction(async()=>{invoked=true;},true,controller.signal);const settled=assert.rejects(pending,e=>e instanceof PolicyError&&e.code==='SERVICE_UNAVAILABLE');controller.abort();
  assert.equal(f.service.db.workStats.active,1);assert.equal(f.service.db.workStats.poolWaiting,1);await assert.rejects(f.service.db.transaction(tx=>tx.query('SELECT 1')),e=>e instanceof PolicyError&&e.code==='CAPACITY');held.pop()!.release();await settled;
  assert.equal(invoked,false);assert.equal(f.service.db.workStats.active,0);assert.equal(f.service.db.workStats.poolWaiting,0);for(const client of held.splice(0))client.release();await f.service.db.transaction(tx=>tx.query('SELECT 1'));
 }finally{for(const client of held)client.release();await f.close();}
});
