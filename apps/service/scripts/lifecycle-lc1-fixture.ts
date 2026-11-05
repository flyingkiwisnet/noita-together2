import {RACE_CAPABILITY} from '../../../packages/domain/src/g3-race-config.js';
import {LifecycleRaceControls} from './lc1-race-controls.js';
import {LifecycleD1Controls} from './lc1-d1-controls.js';
import {schema as c2Schema,validate as validateC2} from '../../../packages/contracts/src/g3-c2.js';
import {C1RuleDiagnostic,runC1RuleControl} from './lc1-c1-rule-control.js';
import {LifecycleC1Controls,C1_FIXTURE_LIMITS} from './lc1-c1-controls.js';
import {schema as c1Schema,validate as validateC1} from '../../../packages/contracts/src/g3-c1.js';
import type {Rules as C1Rules} from '../../../packages/contracts/g3-c1/g3-c1.js';
import {LifecycleGoldControls} from './lc1-gold-controls.js';
import {LifecycleTransport,type TransportTarget} from './lifecycle-lc1-transport.js';
import {StartDiagnostics,diagnosticCode} from './lifecycle-lc1-start-diagnostics.js';
import {traceLiveCapacity} from './lc1-live-capacity-diagnostic.js';
import {traceLiveAuthentication} from './lc1-live-auth-diagnostic.js';
import {LifecycleFixtureClock} from './lifecycle-lc1-clock.js';
import {fixtureHostAction,fixtureC1ReadRefreshCapacity,fixtureChatCapacity,fixtureHostReadCapacity,FixtureRpcRejection,type HostAction} from './lc1-host-action.js';
import {LifecycleSocialControls} from './lifecycle-lc1-social-controls.js';
import {resolve,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {randomUUID,randomBytes} from 'node:crypto';
import {startG3Fixture} from './g3-loot01-fixture.js';
import {hash,fingerprint,PolicyError} from '../src/g1/security.js';
import {schema as lobby,validate as validateLobby} from '../../../packages/contracts/src/g2-next.js';
import {schema,validate} from '../../../packages/contracts/src/g3-lifecycle.js';
import type {Credentials,GameObservation,Config} from '../../../packages/contracts/g2-next/g2-next.js';
import type {Request,WorldAssociation} from '../../../packages/contracts/g3-lifecycle/g3-lifecycle.js';
import type {ArtifactPin} from '../src/rejoin/composition-profile.js';
type Service=Awaited<ReturnType<typeof startG3Fixture>>['service'];
export type FaultPlan={worldId:string;attempt:number;phase:'before'|'rollback'|'reply';effect:'drop'|'delay'|'refuse';delayMs?:number};
export type StartRoster={selectedWorldIds:string[];excludedDesktopIds:string[]};
export class LifecycleFaults{
 private active:{id:string;plan:FaultPlan;release?:()=>void}|undefined;private count=0;private events:any[]=[];
 readonly raceControl=new LifecycleRaceControls();readonly d1Control=new LifecycleD1Controls();readonly c1Control=new LifecycleC1Controls();readonly c2Control=new LifecycleC1Controls('c2');readonly goldControl=new LifecycleGoldControls();readonly transportControl=new LifecycleTransport();readonly startDiagnostics=new StartDiagnostics();readonly socialControl=new LifecycleSocialControls();private optionalDelay=0;
 capacityDiagnostics:ReturnType<typeof traceLiveCapacity>|undefined;
 authDiagnostics:ReturnType<typeof traceLiveAuthentication>|undefined;
 transport(kind:'http'|'wss',enabled:boolean,target?:TransportTarget){return this.transportControl.transport(kind,enabled,target);}
 optionalStall(ms:number){if(!Number.isInteger(ms)||ms<0||ms>10000)throw Error('Bounded optional stall');this.optionalDelay=ms;}
 private event(kind:string){if(this.active)this.events.push({kind,id:this.active.id,worldId:this.active.plan.worldId,attempt:this.active.plan.attempt});if(this.events.length>128)this.events.shift();}
 arm(plan:FaultPlan){if(this.active||this.count++>=32||!/^[-a-f0-9]{36}$/i.test(plan.worldId)||!Number.isInteger(plan.attempt)||plan.attempt<1||!['before','rollback','reply'].includes(plan.phase)||!['drop','delay','refuse'].includes(plan.effect)||plan.phase==='rollback'&&plan.effect!=='refuse'||plan.phase==='reply'&&plan.effect==='refuse'||(plan.delayMs??10000)<1||(plan.delayMs??10000)>10000)throw Error('Invalid/busy bounded fixture fault');this.active={id:randomUUID(),plan:structuredClone(plan)};this.event('armed');return this.active.id;}
 snapshot(){return structuredClone(this.events);}
 release(){this.event('released');this.active?.release?.();this.active=undefined;this.raceControl.release();this.d1Control.release();this.socialControl.release();this.goldControl.release();this.c1Control.release();this.c2Control.release();}
 private match(req:any,phase:string){const p=req.body?.reconcile;return this.active?.plan.phase===phase&&req.url==='/v3/lifecycle/rpc'&&p?.association?.worldId===this.active.plan.worldId&&p?.attempt===this.active.plan.attempt;}
 private async effect(reply:any){const a=this.active!;this.event(a.plan.effect);if(a.plan.effect==='delay'){await new Promise<void>(resolve=>{const timer=setTimeout(resolve,a.plan.delayMs??10000);a.release=()=>{clearTimeout(timer);resolve();};});if(this.active===a)this.active=undefined;return false;}if(a.plan.effect==='drop'){reply.hijack();reply.raw.destroy();}else await reply.code(503).send({error:{code:'FIXTURE_REFUSED'}});this.active=undefined;return true;}
 install(service:Service){
  this.raceControl.install(service);this.d1Control.install(service);this.c1Control.install(service);this.c2Control.install(service);this.goldControl.install(service);this.capacityDiagnostics=traceLiveCapacity(service);
  this.startDiagnostics.install(service);
  this.transportControl.install(service);
  this.socialControl.install(service,(target,actor,roomId)=>this.transportControl.participantMatches(target,actor,roomId));
  this.authDiagnostics=traceLiveAuthentication(service,()=>this.transportControl.wssRefusalStatus().totalConsumed);
  service.app.addHook('preHandler',async(req,reply)=>{const body=req.body as any;if(this.optionalDelay&&(body?.chatList||body?.roomList||body?.inspectionList))await new Promise(r=>setTimeout(r,this.optionalDelay));if(this.match(req,'before'))await this.effect(reply);});
  service.app.addHook('onSend',async(req,reply,payload)=>{if(this.match(req,'reply')){let body:any;try{body=JSON.parse(String(payload));}catch{}if(body?.world&&body.world.attempt===this.active!.plan.attempt&&!body.error){this.event('committed-reply');if(await this.effect(reply))return null;}}return payload;});
  const previous=service.db.transactionProjection;
  service.db.transactionProjection={uncertain:()=>previous?.uncertain(),prepare:async tx=>{await previous?.prepare(tx);const a=this.active;if(a?.plan.phase==='rollback'&&(await tx.query('SELECT 1 FROM world_associations WHERE world_id=$1 AND high_water=$2',[a.plan.worldId,a.plan.attempt])).rowCount){this.event('rollback');this.active=undefined;throw new PolicyError('FIXTURE_ROLLBACK');}}};
 }
}
export async function startLifecycleLc1Fixture({race=false,raceConfiguration,persistentIdentity=false,testingStartingLoadout=false,testingMode,worldRules=false,runtime,itemTransfers=false,b1Gold=false,sendGold=true,sendWands=true,sendFlasks=true,sendItems=true,includeGoldCapability=true,diagnosticCapture,configure}:{raceConfiguration?:import('../../../packages/contracts/g2-next/g2-next.js').RaceConfig;race?:boolean;persistentIdentity?:boolean;testingStartingLoadout?:boolean;testingMode?:boolean;worldRules?:boolean;itemTransfers?:boolean;sendWands?:boolean;sendFlasks?:boolean;sendItems?:boolean;b1Gold?:boolean;sendGold?:boolean;includeGoldCapability?:boolean;runtime?:ArtifactPin;diagnosticCapture?:{root:string;enabled:boolean};configure?:(service:Service)=>void}={}){
 if(itemTransfers)b1Gold=true;
 const testRoot=process.env.NT2_SERVER_DIAGNOSTIC_TEST_ROOT;if(!diagnosticCapture&&testRoot){const root=resolve(testRoot),allowed=resolve(fileURLToPath(new URL('../../../spikes/noita-entry/.build/',import.meta.url)));if(!root.startsWith(allowed+sep))throw Error('Diagnostic test root must stay inside owned .build');diagnosticCapture={root,enabled:true};}
 const clock=new LifecycleFixtureClock(),faults=new LifecycleFaults();
 const f=await startG3Fixture({persistentIdentity,testingStartingLoadout,worldRules,itemTransfers,b1Gold,...(diagnosticCapture?{debugDiagnostics:diagnosticCapture.enabled,debugDiagnosticOptions:{root:diagnosticCapture.root,settingsPath:resolve(diagnosticCapture.root,'debug-settings.json')}}:{}),...(runtime?{fixtureRuntime:runtime}:{}),rejoin:true,loot01Runtime:true,live:true,tls:true,databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',now:clock.now,configure:s=>{faults.install(s);configure?.(s);}});
 if(f.profile.c1Fixture!==undefined)faults.c1Control.activate(f.profile);if(f.profile.c2Fixture!==undefined)faults.c2Control.activate(f.profile);
 let c1RuleCommands=0;
 const requireC1=()=>{if(!faults.c1Control.enabled)throw Error('FIXTURE_C1_DISABLED');};
 let room:any;const runIds=new Set<string>(),ownedRooms=new Map<string,any>();
 const diagnostics=faults.startDiagnostics;
 let startOriginal:{runId:string;requestId:string;fingerprint:string}|undefined;
 const chatEvents:any[]=[];const chatEvent=(event:any)=>{chatEvents.push(event);if(chatEvents.length>32)chatEvents.shift();};
 const hostEvents:any[]=[];const hostEvent=(event:any)=>{hostEvents.push(event);if(hostEvents.length>32)hostEvents.shift();};
 const rpc=async(c:Credentials|undefined,payload:any,retry?:{deadline:number;target:'host'|'peer';stage:'read'|'send'}|{deadline:number;operation:'end'}|{deadline:number;operation:'c1-refresh';busy:(attempt:number)=>void},timeoutDeadline?:number)=>{
  clock.rpcStep();const request={major:1,revision:lobby.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId:randomUUID(),...(c?{sessionEpoch:c.sessionEpoch}:{}),...(room?{roomId:room.roomId,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,expectedRevision:room.revision}:{}),...payload};
  diagnostics.request(request.requestId,!!payload.finalStart);if(payload.finalStart)startOriginal={runId:room.runId,requestId:request.requestId,fingerprint:fingerprint(payload.finalStart)};
  const send=async(bytes:string,token:string|undefined,timeoutMs=5000)=>{if(payload.endRun)hostEvent({operation:'end',stage:'submit',requestId:request.requestId,outcome:'issued'});const response=await f.call('/v1/rpc',bytes,token,{},timeoutMs),value=JSON.parse(response.body);if(retry&&'operation'in retry&&retry.operation==='c1-refresh'){validateLobby('Response',value);if(!value.error&&(value.serviceId!==f.service.db.serviceId||value.serviceEpoch!==f.service.db.epoch||value.requestId!==request.requestId||value.major!==1||value.revision!==lobby.revision))throw Error('FIXTURE_C1_RESPONSE');}diagnostics.response(response.status,value.error?.code,response.headers['x-lc1-fixture-transport-cut']==='1');if(payload.endRun)hostEvent({operation:'end',stage:'submit',requestId:request.requestId,...(value.error?{code:value.error.code}:{outcome:'accepted'})});if(value.error)throw new FixtureRpcRejection(value.error.code,response.status,value.serviceId===f.service.db.serviceId&&value.serviceEpoch===f.service.db.epoch&&value.requestId===request.requestId&&value.major===1&&value.revision===lobby.revision);return value;};
  const bytes=JSON.stringify(request),timeout=timeoutDeadline===undefined?5000:Math.min(5000,Math.floor(timeoutDeadline-performance.now()));if(timeout<=0)throw Error('FIXTURE_HOST_ACTION_DEADLINE');
  const value=retry?'operation'in retry?retry.operation==='c1-refresh'?await fixtureC1ReadRefreshCapacity('refresh',bytes,c?.accessToken??'',retry.deadline,send,retry.busy):await fixtureHostReadCapacity(bytes,c!.accessToken!,retry.deadline,send,attempt=>hostEvent({operation:'end',stage:'read',requestId:request.requestId,code:'CAPACITY',attempt})):await fixtureChatCapacity(bytes,c!.accessToken!,retry.deadline,send,attempt=>chatEvent({target:retry.target,stage:retry.stage,code:'CAPACITY',attempt,requestId:request.requestId})):await send(bytes,c?.accessToken,timeout);
  if(value.room){ownedRooms.set(value.room.roomId,structuredClone(value.room));runIds.add(value.room.runId);if(!room||payload.roomCreate||value.room.roomId===room.roomId)room=value.room;}return value;
 };
 const devices=[randomUUID(),randomUUID()];let desktop:Credentials;
 const signInAs=async(provider:'website'|'twitch',subject:'alex'|'sam',deviceId:string)=>{
  const appInstanceId=randomUUID(),initiatorProof=randomBytes(32).toString('base64url'),callbackState=randomBytes(32).toString('base64url');
  const begun=await rpc(undefined,{authBegin:{provider,deviceId,appInstanceId,proofChallenge:hash(initiatorProof),callbackPort:49152,callbackState}}),path=new URL(begun.authAttempt.launchUrl).pathname,page=await f.call(path),cookie=String(page.headers['set-cookie']).split(';')[0]!;
  const csrf=page.body.match(/name="csrf" value="([^"]+)"/)?.[1];if(!csrf)throw Error('Fixture csrf');const returned=await f.call(path,new URLSearchParams({csrf,subject}).toString(),undefined,{origin:f.origin,cookie,'content-type':'application/x-www-form-urlencoded'}),fields=Object.fromEntries([...returned.body.matchAll(/name="([^"]+)" value="([^"]+)"/g)].map(m=>[m[1],m[2]]));
  if(returned.status!==200||fields.callbackState!==callbackState||!fields.completionGrant)throw Error('Fixture signin');return (await rpc(undefined,{authExchange:{attemptId:begun.authAttempt.attemptId,deviceId,appInstanceId,initiatorProof,completionGrant:fields.completionGrant}})).credentials as Credentials;
 };
 const signIn=async(index=0)=>{if(index!==0&&index!==1)throw Error('FIXTURE_ACCOUNT');const current=await signInAs('website',index===0?'alex':'sam',devices[index]!);if(index===0)desktop=current;return current;};
 const chatActors=new Map<'host'|'peer',Credentials>();
 const prepareChatActor=async(target:'host'|'peer',principalId:string)=>{try{const actor=await signInAs(target==='host'?'twitch':'website',target==='host'?'alex':'sam','lc1-chat-'+randomUUID());if(actor.accountId!==principalId)throw Error('FIXTURE_CHAT_ACTOR');chatActors.set(target,actor);chatEvent({target,stage:'actor-bootstrap',outcome:'accepted'});}catch(error){chatEvent({target,stage:'actor-bootstrap',code:diagnosticCode(error)});throw error;}};
 try{
  // Distinct canonical provider account permits real host moderation of either
  // website consumer; the service correctly forbids self-moderation.
  let host=JSON.parse((await f.call('/v1/mock/browser-login',{subject:'twitch-local-alex',deviceId:'lc1-host-'+randomUUID(),requestId:randomUUID()})).body).credentials as Credentials;
  let hostApp:Credentials|undefined;
  const prepareHostApp=async()=>{if(!hostApp){const current=await signInAs('twitch','alex',randomUUID());if(current.accountId!==host.accountId||current.kind!=='desktop')throw Error('FIXTURE_HOST_APP_IDENTITY');hostApp=current;}return {prepared:true};};
  const ensureHost=async(retry?:{deadline:number;operation:'c1-refresh';busy:(attempt:number)=>void})=>{if(+new Date(host.expiresAt!)<=clock.now()+1000)host=(await rpc(undefined,{refresh:{secret:host.refreshSecret,deviceId:host.deviceId}},retry)).credentials;};
  await rpc(host,{roomCreate:{name:'LC1 isolated world lifecycle',config:{seed:424242,seedPolicy:'shared-fixed',randomizeLoot:true,mode:race?'race':'core',...(race?{race:raceConfiguration??{registryRevision:'race-goals-1',durationMs:0,goals:[{instanceId:'hp500',id:'reach_max_hp',target:500}]} }:{}),requiredCapabilities:[...new Set([...f.profile.compatibility.capabilities,...(race?[RACE_CAPABILITY]:[])])].filter((cap:string)=>!b1Gold||includeGoldCapability||cap!=='core.gold'),...((b1Gold||testingMode!==undefined)?{settings:[...(b1Gold?[{key:'NT_send_gold',value:String(race?false:sendGold)},...(itemTransfers?[{key:'NT_send_wands',value:String(race?false:sendWands)},{key:'NT_send_flasks',value:String(race?false:sendFlasks)},{key:'NT_send_items',value:String(race?false:sendItems)}]:[])]:[]),...(testingMode!==undefined?[{key:'NT_testing_mode',value:String(testingMode)}]:[])]}:{})}}});
  await prepareChatActor('host',host.accountId!);
  desktop=await signIn();faults.raceControl.bind('consumer',desktop.accountId!);faults.d1Control.bind('consumer',desktop.accountId!);const joined=await rpc(desktop,{roomJoin:{}}),preparation=joined.preparation;await faults.transportControl.bind('consumer',desktop,room.roomId);faults.goldControl.bind('consumer',desktop.accountId!,room.roomId);if(faults.c1Control.enabled)faults.c1Control.bind('consumer',desktop,room.roomId);if(faults.c2Control.enabled)faults.c2Control.bind('consumer',desktop,room.roomId);
  const prepareWorld=async(c=desktop,worldId:string=randomUUID(),proof=randomBytes(32).toString('base64url'))=>{const q={major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),prepare:{worldId,roomId:room.roomId,proof,configRevision:room.configRevision,configDigest:fingerprint(room.config)}};const response=await f.call('/v3/lifecycle/rpc',q,c.accessToken),value=JSON.parse(response.body);if(value.error)throw Error(value.error.code);validate('Response',value);return {association:value.prepared.association as WorldAssociation,proof,prepared:value.prepared};};
  let {association,proof}=await prepareWorld();
  let observation:GameObservation={worldToken:association.worldId!,gameInstanceId:'lc1-contact-1',observationRevision:1,actualSeed:String(room.config.seed),seedStatus:'known',activeMods:['noita_together2'],inventoryComplete:true,collectorFailure:'',beta:'unknown',protected:true,loaded:true,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,compatibilityProvenance:'game-thread-client-observation'};
  const request=(c=desktop,attempt=1,connectionId='lc1-connection-1',obs=observation):Request=>({major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch!,requestId:randomUUID(),reconcile:{association,proof,attempt,connectionId,action:'connect',gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:[...new Set([...f.profile.compatibility.capabilities,...(race?[RACE_CAPABILITY]:[])])],observation:{...obs}}});
  const send=async(c:Credentials,q:Request)=>{const response=await f.call('/v3/lifecycle/rpc',q,c.accessToken);const value=JSON.parse(response.body);if(response.status===200)validate('Response',value);return value;};
  runIds.add(room.runId);
  type Bootstrap={desktop:Credentials;deviceId:string;association:WorldAssociation;proof:string;observation:GameObservation};
  let peer:Bootstrap|undefined;
  const bootstrapPeer=async()=>{if(peer)return peer;const c=await signIn(1);faults.raceControl.bind('peer',c.accountId!);faults.d1Control.bind('peer',c.accountId!);await prepareChatActor('peer',c.accountId!);await faults.transportControl.bind('peer',c,room.roomId);faults.goldControl.bind('peer',c.accountId!,room.roomId);if(faults.c1Control.enabled)faults.c1Control.bind('peer',c,room.roomId);if(faults.c2Control.enabled)faults.c2Control.bind('peer',c,room.roomId);await rpc(c,{roomPreview:{}});await rpc(c,{roomJoin:{}});const p=await prepareWorld(c);peer={desktop:c,deviceId:devices[1]!,association:p.association,proof:p.proof,observation:{...observation,worldToken:p.association.worldId!,actualSeed:String(room.config.seed),gameInstanceId:'lc1-peer-contact-1'}};return peer;};
  const state=async(runId=room.runId)=>{if(!runIds.has(runId))throw Error('FIXTURE_RUN');return f.service.db.transaction(async tx=>({room:{roomId:room.roomId,runId:room.runId,runEpoch:room.runEpoch,state:room.state,configRevision:room.configRevision},runId,worlds:(await tx.query('SELECT world_id,world_id AS "worldId",account_id AS "accountId",desktop_id AS "desktopId",attachment_id AS "attachmentId",high_water,generation,owner_attempt,terminal_reason FROM world_associations WHERE run_id=$1',[runId])).rows,participants:(await tx.query('SELECT p.id AS "participantId",p.account_id AS "accountId",p.desktop_id AS "desktopId",world_token,world_token AS "worldId",generation,state,terminal,original_start_id FROM rejoin_participants p JOIN rejoin_runs r USING(run_id) WHERE p.run_id=$1',[runId])).rows}));};
  const startReview=(r:any)=>fingerprint({roomId:r.roomId,runId:r.runId,runEpoch:r.runEpoch,configRevision:r.configRevision,config:r.config,state:r.state,hostAccountId:r.hostAccountId,members:(r.members??[]).map((m:any)=>({desktopId:m.desktopId,accountId:m.accountId})).sort((a:any,b:any)=>a.desktopId.localeCompare(b.desktopId))});
  type StartReview={room:any;reports:any[];selectedWorldIds:string[];excludedDesktopIds:string[]};
  const start=async(roster?:StartRoster,descriptor?:()=>Promise<void>,review?:(value:StartReview)=>void)=>{const diagnostic=diagnostics.begin();startOriginal=undefined;try{await diagnostics.step('host-renewal',ensureHost);await diagnostics.step('roomGet',()=>rpc(host,{roomGet:{}}));const reviewedRoom=structuredClone(room),reports:any[]=[];let cursor='',pages=0;do{if(++pages>32)throw Error('FIXTURE_START_BOUNDS');const v=await diagnostics.step('inspectionList',()=>rpc(host,{inspectionList:{limit:1,afterDesktopId:cursor}}));reports.push(...v.inspections);cursor=v.inspectionNextDesktopId??'';}while(cursor);
   const selected=roster?.selectedWorldIds??reports.map(p=>p.worldToken),excluded=roster?.excludedDesktopIds??[];
   for(const ids of [selected,excluded])if(!Array.isArray(ids)||ids.length>32||new Set(ids).size!==ids.length||ids.some(id=>typeof id!=='string'||!/^[-a-f0-9]{36}$/i.test(id)))throw Error('FIXTURE_ROSTER');
   if(selected.some(id=>!reports.some(p=>p.worldToken===id))||excluded.some(id=>!reviewedRoom.members.some((m:any)=>m.desktopId===id)))throw Error('FIXTURE_ROSTER');
   // Ordinary observations can advance the room revision during paged reads.
   // Refresh only that fence, preserving the exact reviewed scope/roster/reports.
   // FinalStart still validates every selected observation and is submitted once.
   await diagnostics.step('roomRecheck',async()=>{await rpc(host,{roomGet:{}});if(startReview(room)!==startReview(reviewedRoom))throw Error('FIXTURE_START_INTENT_CHANGED');});
   if(review)await diagnostics.step('reviewPolicy',async()=>review(structuredClone({room,reports,selectedWorldIds:selected,excludedDesktopIds:excluded})));
   const value=await diagnostics.step('finalStart',()=>rpc(host,{finalStart:{participants:selected.map(id=>{const p=reports.find(p=>p.worldToken===id)!;return {desktopId:p.desktopId,attachmentId:p.attachmentId,worldToken:p.worldToken,observationRevision:p.observationRevision,contentDigest:p.contentDigest};}),excludedDesktopIds:excluded}}));
   if(descriptor)await diagnostics.step('descriptor',descriptor);
   const current=await diagnostics.step('state',()=>state());
   return descriptor?{...current,started:true}:value;
  }catch(e){if(!diagnostic.failedStage){diagnostic.failedStage='roster';diagnostic.category='control';diagnostic.code=diagnosticCode(e);}throw e;
  }finally{
   if(startOriginal){const original=startOriginal as {runId:string;requestId:string;fingerprint:string};try{
    await diagnostics.step('outcome',async()=>{const row=await f.service.db.transaction(async tx=>(await tx.query('SELECT authorization_id,start_fingerprint FROM inspected_releases WHERE run_id=$1',[original.runId])).rows[0],true,AbortSignal.timeout(2500));const final=diagnostic.stages.find(s=>s.stage==='finalStart');diagnostic.outcome=row?(row.authorization_id===original.requestId&&row.start_fingerprint===original.fingerprint?'committed':'different-start'):final?.serviceCompleted||final?.httpStatus!==undefined?'not-committed':'unknown';diagnostic.outcomeCheck='durable-request-and-fingerprint';});
   }catch{const confirmed=diagnostic.stages.find(s=>s.stage==='finalStart')?.transactions.some(t=>t.commit==='confirmed');diagnostic.outcome=confirmed?'committed':'unknown';diagnostic.outcomeCheck=confirmed?'request-correlated-service-commit':'unavailable';}}
   diagnostics.finish();
  }
  };
  // Explicit private host re-review. No uncertain/committed original is retried.
  const startReviewed=async(roster?:StartRoster,descriptor?:()=>Promise<void>)=>{
   let baseline:StartReview|undefined,previousOperationId:string|undefined;const limit=3;
   const metadata=new Set(['observationRevision','capturedAt','validUntil','presenceUntil']);
   for(let attempt=1;attempt<=limit;attempt++){
    try{return await start(baseline?{selectedWorldIds:baseline.selectedWorldIds,excludedDesktopIds:baseline.excludedDesktopIds}:roster,descriptor,next=>{
     const changes:{worldId:string;fields:string[]}[]=[];
     if(baseline){
      if(startReview(next.room)!==startReview(baseline.room)||fingerprint(next.selectedWorldIds)!==fingerprint(baseline.selectedWorldIds)||fingerprint(next.excludedDesktopIds)!==fingerprint(baseline.excludedDesktopIds))throw Error('FIXTURE_START_INTENT_CHANGED');
      for(const worldId of baseline.selectedWorldIds){const old=baseline.reports.find(p=>p.worldToken===worldId)!,current=next.reports.find(p=>p.worldToken===worldId);if(!current)throw Error('FIXTURE_START_INSPECTION_CHANGED');
       const fields=[...new Set([...Object.keys(old),...Object.keys(current)])].filter(k=>fingerprint(old[k]??null)!==fingerprint(current[k]??null)).sort();
       if(fields.length)changes.push({worldId,fields});
       diagnostics.review({attempt,limit,previousOperationId:previousOperationId!,inspectionChanges:changes});
       if(fields.some(k=>!metadata.has(k))||current.observationRevision<old.observationRevision)throw Error('FIXTURE_START_INSPECTION_CHANGED');
      }
     }else baseline=structuredClone(next);
     diagnostics.review({attempt,limit,...(previousOperationId?{previousOperationId}:{}),inspectionChanges:changes});
    });}catch(e){const d=diagnostics.snapshot().at(-1)!;
     if(!(e instanceof FixtureRpcRejection)||!['STALE_REVISION','REPORT_CHANGED'].includes(e.code)||e.status!==200||!e.sameService||d.failedStage!=='finalStart'||d.outcome!=='not-committed'||d.outcomeCheck!=='durable-request-and-fingerprint'||!d.startRequestId)throw e;
     if(attempt===limit)throw Error(`FIXTURE_START_REVIEW_EXHAUSTED: ${e.code} at ${d.failedStage}; ${d.outcome} (${d.outcomeCheck})`,{cause:e});previousOperationId=d.operationId;
    }
   }
   throw Error('FIXTURE_START_REVIEW_EXHAUSTED');
  };
  const hostAction=async(operation:HostAction,intent:(r:any)=>any)=>{
   if(operation!=='end'){await ensureHost();return fixtureHostAction(operation,async()=>(await rpc(host,{roomGet:{}})).room,(r,payload)=>rpc(host,{roomId:r.roomId,runId:r.runId,runEpoch:r.runEpoch,configRevision:r.configRevision,expectedRevision:r.revision,...payload}),intent);}
   const deadline=performance.now()+12000;let stage:'host-auth'|'read'|'submit'='host-auth';
   try{await ensureHost();const actor=structuredClone(host);hostEvent({operation,stage,outcome:'accepted'});
    return await fixtureHostAction(operation,async()=>{stage='read';const value=await rpc(actor,{roomGet:{}},{operation:'end',deadline});hostEvent({operation,stage,outcome:'accepted'});return value.room;},(r,payload)=>{stage='submit';return rpc(actor,{roomId:r.roomId,runId:r.runId,runEpoch:r.runEpoch,configRevision:r.configRevision,expectedRevision:r.revision,...payload},undefined,deadline);},intent);
   }catch(error){hostEvent({operation,stage,code:diagnosticCode(error)});throw error;}
  };
  const end=()=>hostAction('end',()=>({endRun:{}}));
  const createCoreLobby=async(config:Config)=>{
   if(!race||config?.mode!=='core'||config.race!==undefined)throw Error('FIXTURE_CORE_LOBBY');
   const previousRoomId=room.roomId;await ensureHost();
   const current=(await rpc(host,{roomId:previousRoomId,roomGet:{}})).room;
   if(current?.roomId!==previousRoomId||current.config.mode!=='race'||current.state!=='ended')throw Error('FIXTURE_CORE_LOBBY');
   await rpc(host,{roomCreate:{name:'LC1 separate Core lobby',config:structuredClone(config),allowGuests:false,visibility:'public'}});
   if(room.roomId===previousRoomId||room.config.mode!=='core'||!['waiting','staging'].includes(room.state))throw Error('FIXTURE_CORE_LOBBY');
   return {previousRoomId,roomId:room.roomId};
  };
  // Private composition controls use real authorized lobby/lifecycle routes.
  const createLobby=async()=>{if(!b1Gold||room.state!=='ended')throw Error('FIXTURE_END_REQUIRED');await ensureHost();try{await f.service.db.transaction(tx=>f.service.auth.authenticate(tx,desktop.accessToken!));}catch(e){if(!(e instanceof PolicyError)||e.code!=='UNAUTHENTICATED')throw e;await signIn();}const config=structuredClone(room.config);await rpc(host,{roomCreate:{name:'LC1 separate transfer lobby',config,allowGuests:false,visibility:'public'}});await rpc(desktop,{roomJoin:{}});await faults.transportControl.bind('consumer',desktop,room.roomId);faults.goldControl.bind('consumer',desktop.accountId!,room.roomId);if(faults.c1Control.enabled)faults.c1Control.bind('consumer',desktop,room.roomId);if(faults.c2Control.enabled)faults.c2Control.bind('consumer',desktop,room.roomId);({association,proof}=await prepareWorld());observation={...observation,worldToken:association.worldId!,actualSeed:String(room.config.seed),gameInstanceId:'lc1-contact-'+randomUUID(),observationRevision:1,protected:true};peer=undefined;return {roomId:room.roomId};};
  const deleteLobby=async(roomId=room.roomId)=>{if(!b1Gold||!ownedRooms.has(roomId))throw Error('FIXTURE_ROOM');await ensureHost();let target=ownedRooms.get(roomId);try{target=(await rpc(host,{roomId,roomGet:{}})).room;}catch(e){if(!(e instanceof FixtureRpcRejection)||e.code!=='NOT_FOUND')throw e;}return rpc(host,{roomId,runId:target.runId,runEpoch:target.runEpoch,configRevision:target.configRevision,expectedRevision:target.revision,roomDelete:{}});};
  let deleteProbe:Credentials|undefined,deleteProbes=0;
  const forbiddenLobbyDelete=async(roomId=room.roomId)=>{if(!b1Gold||!ownedRooms.has(roomId)||deleteProbes++>=16)throw Error('FIXTURE_ROOM');if(!deleteProbe||+new Date(deleteProbe.expiresAt!)<=clock.now()+1000)deleteProbe=JSON.parse((await f.call('/v1/mock/browser-login',{subject:'website-local-alex',deviceId:'lc1-delete-probe-'+randomUUID(),requestId:randomUUID()})).body).credentials;if(!deleteProbe||deleteProbe.accountId===host.accountId)throw Error('FIXTURE_DELETE_ACTOR');const target=ownedRooms.get(roomId);try{const v=await rpc(deleteProbe,{roomId,runId:target.runId,runEpoch:target.runEpoch,configRevision:target.configRevision,expectedRevision:target.revision,roomDelete:{}});return {accepted:true,deletedRoomId:v.deletedRoomId};}catch(e){if(!(e instanceof FixtureRpcRejection)||!e.sameService||!/^[A-Z_]{1,64}$/.test(e.code))throw e;return {accepted:false,rejection:e.code};}};
  // Read-only counters for composition assertions, never Client deletion proof.
  const lobbyTransferSummary=async(roomId=room.roomId)=>{if(!b1Gold||!ownedRooms.has(roomId))throw Error('FIXTURE_ROOM');return f.service.db.transaction(async tx=>{const result=(await tx.query(`SELECT r.deleted_at IS NOT NULL AS deleted,
   (SELECT count(*)::int FROM g3_runs WHERE room_id=r.id) AS runs,
   (SELECT count(*)::int FROM g3_operations WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS operations,
   (SELECT count(*)::int FROM g3_receipts WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS receipts,
   (SELECT count(*)::int FROM g3_ledger WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS "ledgerEntries",
   (SELECT coalesce(sum(delta),0)::text FROM g3_ledger WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS "ledgerDelta",
   (SELECT coalesce(sum(amount),0)::text FROM g3_balances WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS "balanceTotal",
   (SELECT count(*)::int FROM g3_outbox WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id) AND kind IN ('gold.intent','gold.receipt','gold.deposit')) AS "goldOutbox",
   (SELECT count(*)::int FROM g3_bank_items WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS "bankItems",
   (SELECT count(*)::int FROM g3_item_operations WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS "itemOperations",
   (SELECT count(*)::int FROM g3_item_receipts WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS "itemReceipts",
   (SELECT count(*)::int FROM g3_item_inventory WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS "itemInventories",
   (SELECT count(*)::int FROM g3_outbox WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id) AND kind IN ('item.intent','item.receipt','item.deposit')) AS "itemOutbox",
   (SELECT count(*)::int FROM g3_statistics WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS statistics,
   (SELECT coalesce(sum(count),0)::text FROM g3_statistics WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=r.id)) AS "statisticsTotal"
   FROM rooms r WHERE r.id=$1`,[roomId])).rows[0];if(!result)throw Error('FIXTURE_ROOM');return {roomId,...result};});};
  const moderate=async(target:'consumer'|'peer',action:'ban'|'kick'|'unban')=>{if(!['consumer','peer'].includes(target)||!['ban','kick','unban'].includes(action)||target==='peer'&&!peer)throw Error('FIXTURE_ACCOUNT');const accountId=(target==='consumer'?desktop:peer!.desktop).accountId;return hostAction('moderate',()=>({roomModerate:{accountId,action}}));};
  const nextRun=async(config?:Config)=>{const old=room.runId,frozen=config===undefined?undefined:structuredClone(config);const value=await hostAction('nextRun',r=>({configure:frozen??r.config}));if(room.runId===old||room.state!=='staging')throw Error('FIXTURE_NEXT_RUN');runIds.add(room.runId);return value;};
  // These actors never enter the descriptor, join/prepare or share refresh secrets
  // with ordinary Clients. Rotating a Client session cannot revoke chat's bearer.
  const chat=async(target:'host'|'peer',text:string)=>{if(!['host','peer'].includes(target)||target==='peer'&&!peer||typeof text!=='string'||!text.trim()||Buffer.byteLength(text)>1024)throw Error('FIXTURE_CHAT_ARGUMENT');
   const principalId=target==='host'?host.accountId:peer!.desktop.accountId,credentials=chatActors.get(target),deadline=performance.now()+12000;let stage:'actor-bootstrap'|'read'|'send'='actor-bootstrap';
   try{
    // Actors are established during host/peer setup. No signin/refresh is
    // hidden inside the measured chat-during-transfer operation.
    if(!credentials||credentials.accountId!==principalId)throw Error('FIXTURE_CHAT_ACTOR');
    if(+new Date(credentials.expiresAt!)<=clock.now()+1000)throw Error('FIXTURE_CHAT_ACTOR_EXPIRED');
    const value=await fixtureHostAction('chat',async()=>{stage='read';return(await rpc(credentials,{roomGet:{}},{target,stage,deadline})).room;},(r,payload)=>{stage='send';return rpc(credentials,{roomId:r.roomId,runId:r.runId,runEpoch:r.runEpoch,configRevision:r.configRevision,expectedRevision:r.revision,...payload},{target,stage,deadline});},()=>({chatSend:{text}}));
    const message=value.chatMessages?.at(-1);if(!message)throw Error('FIXTURE_CHAT_RESULT');chatEvent({target,stage:'send',outcome:'accepted'});return {chat:{roomId:room.roomId,sequence:message.sequence,accountId:message.accountId,principal:message.principal,sentAt:message.sentAt},chatDiagnostic:structuredClone(chatEvents)};
   }catch(error){chatEvent({target,stage,code:diagnosticCode(error)});throw error;}
  };
  const c1Diagnostic=new C1RuleDiagnostic();
  const c1Rules=async(input:{expectedRevision?:number;rules?:C1Rules}={})=>{
   const value=await runC1RuleControl(input,{before:()=>{requireC1();if(c1RuleCommands++>=C1_FIXTURE_LIMITS.ruleCommands)throw Error('FIXTURE_C1_RULE_BOUNDS');},ensureHost:(deadline,busy)=>ensureHost({deadline,operation:'c1-refresh',busy}),send:async(payload,deadline,busy)=>{
    clock.rpcStep();const credential=structuredClone(host),request=validateC1('Request',{major:1,revision:c1Schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:credential.sessionEpoch,requestId:randomUUID(),roomId:room.roomId,...payload}),bytes=JSON.stringify(request);
    const send=async(body:string,token:string,timeoutMs:number)=>{const response=await f.call('/v3/c1/rpc',body,token,{},timeoutMs),value=JSON.parse(response.body);validateC1('Response',value);const correlated=value.major===1&&value.revision===c1Schema.revision&&value.requestId===request.requestId&&value.serviceId===f.service.db.serviceId&&value.serviceEpoch===f.service.db.epoch;if(value.error)throw new FixtureRpcRejection(value.error.code,response.status,correlated);if(!correlated)throw Error('FIXTURE_C1_RESPONSE');return value;};
    if(payload.rulesGet)return fixtureC1ReadRefreshCapacity('read',bytes,credential.accessToken!,deadline,send,busy);
    const remaining=Math.floor(deadline-performance.now());if(remaining<=0)throw Error('FIXTURE_C1_DEADLINE');return send(bytes,credential.accessToken!,Math.min(5000,remaining));
   }},c1Diagnostic);return {...value,c1Diagnostic:c1Diagnostic.snapshot()};
  };
  let c2RuleCommands=0;let c2Diagnostic={stage:'idle',code:'NONE'};
  const requireC2=()=>{if(!faults.c2Control.enabled)throw Error('FIXTURE_C2_DISABLED');};
  const c2Rules=async(input:{expectedRevision?:number;rules?:{NT_sync_steve?:boolean}}={})=>{c2Diagnostic={stage:'arguments',code:'NONE'};requireC2();if(c2RuleCommands++>=32||!input||Object.keys(input).some(k=>!['expectedRevision','rules'].includes(k))||input.rules===undefined&&input.expectedRevision!==undefined)throw Error('FIXTURE_C2_RULES');if(input.rules!==undefined){validateC2('Rules',input.rules);if(!Number.isInteger(input.expectedRevision)||input.expectedRevision!<1)throw Error('FIXTURE_C2_RULES');}c2Diagnostic.stage='host-refresh';await ensureHost();const send=async(payload:any)=>{c2Diagnostic.stage=payload.rulesGet?'rules-read':'rules-write';const credential=structuredClone(host),request=validateC2('Request',{major:1,revision:c2Schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:credential.sessionEpoch,requestId:randomUUID(),roomId:room.roomId,...payload}),response=await f.call('/v3/c2/rpc',request,credential.accessToken),value=JSON.parse(response.body);validateC2('Response',value);if(value.major!==1||value.revision!==c2Schema.revision||value.requestId!==request.requestId||value.serviceId!==f.service.db.serviceId||value.serviceEpoch!==f.service.db.epoch)throw Error('FIXTURE_C2_RESPONSE');if(value.error){c2Diagnostic.code=value.error.code;throw Error('FIXTURE_C2_REJECTED');}if(!value.snapshot?.rules)throw Error('FIXTURE_C2_RESPONSE');return {snapshot:value.snapshot.rules,sequence:value.snapshot.sequence};};const current=await send({rulesGet:{}});return input.rules===undefined?current:send({rulesSet:{expectedRevision:input.expectedRevision,rules:{...current.snapshot.rules,...input.rules}}});};
  const c2Status=(eventId?:string)=>{requireC2();if(!runIds.has(room.runId))throw Error('FIXTURE_RUN');return faults.c2Control.audit(room.runId,room.roomId,eventId);};
  const c2DropPublishReply=(target:'consumer'|'peer',eventId?:string)=>{requireC2();return faults.c2Control.arm({target,...(eventId!==undefined?{eventId}:{})},room.runId,room.roomId);};
  const c2Network=(target:'consumer'|'peer',durationMs?:number)=>{requireC2();return faults.transportControl.networkProfile(target,'c1Fixed250',0,durationMs);};
  const c1Status=(eventId?:string)=>{requireC1();if(!runIds.has(room.runId))throw Error('FIXTURE_RUN');return faults.c1Control.audit(room.runId,room.roomId,eventId);};
  const c1DropPublishReply=(target:'consumer'|'peer',eventId?:string)=>{requireC1();return faults.c1Control.arm({target,...(eventId!==undefined?{eventId}:{})},room.runId,room.roomId);};
  const c1Network=(target:'consumer'|'peer',durationMs?:number)=>{requireC1();return faults.transportControl.networkProfile(target,'c1Fixed250',0,durationMs);};
  const seedSocialPois=(target:'consumer'|'peer',eventId:string,total:number)=>faults.socialControl.seed(target,eventId,total,room.roomId);
  return {fixture:f,c2Rules,c2RulesDiagnostics:()=>({...c2Diagnostic}),c2Status,c2DropPublishReply,c2Network,c1Rules,c1RulesDiagnostics:()=>c1Diagnostic.snapshot(),c1Status,c1DropPublishReply,c1Network,startDiagnostics:()=>diagnostics.snapshot(),get host(){return host;},get hostApp(){return hostApp;},prepareHostApp,get desktop(){return desktop;},devices,get association(){return association;},get proof(){return proof;},get observation(){return observation;},prepareWorld,request,send,rpc,signIn,start,startReviewed,end,createLobby,createCoreLobby,deleteLobby,forbiddenLobbyDelete,lobbyTransferSummary,moderate,nextRun,chat,chatDiagnostics:()=>structuredClone(chatEvents),hostActionDiagnostics:()=>structuredClone(hostEvents),seedSocialPois,bootstrapPeer,get peer(){return peer;},state,faults,denyWssOnce:(target:'consumer')=>faults.transportControl.denyWssOnce(target),wssRefusalStatus:()=>faults.transportControl.wssRefusalStatus(),clearWssRefusal:()=>faults.transportControl.clearWssRefusal(),transport:(kind:'http'|'wss',enabled:boolean,target?:TransportTarget)=>faults.transport(kind,enabled,target),clockMode:(mode:'manual'|'elapsed')=>clock.setMode(mode),clockStatus:()=>clock.status(),advance:(ms:number,kind?:'wall'|'injected')=>clock.advance(ms,kind),close:async()=>{faults.release();await f.close();},get room(){return room;}};
 }catch(e){faults.release();await f.close();throw e;}
}









