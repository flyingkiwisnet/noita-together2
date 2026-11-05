import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,randomBytes,createHash} from 'node:crypto';
import {generateKeyPair,exportPKCS8,exportSPKI} from 'jose';
import pg from 'pg';
import {createG2App} from '../src/g2/app.js';
import {G2Consumer,g2Compatibility} from '../../../packages/test-kit/src/g2.js';
import type {Room,Response} from '../../../packages/contracts/g2/g2.js';
// Production browser client with injected HTTP transport, not a replacement host authority.
// @ts-expect-error Browser module intentionally has no Node-only dependency/type layer.
import {HostClient} from '../../web/public/client.js';
// @ts-expect-error Browser controller is exercised through its production handlers.
import {createHostController} from '../../web/public/app.js';
// @ts-expect-error Test-only DOM adapter; authorization stays in the real service.
import {controllerDom} from '../../web/test/dom-fixture.mjs';
const url=process.env.NT2_G2_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
const config={seed:424242,mode:'core',settings:[],requiredCapabilities:['core.seed']};
function room(result:Response):Room{assert.equal(result.error,undefined,JSON.stringify(result.error));assert.ok(result.room);return result.room;}
async function setup(){
 const pair=await generateKeyPair('EdDSA',{extractable:true});let now=Date.now();const namespace='g2_test_'+randomUUID().replaceAll('-','');
 const options={databaseUrl:url,namespace,serviceId:'nt2-g2-test',keys:{privateKey:await exportPKCS8(pair.privateKey),publicKey:await exportSPKI(pair.publicKey),recoveryKey:randomBytes(32).toString('base64url'),issuer:'http://127.0.0.1:4351'},mockEnabled:true,now:()=>now,compatibility:g2Compatibility,website:true};
 let service=await createG2App(options);const consumers:G2Consumer[]=[];const consumer=()=>{const c=new G2Consumer(service.app,options.serviceId,service.db.epoch);consumers.push(c);return c;};
 return {get service(){return service;},consumer,advance(ms:number){now+=ms;},async restart(){await service.app.close();service=await createG2App(options);for(const c of consumers){c.app=service.app;c.serviceEpoch=service.db.epoch;}},async close(){await service.app.close();const pool=new pg.Pool({connectionString:url});await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);await pool.end();}};
}
function hostTransport(f:Awaited<ReturnType<typeof setup>>){return async(path:string,body?:unknown,token?:string)=>{
 const response=await f.service.app.inject({method:body?'POST':'GET',url:path,...(body?{payload:body}:{}),headers:{...(token?{authorization:'Bearer '+token}:{})}});assert.equal(response.statusCode,200);return response.json();
};}
async function joined(f:Awaited<ReturnType<typeof setup>>){
 const host=await f.consumer().browser('host','browser'),guest=await f.consumer().browser('guest','browser');const a=await f.consumer().desktop(host,'a'),b=await f.consumer().desktop(guest,'b');
 let r=room(await host.rpc({roomCreate:{name:'Website expedition',config}}));r=room(await a.rpc({roomJoin:{}},r));r=room(await b.rpc({roomJoin:{}},r));return {host,guest,a,b,r};
}
async function ready(a:G2Consumer,b:G2Consumer,r:Room){r=room(await a.rpc({ready:{ready:true}},r));return room(await b.rpc({ready:{ready:true}},r));}
async function loaded(f:Awaited<ReturnType<typeof setup>>,a:G2Consumer,b:G2Consumer,r:Room){
 const games=[];
 for(const [desktop,id] of [[a,'a-game'],[b,'b-game']] as const){const response=await desktop.rpc({gameAttach:{gameInstanceId:id,gameVersion:g2Compatibility.gameVersion,modVersion:g2Compatibility.modVersion,bridgeVersion:g2Compatibility.bridgeVersion,capabilities:g2Compatibility.capabilities}},r);r=room(response);const game=f.consumer();game.credentials=response.credentials;games.push({game,fact:{attachmentId:game.credentials!.attachmentId!,gameInstanceId:id}});}
 for(const {game,fact} of games)r=room(await game.rpc({gameSeedApplied:fact},r));
 r=room(await games[0]!.game.rpc({gameLoaded:games[0]!.fact},r));assert.equal(r.state,'loading');
 r=room(await games[1]!.game.rpc({gameLoaded:games[1]!.fact},r));assert.equal(r.state,'running');return {r,games};
}
test('G2 browser assets/CSP, production host client creation/configuration/pairing and explicit revision',async()=>{
 const f=await setup();try{
  const page=await f.service.app.inject('/');assert.equal(page.statusCode,200);assert.match(page.headers['content-security-policy'] as string,/frame-ancestors 'none'/);assert.match(page.body,/Development · mock sign-in/);assert.equal((await f.service.app.inject('/client.js')).statusCode,200);assert.equal((await f.service.app.inject('/secrets/g2-dev-4351.json')).statusCode,404);
  const transport=async(path:string,body?:unknown,token?:string)=>{const response=await f.service.app.inject({method:body?'POST':'GET',url:path,...(body?{payload:body}:{}),headers:{...(token?{authorization:'Bearer '+token}:{})}});assert.equal(response.statusCode,200,response.body);return response.json();};
  const web=new HostClient(transport);const info=await web.initialize();assert.equal(info.profile.evidenceStatus,'simulated-game-facts');await web.login('web-host');
  let r=room(await web.rpc({roomCreate:{name:'<script>untrusted</script>',config,visibility:'private'}}));r=room(await web.rpc({configure:{...config,seed:9001}},r));assert.equal(r.config!.seed,9001);
  const d=f.consumer(),pairing=(await d.rpc({pairBegin:{deviceId:'desktop'}})).pairing!;await web.rpc({pairApprove:{attemptId:pairing.attemptId!,code:pairing.code}});const exchange=await d.rpc({pairExchange:{attemptId:pairing.attemptId!,proof:pairing.proof!,deviceId:'desktop'}});assert.equal(exchange.credentials!.accountId!,web.credentials.accountId);
  const wrong=await f.service.app.inject({method:'POST',url:'/v1/rpc',payload:{major:1,revision:'g1-draft-2',serviceId:info.serviceId,requestId:'wrong',pairBegin:{deviceId:'x'}}});assert.equal(wrong.json().error.code,'VERSION_MISMATCH');
  assert.deepEqual((await f.service.db.pool.query('SELECT version FROM schema_migrations ORDER BY version')).rows.map(r=>r.version),[1,2,3,4,5,6]);
 }finally{await f.close();}
});
test('G2 independent admission: salted password, capacity, ban precedence, no private leakage or stale mutation',async()=>{
 const f=await setup();try{
  const host=await f.consumer().browser('host','tab'),guest=await f.consumer().browser('guest','tab'),a=await f.consumer().desktop(guest,'a');let r=room(await host.rpc({roomCreate:{name:'Password',config}}));const stale=r;
  r=room(await host.rpc({roomPolicy:{capacity:1,password:'a strong local password'}},r));assert.equal((await host.rpc({roomPolicy:{capacity:2}},stale)).error?.code,'STALE_REVISION');
  const preview=(await a.rpc({roomPreview:{}},r)).admission!;assert.equal(preview.passwordRequired,true);assert.equal((preview as Record<string,unknown>).config,undefined);
  assert.equal((await a.rpc({roomJoin:{}},r)).error?.code,'PASSWORD_REQUIRED');assert.equal((await a.rpc({roomJoin:{password:'wrong'}},r)).error?.code,'PASSWORD_REQUIRED');r=room(await a.rpc({roomJoin:{password:'a strong local password'}},r));
  const verifier=(await f.service.db.pool.query('SELECT password_verifier FROM rooms WHERE id=$1',[r.roomId])).rows[0].password_verifier;assert.ok(!verifier.includes('a strong local password'));assert.equal((await a.rpc({roomGet:{}},r)).room!.passwordRequired,true);
  const another=await f.consumer().desktop(host,'another');assert.equal((await another.rpc({roomJoin:{password:'a strong local password'}},r)).error?.code,'CAPACITY');
  assert.equal((await a.rpc({roomModerate:{accountId:host.credentials!.accountId!,action:'ban'}},r)).error?.code,'FORBIDDEN');r=room(await host.rpc({roomModerate:{accountId:guest.credentials!.accountId!,action:'ban'}},r));
  assert.equal((await a.rpc({roomGet:{}},r)).error?.code,'NOT_FOUND');assert.equal((await a.rpc({roomJoin:{password:'a strong local password'}},r)).error?.code,'NOT_FOUND');assert.equal((await a.rpc({roomList:{}})).rooms!.length,0);
  r=room(await host.rpc({roomModerate:{accountId:guest.credentials!.accountId!,action:'unban'}},r));r=room(await a.rpc({roomJoin:{password:'a strong local password'}},r));assert.equal(r.members!.length,1);
 }finally{await f.close();}
});
test('G2 chat trusted sender, exact retry, independent-room denial, finite rate/retention and reset',async()=>{
 const f=await setup();try{
  const {host,guest,a,b,r}=await joined(f);const privateRoom=room(await host.rpc({roomCreate:{name:'Private',visibility:'private',config}}));
  const request=a.request({chatSend:{text:'<img src=x onerror=alert(1)>'}},r);const first=await a.send(request,true);assert.equal(first.error,undefined);assert.equal(first.chatMessages![0]!.accountId,a.credentials!.accountId);assert.equal(first.chatMessages![0]!.sessionKind,'desktop');assert.deepEqual(await a.send(request,true),first);
  assert.equal((await b.rpc({chatList:{afterSequence:0,limit:32}},r)).chatMessages![0]!.text,'<img src=x onerror=alert(1)>');assert.equal((await guest.rpc({chatList:{}},privateRoom)).error?.code,'FORBIDDEN');assert.equal((await b.rpc({chatList:{}},privateRoom)).error?.code,'FORBIDDEN');
  for(let i=0;i<4;i++)assert.equal((await a.rpc({chatSend:{text:'message'+i}},r)).error,undefined);const overflow=await a.rpc({chatSend:{text:'overflow'}},r);assert.equal(overflow.error?.code,'CAPACITY');assert.ok(overflow.error!.retryAfterMs!>0);assert.equal((await a.rpc({chatList:{limit:33}},r)).error?.code,'INVALID_MESSAGE');
  await f.service.db.transaction(async tx=>{await tx.query('INSERT INTO room_chat(room_id,sequence,account_id,session_id,session_kind,sent_at,text) SELECT $1,n,$2,$3,\'browser\',$4,\'retention fixture\' FROM generate_series(6,270) n',[r.roomId,host.credentials!.accountId,host.credentials!.sessionId,new Date(f.service.rooms.now)]);await tx.query('UPDATE rooms SET chat_sequence=270 WHERE id=$1',[r.roomId]);});
  const bounded=await host.rpc({chatList:{afterSequence:1,limit:32}},r);assert.equal(bounded.chatReset,true);assert.equal(bounded.chatMessages!.length,32);assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM room_chat')).rows[0].count,256);
  assert.equal((await a.rpc({chatSend:{text:'rate survives history pruning'}},r)).error?.code,'CAPACITY');
  f.advance(15*60*1000+1);host.credentials=(await host.rpc({refresh:{secret:host.credentials!.refreshSecret!,deviceId:'browser'}})).credentials!;const reset=await host.rpc({chatList:{afterSequence:1}},r);assert.equal(reset.chatReset,true);assert.deepEqual(reset.chatMessages,[]);assert.equal((await a.send(request,true)).error?.code,'UNAUTHENTICATED');a.credentials=(await a.rpc({refresh:{secret:a.credentials!.refreshSecret!,deviceId:'a'}})).credentials!;assert.equal((await a.send(request,true)).error?.code,'EXPIRED');await f.service.rooms.sweep();assert.deepEqual((await f.service.db.pool.query('SELECT result FROM command_results WHERE scope=$1 AND request_id=$2',[a.credentials!.sessionId,request.requestId])).rows[0].result,{expired:true});assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM room_chat')).rows[0].count,0);
 }finally{await f.close();}
});
test('G2 website and host desktop Start share frozen authority and two-game release; moderation keeps frozen roster',async()=>{
 const f=await setup();try{
  const {host,a,b,r:initial}=await joined(f);let r=await ready(a,b,initial);const roster={participantDesktopIds:[a.credentials!.sessionId!,b.credentials!.sessionId!],excludedDesktopIds:[]};
  r=room(await host.rpc({start:roster},r));assert.equal((await host.rpc({gameLoaded:{attachmentId:randomUUID(),gameInstanceId:'fake'}},r)).error?.code,'FORBIDDEN');
  const first=await loaded(f,a,b,r);r=first.r;const frozen=(await f.service.db.pool.query('SELECT roster,config FROM runs WHERE id=$1',[r.runId])).rows[0];assert.equal(frozen.config.seed,424242);
  r=room(await host.rpc({roomModerate:{accountId:b.credentials!.accountId!,action:'kick'}},r));assert.equal(r.state,'suspended');assert.deepEqual((await f.service.db.pool.query('SELECT roster FROM runs WHERE id=$1',[r.runId])).rows[0].roster,frozen.roster);assert.equal((await first.games[1]!.game.rpc({roomGet:{}},r)).error?.code,'UNAUTHENTICATED');
  r=room(await host.rpc({endRun:{}},r));r=room(await b.rpc({roomJoin:{}},r));r=await ready(a,b,r);r=room(await a.rpc({start:roster},r));assert.equal(r.state,'preparing');const second=await loaded(f,a,b,r);assert.notEqual(second.r.runId,first.r.runId);
 }finally{await f.close();}
});
test('G2 restart retains rooms/config/access/chat; selected preparation ban cancels whole run',async()=>{
 const f=await setup();try{
  const {host,a,b,r:initial}=await joined(f);let r=await ready(a,b,initial);r=room(await host.rpc({start:{participantDesktopIds:[a.credentials!.sessionId!,b.credentials!.sessionId!],excludedDesktopIds:[]}},r));const runId=r.runId;
  r=room(await host.rpc({roomModerate:{accountId:b.credentials!.accountId!,action:'ban'}},r));assert.equal(r.state,'waiting');assert.equal((await f.service.db.pool.query('SELECT state FROM runs WHERE id=$1',[runId])).rows[0].state,'cancelled');await host.rpc({chatSend:{text:'preserved'}},r);await f.restart();r=room(await host.rpc({roomGet:{}},r));assert.equal(r.config!.seed,424242);assert.equal((await b.rpc({roomPreview:{}},r)).error?.code,'NOT_FOUND');assert.equal((await host.rpc({chatList:{}},r)).chatMessages![0]!.text,'preserved');
 }finally{await f.close();}
});

