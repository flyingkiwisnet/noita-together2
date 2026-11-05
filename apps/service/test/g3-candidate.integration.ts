import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,randomBytes,generateKeyPairSync} from 'node:crypto';
import {readFile} from 'node:fs/promises';
import pg from 'pg';
import {createG3CandidateApp} from '../src/g3/candidate.js';
import {hash} from '../src/g1/security.js';
import {schema as lobbySchema} from '../../../packages/contracts/src/g2-next.js';
import {schema as gameplaySchema,encode,decode} from '../../../packages/contracts/src/g3.js';
import type {Credentials,Request,Response,Room,GameObservation,Inspection} from '../../../packages/contracts/g2-next/g2-next.js';
import type {Request as GameplayRequest,Response as GameplayResponse,Scope,Operation} from '../../../packages/contracts/g3/g3.js';
// @ts-expect-error Browser module consumed with an injected real HTTP transport.
import {HostClient} from '../../web/candidate/client.js';
// @ts-expect-error Browser module consumed with an injected real HTTP transport.
import {StatisticsClient} from '../../web/g3/client.js';
const url=process.env.NT2_G2_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';

async function fixture(){
 const profile=JSON.parse(await readFile(new URL('../../../deploy/profiles/g3-simulated-candidate.json',import.meta.url),'utf8'));
 const namespace='g3_candidate_'+randomUUID().replaceAll('-',''),pair=generateKeyPairSync('ed25519');let now=Date.now(),room:Room|undefined;
 const options={databaseUrl:url,namespace,serviceId:profile.serviceId,mockEnabled:true,website:true,now:()=>now,compatibility:profile.compatibility,
  keys:{privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}).toString(),publicKey:pair.publicKey.export({type:'spki',format:'pem'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:profile.issuerExample}};
 let service=await createG3CandidateApp(options);
 const envelope=(c:Credentials|undefined,payload:Partial<Request>):Request=>({major:1,revision:lobbySchema.revision,serviceId:service.db.serviceId,serviceEpoch:service.db.epoch,requestId:randomUUID(),...(c?{sessionEpoch:c.sessionEpoch!}:{}),...(room?{roomId:room.roomId!,runId:room.runId??'',runEpoch:room.runEpoch??'',configRevision:room.configRevision!,expectedRevision:room.revision!}:{}),...payload});
 async function rpc(c:Credentials|undefined,payload:Partial<Request>){now+=100;const response=await service.app.inject({method:'POST',url:'/v1/rpc',headers:c?{authorization:'Bearer '+c.accessToken}:{},payload:envelope(c,payload)});assert.equal(response.statusCode,200,response.body);const r=response.json() as Response;if(r.room)room=r.room;return r;}
 async function ok(c:Credentials|undefined,payload:Partial<Request>){const r=await rpc(c,payload);assert.equal(r.error,undefined,r.error?.code);return r;}
 async function signIn(subject:'alex'|'sam'){
  const deviceId='desktop-'+subject,appInstanceId=randomUUID(),initiatorProof=randomBytes(32).toString('base64url'),callbackState=randomBytes(32).toString('base64url');
  const begun=await ok(undefined,{authBegin:{provider:'website',deviceId,appInstanceId,proofChallenge:hash(initiatorProof),callbackPort:49152,callbackState}});
  const path=new URL(begun.authAttempt!.launchUrl!).pathname,page=await service.app.inject(path);
  const cookie=String(page.headers['set-cookie']).split(';')[0]!,csrf=page.body.match(/name="csrf" value="([^"]+)"/)![1]!;
  const returned=await service.app.inject({method:'POST',url:path,headers:{origin:options.keys.issuer,cookie,'content-type':'application/x-www-form-urlencoded'},payload:new URLSearchParams({csrf,subject}).toString()});
  assert.equal(returned.statusCode,200,returned.body);
  const fields=Object.fromEntries([...returned.body.matchAll(/name="([^"]+)" value="([^"]+)"/g)].map(m=>[m[1],m[2]]));assert.equal(fields.callbackState,callbackState);
  return (await ok(undefined,{authExchange:{attemptId:begun.authAttempt!.attemptId!,deviceId,appInstanceId,initiatorProof,completionGrant:fields.completionGrant!}})).credentials!;
 }
 const host=(await service.app.inject({method:'POST',url:'/v1/mock/browser-login',payload:{subject:'website-local-alex',deviceId:'g3-host-web',requestId:randomUUID()}})).json().credentials as Credentials;
 await ok(host,{roomCreate:{name:'G3 isolated integration',config:{seed:424242,mode:'core',requiredCapabilities:profile.compatibility.capabilities}}});
 const a=await signIn('alex'),b=await signIn('sam');
 for(const c of [a,b]){await ok(c,{roomJoin:{}});await ok(c,{ready:{ready:true}});}
 const attach=async(c:Credentials,instance:string)=>{const prep=(await ok(c,{prepare:{}})).preparation!;return (await ok(c,{gameAttach:{preparationId:prep.preparationId!,gameInstanceId:instance,gameVersion:profile.compatibility.gameVersion,modVersion:profile.compatibility.modVersion,bridgeVersion:profile.compatibility.bridgeVersion,capabilities:profile.compatibility.capabilities}})).credentials!;};
 const ga=await attach(a,'instance-a'),gb=await attach(b,'instance-b');
 // Only the fields in GameAttach are transmitted (metadata's modes/settings are not wire fields).
 const observation=(game:Credentials,instance:string,world:string,extra:Partial<GameObservation>={}):GameObservation=>({attachmentId:game.attachmentId!,gameInstanceId:instance,worldToken:world,observationRevision:1,actualSeed:'424242',seedStatus:'known',activeMods:['noita_together2'],inventoryComplete:true,collectorFailure:'',beta:'unknown',protected:true,loaded:true,modVersion:profile.compatibility.modVersion,bridgeVersion:profile.compatibility.bridgeVersion,compatibilityProvenance:'game-thread-client-observation',...extra});
 await ok(ga,{gameObservation:observation(ga,'instance-a','world-a')});await ok(gb,{gameObservation:observation(gb,'instance-b','world-b')});
 const reports:Inspection[]=[];let cursor='';do{const r=await ok(host,{inspectionList:{limit:1,afterDesktopId:cursor}});reports.push(...r.inspections!);cursor=r.inspectionNextDesktopId??'';}while(cursor);
 const scopes:Scope[]=[ga,gb].map((game,i)=>({roomId:room!.roomId!,runId:room!.runId!,runEpoch:room!.runEpoch!,configRevision:room!.configRevision!,attachmentId:game.attachmentId!,worldToken:i?'world-b':'world-a'}));
 async function game(c:Credentials,scope:Scope,body:Partial<GameplayRequest>,binary=false){now+=100;const request:GameplayRequest={major:1,revision:gameplaySchema.revision,serviceId:service.db.serviceId,serviceEpoch:service.db.epoch,sessionEpoch:c.sessionEpoch!,requestId:randomUUID(),scope,...body};const response=await service.app.inject({method:'POST',url:'/v3/rpc',headers:{authorization:'Bearer '+c.accessToken,'content-type':binary?'application/x-protobuf':'application/json'},payload:binary?Buffer.from(encode('Request',request)):request});assert.equal(response.statusCode,200,response.body);return (binary?decode('Response',response.rawPayload):response.json()) as GameplayResponse;}
 async function gameOk(c:Credentials,scope:Scope,body:Partial<GameplayRequest>,binary=false){const r=await game(c,scope,body,binary);assert.equal(r.error,undefined,r.error?.code);return r;}
 const start=()=>ok(host,{finalStart:{participants:reports.map(r=>({desktopId:r.desktopId!,attachmentId:r.attachmentId!,worldToken:r.worldToken!,observationRevision:r.observationRevision!,contentDigest:r.contentDigest!})),excludedDesktopIds:[]}});
 const playing=async()=>{await ok(ga,{gameObservation:observation(ga,'instance-a','world-a',{observationRevision:2,protected:false})});await ok(gb,{gameObservation:observation(gb,'instance-b','world-b',{observationRevision:2,protected:false})});};
 const receipt=(o:Operation,outcome:string)=>({goldReceipt:{operationId:o.operationId!,deliveryId:o.deliveryId!,payloadDigest:o.payloadDigest!,receiptId:randomUUID(),outcome,evidenceDigest:'a'.repeat(64)}});
 return {a,b,ga,gb,host,scopes,game,gameOk,ok,start,playing,receipt,observation,get service(){return service;},get room(){return room!;},async restart(){await service.app.close();service=await createG3CandidateApp(options);},async close(){await service.app.close();const pool=new pg.Pool({connectionString:url});try{await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);}finally{await pool.end();}}};
}