test('G2 browser refresh recovers two committed reply losses across later rpc without revoking the family',async()=>{
 for(const automatic of [false,true]){
  const f=await setup();try{
   const actual=hostTransport(f);let drops=2;const attempts:{id:string;fingerprint:string}[]=[];
   const web=new HostClient(async(path:string,body:any,token?:string)=>{const result=await actual(path,body,token);if(body?.refresh){attempts.push({id:body.requestId,fingerprint:createHash('sha256').update(JSON.stringify(body)).digest('hex')});if(drops-->0)throw Error('committed refresh response dropped');}return result;},randomUUID,()=>f.service.rooms.now);
   await web.initialize();await web.login('refresh-host');const identity={accountId:web.credentials.accountId,sessionId:web.credentials.sessionId,sessionEpoch:web.credentials.sessionEpoch,deviceId:web.credentials.deviceId};
   if(automatic)f.advance(14*60*1000+30000);
   await assert.rejects(automatic?web.rpc({roomList:{}}):web.renew());assert.equal(attempts.length,2);const deadline=web.pendingRefresh.deadline;
   f.advance(5000);await web.rpc({roomList:{}});assert.equal(attempts.length,3);assert.equal(new Set(attempts.map(a=>a.id)).size,1);assert.equal(new Set(attempts.map(a=>a.fingerprint)).size,1);assert.equal(deadline,f.service.rooms.now-5000+30000);assert.equal(web.pendingRefresh,null);
   for(const [key,value] of Object.entries(identity))assert.equal(web.credentials[key],value);
   assert.equal((await f.service.db.pool.query('SELECT f.revoked_at FROM session_families f JOIN sessions s ON s.family_id=f.id WHERE s.id=$1',[identity.sessionId])).rows[0].revoked_at,null);
   assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM access_tokens WHERE session_id=$1 AND revoked_at IS NULL',[identity.sessionId])).rows[0].count,1);
  }finally{await f.close();}
 }
});

test('G2 expired pending browser refresh requires sign-in without replaying a spent secret',async()=>{
 const f=await setup();try{
  const actual=hostTransport(f);let attempts=0;
  const web=new HostClient(async(path:string,body:any,token?:string)=>{const result=await actual(path,body,token);if(body?.refresh){attempts++;throw Error('committed refresh response dropped');}return result;},randomUUID,()=>f.service.rooms.now);
  await web.initialize();await web.login('expired-refresh-host');const sessionId=web.credentials.sessionId;await assert.rejects(web.renew());f.advance(30000);await assert.rejects(web.rpc({roomList:{}}),(e:any)=>e.code==='EXPIRED');assert.equal(attempts,2);assert.equal(web.credentials,null);assert.equal(web.pendingRefresh,null);
  await assert.rejects(web.rpc({roomList:{}}),(e:any)=>e.code==='UNAUTHENTICATED');assert.equal(attempts,2);assert.equal((await f.service.db.pool.query('SELECT f.revoked_at FROM session_families f JOIN sessions s ON s.family_id=f.id WHERE s.id=$1',[sessionId])).rows[0].revoked_at,null);
  await web.login('expired-refresh-host');await web.rpc({roomList:{}});assert.notEqual(web.credentials.sessionId,sessionId);
 }finally{await f.close();}
});