test('G3 candidate uses one sign-in/attachment/inspection/Start authority before authenticated gameplay and original recovery',async()=>{
 const f=await fixture();try{
  const [sa,sb]=f.scopes as [Scope,Scope],pose={movement:{frames:[{sequence:1,xMilli:'1',yMilli:'2',vxMilli:'0',vyMilli:'0',facingLeft:false}]}};
  assert.ok((await f.game(f.ga,sa,pose)).error);await f.start();assert.ok((await f.game(f.ga,sa,pose)).error);await f.playing();
  const info=(await f.service.app.inject('/v3/info')).json();assert.equal(info.serviceEpoch,(await f.service.app.inject('/v1/info')).json().serviceEpoch);
  await f.gameOk(f.ga,sa,pose,true);const ghosts=(await f.gameOk(f.gb,sb,{ghostList:{}},true)).ghosts!;assert.equal(ghosts[0]!.principalId,f.a.accountId);assert.equal(ghosts[0]!.gameSessionId,f.ga.sessionId);assert.equal(ghosts[0]!.worldToken,'world-a');
  await f.ok(f.ga,{gameObservation:f.observation(f.ga,'instance-a','world-a',{observationRevision:3,protected:true})});
  assert.ok((await f.game(f.ga,sa,{movement:{frames:[{...pose.movement.frames[0]!,sequence:2}]}})).error);
  await f.ok(f.ga,{gameObservation:f.observation(f.ga,'instance-a','world-a',{observationRevision:4,protected:false})});
  assert.equal((await f.game(f.ga,sa,pose)).error?.code,'INVALID_MESSAGE');
  await f.gameOk(f.ga,sa,{movement:{frames:[{...pose.movement.frames[0]!,sequence:2}]}});
  const original={goldIntent:{operationId:randomUUID(),direction:'deposit',amount:10}},deposit=(await f.gameOk(f.ga,sa,original)).operation!,proof=f.receipt(deposit,'escrowed');
  assert.equal((await f.gameOk(f.ga,sa,proof)).operation!.balance,10);
  await f.gameOk(f.gb,sb,{gameEvent:{eventId:randomUUID(),catalogId:'player.win',count:1}});
  const transport=async(path:string,body?:unknown,token?:string)=>{const response=await f.service.app.inject({method:body?'POST':'GET',url:path,...(body?{payload:body}:{}),headers:token?{authorization:'Bearer '+token}:{}});assert.equal(response.statusCode,200,response.body);return response.json();};
  const browser=new HostClient(transport),statistics=new StatisticsClient(browser,transport);await browser.initialize();await browser.login('website-local-alex');
  assert.equal((await statistics.request(sa,{statisticsList:{limit:1}})).statistics.length,1);
  assert.equal((await statistics.request(sa,{statisticsAggregate:{}})).aggregates.length,2);
  const page=await f.service.app.inject('/statistics');assert.equal(page.statusCode,200);assert.match(page.body,/Delete this run/);assert.match(String(page.headers['content-security-policy']),/frame-ancestors 'none'/);
  for(const path of ['/g3/page.js','/g3/client.js','/g3/statistics.js','/candidate/client.js'])assert.equal((await f.service.app.inject(path)).statusCode,200);
  const reserved=(await f.gameOk(f.ga,sa,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:5}})).operation!;
  await f.ok(f.host,{endRun:{}});
  assert.equal((await f.gameOk(f.a,sa,proof)).operation!.state,'completed');
  assert.equal((await f.game(f.b,sb,proof)).error?.code,'NOT_FOUND');
  assert.equal((await f.game(f.a,{...sa,worldToken:'foreign'},proof)).error?.code,'NOT_FOUND');
  assert.equal((await f.game(f.a,sa,{goldReceipt:{...proof.goldReceipt,evidenceDigest:'b'.repeat(64)}})).error?.code,'CONFLICT');
  assert.equal((await f.game(f.a,sa,f.receipt(deposit,'uncertain'))).error?.code,'INVALID_STATE');
  assert.equal((await f.game(f.a,sa,f.receipt(reserved,'applied'))).error?.code,'FORBIDDEN');
  const uncertain=f.receipt(reserved,'uncertain');assert.equal((await f.gameOk(f.a,sa,uncertain)).operation!.balance,5);
  assert.equal((await f.game(f.a,sa,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:1}})).error?.code,'FORBIDDEN');
  await f.restart();assert.equal((await f.gameOk(f.a,sa,original)).operation!.deliveryId,deposit.deliveryId);
  assert.equal((await f.gameOk(f.a,sa,proof)).operation!.balance,5);
  assert.equal((await f.gameOk(f.a,sa,uncertain)).operation!.state,'uncertain');
  assert.equal((await f.gameOk(f.a,sa,{goldStatus:{operationId:reserved.operationId!}})).operation!.state,'uncertain');
  assert.ok((await f.gameOk(f.a,sa,{outboxList:{limit:32}})).outbox!.length>=4);
  assert.equal((await f.game(f.a,sa,f.receipt(reserved,'applied'))).error?.code,'FORBIDDEN');
  await statistics.request(sa,{statisticsDelete:{}});assert.equal((await statistics.request(sa,{statisticsList:{}})).statistics?.length??0,0);
  assert.ok((await f.game(f.ga,sa,pose)).error);
 }finally{await f.close();}
});