test('G2 production controller cannot carry invitations across room, denied selection or account boundaries',async()=>{
 const f=await setup();try{
  const invited=await f.consumer().browser('invited','tab'),observer=await f.consumer().browser('observer','tab');const web=new HostClient(hostTransport(f),randomUUID,()=>f.service.rooms.now),dom=await controllerDom();
  const controller=createHostController({...dom,client:web});await controller.ready;await dom.submit('login',{subject:'controller-host'});
  await dom.submit('create',{name:'Private A',seed:'1',mode:'core',visibility:'private'});const a=dom.get('invite').value;await dom.submit('access',{visibility:'private',accounts:invited.credentials!.accountId!});
  assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM room_admissions WHERE room_id=$1',[a])).rows[0].count,1);
  await dom.submit('create',{name:'Private B',seed:'2',mode:'core',visibility:'private'});const b=dom.get('invite').value;assert.notEqual(a,b);assert.equal(dom.get('access').elements.accounts.value,'');await dom.submit('access',{visibility:'private'});
  assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM room_admissions WHERE room_id=$1',[b])).rows[0].count,0);assert.equal((await invited.rpc({roomId:b,roomPreview:{}})).error?.code,'NOT_FOUND');
  dom.get('access').elements.accounts.value=invited.credentials!.accountId!;await dom.submit('preview',{roomId:a});assert.equal(dom.get('access').elements.accounts.value,'');
  const outsider=room(await observer.rpc({roomCreate:{name:'Other account room',visibility:'public',config}}));dom.get('access').elements.accounts.value=invited.credentials!.accountId!;await dom.submit('preview',{roomId:outsider.roomId});assert.equal(dom.get('room-panel').hidden,true);assert.equal(dom.get('access').elements.accounts.value,'');
  await dom.submit('preview',{roomId:a});dom.get('access').elements.accounts.value=invited.credentials!.accountId!;await dom.click('logout');assert.equal(dom.get('access').elements.accounts.value,'');assert.equal(dom.get('room-panel').hidden,true);
  await dom.submit('login',{subject:'new-controller-host'});await dom.submit('create',{name:'Private C',seed:'3',mode:'core',visibility:'private'});const c=dom.get('invite').value;assert.equal(dom.get('access').elements.accounts.value,'');await dom.submit('access',{visibility:'private'});
  assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM room_admissions WHERE room_id=$1',[c])).rows[0].count,0);assert.equal((await invited.rpc({roomId:c,roomPreview:{}})).error?.code,'NOT_FOUND');
  // A draft bound before a known service-context change cannot submit afterward.
  dom.get('access').elements.accounts.value=invited.credentials!.accountId!;web.info.serviceEpoch='changed-context';await dom.submit('access',{visibility:'private'});assert.equal(dom.get('room-panel').hidden,true);assert.equal(+(await f.service.db.pool.query('SELECT count(*) FROM room_admissions WHERE room_id=$1',[c])).rows[0].count,0);
  controller.dispose();
 }finally{await f.close();}
});
