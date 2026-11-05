import {verifyLifecycleFixtureRuntime} from '../src/rejoin/lifecycle-fixture-profile.js';
import {verifyRejoinPackage} from '../src/rejoin/package-profile.js';
import {persistentRejoinIdentity} from '../src/rejoin/host-identity.js';
import {materializeRejoinComposition,type ArtifactPin} from '../src/rejoin/composition-profile.js';
import {rejoinFixtureProfile} from '../src/rejoin/fixture-profile.js';
import {verifyNativeLoot01Artifacts} from '../src/g3/native-loot01-profile.js';
import {verifyNativePotionCorrectionArtifacts} from '../src/g3/native-potion-correction-profile.js';
import {verifyNativePotionDiagnosticArtifacts} from '../src/g3/native-potion-diagnostic-profile.js';
import {verifyNativeC2DevelopmentArtifacts} from '../src/g3/native-c2-profile.js';
import {randomBytes,randomUUID,generateKeyPairSync} from 'node:crypto';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createServer} from 'node:net';
import {request as httpRequest} from 'node:http';
import {request as httpsRequest} from 'node:https';
import {checkServerIdentity} from 'node:tls';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import pg from 'pg';
import {createG3CandidateApp} from '../src/g3/candidate-loot01.js';
import {hash} from '../src/g1/security.js';
import {schema as lobbySchema} from '../../../packages/contracts/src/g2-next.js';
import {verifyNativeDevelopmentArtifacts} from '../src/g3/native-profile.js';
import {verifyNativeA1DevelopmentArtifacts} from '../src/g3/native-a1-profile.js';
import {verifyNativeA1RuntimeFixDevelopmentArtifacts} from '../src/g3/native-a1-runtime-fix-profile.js';
import {verifyNativeA1DiagnosticDevelopmentArtifacts} from '../src/g3/native-a1-diagnostic-profile.js';
import {verifyNativeA1HeartbeatBusyDevelopmentArtifacts} from '../src/g3/native-a1-heartbeat-busy-profile.js';
import {verifyNativeSocialDevelopmentArtifacts} from '../src/g3/native-social-profile.js';
import type {Credentials,Request,Response,Room,GameObservation,Inspection} from '../../../packages/contracts/g2-next/g2-next.js';
import type {Scope} from '../../../packages/contracts/g3/g3.js';
import {EntryReplyFault,installEntryReplyFault,type EntryReplyFaultPlan} from './entry-reply-fault.js';

export async function privateG3Directory(prefix:string){
 const directory=await mkdtemp(join(tmpdir(),prefix));
 try{
  if(process.platform==='win32'){
   const {stdout}=await promisify(execFile)('whoami.exe',['/user','/fo','csv','/nh'],{windowsHide:true,timeout:5000,maxBuffer:4096});
   const sid=stdout.match(/S-1-[0-9-]+/)?.[0];if(!sid)throw Error('Cannot resolve fixture owner SID');
   await promisify(execFile)('icacls.exe',[directory,'/inheritance:r','/grant:r',`*${sid}:(OI)(CI)F`],{windowsHide:true,timeout:5000,maxBuffer:4096});
  }
  return directory;
 }catch(e){await rm(directory,{recursive:true,force:true});throw e;}
}

// Disposable integration support only. Never accepts a caller-selected namespace/profile/key.
export async function startG3Fixture({testingStartingLoadout=false,worldRules=false,itemTransfers=false,b1Gold=false,debugDiagnostics=false,debugDiagnosticOptions,fixtureRuntime,configure,rejoinPackageVerifier, rejoinPackage=false,persistentIdentity=false,rejoinInput,rejoin=false,loot01Runtime=false,potionCorrection=false,potionDiagnostic=false,c2Runtime=false,live=false,tls=false,social=false,hourglass=false,nemesis=false,native=false,nativeSocial=false,a1Runtime=false,a1RuntimeFix=false,a1RuntimeDiagnostic=false,a1HeartbeatBusy=false,artifactReader,now,databaseUrl=process.env.NT2_G3_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1'}:{testingStartingLoadout?:boolean;worldRules?:boolean;itemTransfers?:boolean;b1Gold?:boolean;debugDiagnostics?:boolean;debugDiagnosticOptions?:import('../src/g3/debug-diagnostics.js').DebugOptions;fixtureRuntime?:ArtifactPin;rejoinPackageVerifier?:typeof verifyRejoinPackage;configure?:(service:Awaited<ReturnType<typeof createG3CandidateApp>>)=>void;rejoinPackage?:boolean;persistentIdentity?:boolean;rejoinInput?:ArtifactPin;rejoin?:boolean;loot01Runtime?:boolean;potionCorrection?:boolean;potionDiagnostic?:boolean;c2Runtime?:boolean;live?:boolean;tls?:boolean;social?:boolean;hourglass?:boolean;nemesis?:boolean;native?:boolean;nativeSocial?:boolean;a1Runtime?:boolean;a1RuntimeFix?:boolean;a1RuntimeDiagnostic?:boolean;a1HeartbeatBusy?:boolean;artifactReader?:Parameters<typeof verifyNativeDevelopmentArtifacts>[0];now?:()=>number;databaseUrl?:string}={}){
 if(process.env.NODE_ENV==='production')throw Error('Fixture refuses production');
 if(rejoinPackageVerifier&&(!rejoinPackage||!rejoin))throw Error('Normal metadata verifier requires isolated package fixture');
 if(native&&(!tls||social||hourglass))throw Error('Native development fixture requires TLS and cannot grant social capabilities');
 if(nemesis&&(native||hourglass||social))throw Error('Separate Nemesis simulated profile required');
 if(hourglass&&social)throw Error('Separate simulated profiles required');
 if(nativeSocial&&(!tls||social||native||hourglass||nemesis))throw Error('Native social source fixture requires TLS and a separate explicit profile');
 if(a1Runtime&&(!tls||!(native||nativeSocial)))throw Error('A1 runtime fixture requires explicit native profile and TLS');
 if(a1RuntimeFix&&!a1Runtime)throw Error('A1 runtime-fix fixture requires --a1-runtime');
 if(a1RuntimeDiagnostic&&(!a1Runtime||a1RuntimeFix))throw Error('A1 diagnostic fixture requires --a1-runtime and excludes --a1-runtime-fix');
 if(a1HeartbeatBusy&&(!a1Runtime||a1RuntimeFix||a1RuntimeDiagnostic))throw Error('A1 heartbeat-busy fixture requires --a1-runtime and excludes other runtime selectors');
 if([loot01Runtime,potionCorrection,potionDiagnostic,c2Runtime].filter(Boolean).length>1)throw Error('Separate correction/diagnostic/regular C2 selector required');
 if((loot01Runtime||potionCorrection||potionDiagnostic||c2Runtime)&&(!tls||!live||native||nativeSocial||social||hourglass||nemesis||a1Runtime||a1RuntimeFix||a1RuntimeDiagnostic||a1HeartbeatBusy))throw Error('C2 runtime requires --live --tls and excludes other native/profile selectors');
 if(fixtureRuntime&&(!rejoin||rejoinPackage||rejoinInput))throw Error('Separate isolated lifecycle runtime input required');
 const profile=rejoinPackage?await (rejoinPackageVerifier??verifyRejoinPackage)():rejoin?fixtureRuntime?await verifyLifecycleFixtureRuntime(fixtureRuntime):rejoinInput?await materializeRejoinComposition(rejoinInput):await rejoinFixtureProfile(b1Gold,itemTransfers,worldRules):loot01Runtime?await verifyNativeLoot01Artifacts(artifactReader):potionCorrection?await verifyNativePotionCorrectionArtifacts(artifactReader):potionDiagnostic?await verifyNativePotionDiagnosticArtifacts(artifactReader):c2Runtime?await verifyNativeC2DevelopmentArtifacts(artifactReader):a1HeartbeatBusy?await verifyNativeA1HeartbeatBusyDevelopmentArtifacts(nativeSocial,artifactReader):a1RuntimeDiagnostic?await verifyNativeA1DiagnosticDevelopmentArtifacts(nativeSocial,artifactReader):a1RuntimeFix?await verifyNativeA1RuntimeFixDevelopmentArtifacts(nativeSocial,artifactReader):a1Runtime?await verifyNativeA1DevelopmentArtifacts(nativeSocial,artifactReader):nativeSocial?await verifyNativeSocialDevelopmentArtifacts(artifactReader):native?await verifyNativeDevelopmentArtifacts(artifactReader):JSON.parse(await readFile(new URL('../../../deploy/profiles/'+(nemesis?'g3-nemesis-simulated-candidate.json':hourglass?'g3-hourglass-simulated-candidate-2.json':social?'g3-social-simulated-candidate.json':'g3-simulated-candidate.json'),import.meta.url),'utf8'));
 const namespace=(rejoin?'g3_rejoin_':loot01Runtime||potionCorrection?'g3_c2_potion_correction_fx_':potionDiagnostic?'g3_c2_potion_fixture_':c2Runtime?'g3_c2_fixture_':a1Runtime?(nativeSocial?'g3_social_a1_fixture_':'g3_a1_fixture_'):(nativeSocial?'g3_social_fixture_':'g3_fixture_'))+randomUUID().replaceAll('-','');
 if(rejoin){const target=new URL(databaseUrl);if(target.hostname!=='127.0.0.1'||target.port!=='54328'||target.pathname!=='/nt2_g1')throw Error('Rejoin fixture requires the named disposable Noita database');const verify=new pg.Pool({connectionString:databaseUrl,connectionTimeoutMillis:3000,statement_timeout:5000});try{const result=(await verify.query('SELECT current_database() AS database,current_user AS owner')).rows[0];if(result.database!=='nt2_g1'||result.owner!=='nt2_g1')throw Error('Unexpected fixture database identity');}finally{await verify.end();}}
 if(rejoin)console.log(JSON.stringify({allocatedFixtureSchema:namespace,databaseTarget:new URL(databaseUrl).hostname+':'+new URL(databaseUrl).port+new URL(databaseUrl).pathname}));
 const reservation=createServer();await new Promise<void>((resolve,reject)=>{reservation.once('error',reject);reservation.listen(0,'127.0.0.1',resolve);});
 const port=(reservation.address() as {port:number}).port;
 await new Promise<void>((resolve,reject)=>reservation.close(e=>e?reject(e):resolve()));
 const origin=`${tls?'https':'http'}://127.0.0.1:${port}`,pair=generateKeyPairSync('ed25519');
 let directory:string|undefined,ca:Buffer|undefined,transportTls:{pfx:Buffer;passphrase:string}|undefined;
 const lost=new Set<string>(),lostSessions=new Set<string>();
 const entryReplyFault=new EntryReplyFault();
 let service:Awaited<ReturnType<typeof createG3CandidateApp>>|undefined;
 const cleanupSchema=async()=>{const pool=new pg.Pool({connectionString:databaseUrl,connectionTimeoutMillis:3000,statement_timeout:5000});try{await pool.query(`DROP SCHEMA IF EXISTS "${namespace}" CASCADE`);if(rejoin){if((await pool.query('SELECT to_regnamespace($1) AS namespace',[namespace])).rows[0].namespace!==null)throw Error('Owned fixture schema cleanup incomplete');console.log(JSON.stringify({verifiedAbsentSchema:namespace}));}}finally{await pool.end();}};
 try{
  if(tls){
   directory=await privateG3Directory('nt2-g3-fixture-');
   const openssl=process.env.NT2_G3_OPENSSL??(process.platform==='win32'?'C:/Program Files/Git/usr/bin/openssl.exe':'openssl');
   const run=async(args:string[])=>{await promisify(execFile)(openssl,args,{windowsHide:true,cwd:directory!,timeout:15000,maxBuffer:65536});};
   await run(['req','-x509','-newkey','rsa:2048','-nodes','-keyout','root.key','-out','root.pem','-days','1','-subj','/CN=NT2 disposable G3 fixture root','-addext','basicConstraints=critical,CA:TRUE']);
   await run(['req','-newkey','rsa:2048','-nodes','-keyout','leaf.key','-out','leaf.csr','-subj','/CN=127.0.0.1','-addext','subjectAltName=IP:127.0.0.1','-addext','extendedKeyUsage=serverAuth']);
   await run(['x509','-req','-in','leaf.csr','-CA','root.pem','-CAkey','root.key','-CAcreateserial','-out','leaf.pem','-days','1','-copy_extensions','copy']);
   // Secret is used only by this test process; no certificate-store installation.
   await run(['pkcs12','-export','-inkey','leaf.key','-in','leaf.pem','-certfile','root.pem','-out','server.pfx','-passout','pass:fixture-only']);
   ca=await readFile(join(directory,'root.pem'));transportTls={pfx:await readFile(join(directory,'server.pfx')),passphrase:'fixture-only'};
  }
  const options={testingStartingLoadout,worldRules,b1Gold,itemTransfers,debugDiagnostics,...(debugDiagnosticOptions?{debugDiagnosticOptions}:{}),...(rejoinPackageVerifier?{rejoinPackageVerifier}:{}),...(fixtureRuntime?{rejoinFixtureRuntime:fixtureRuntime}:{}),rejoinPackage,...(rejoinInput?{rejoinInput}:{}),rejoin,databaseUrl,namespace,serviceId:profile.serviceId,mockEnabled:true,website:true,loot01Runtime,potionCorrection,potionDiagnostic,c2Runtime,live,social,hourglass,nemesis,native,nativeSocial,a1Runtime,a1RuntimeFix,a1RuntimeDiagnostic,a1HeartbeatBusy,...(artifactReader?{nativeArtifactReader:artifactReader}:{}),...(now?{now}:{}),compatibility:profile.compatibility,
   keys:persistentIdentity?await persistentRejoinIdentity(databaseUrl,profile.serviceId,namespace,origin):{privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}).toString(),publicKey:pair.publicKey.export({type:'spki',format:'pem'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:origin},
   ...(transportTls?{tls:transportTls,canonicalOrigin:origin}:{})};
  const installLoss=(current:Awaited<ReturnType<typeof createG3CandidateApp>>)=>{configure?.(current);current.app.addHook('onRequest',async(req,reply)=>{const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];if(!token)return;let denied=lost.has(token);if(!denied&&lostSessions.size){try{const identity=await current.auth.security.verify(token,current.auth.now());denied=lostSessions.has(String(identity.sid));}catch{}}if(denied)await reply.code(503).header('cache-control','no-store').send({error:{code:'SERVICE_UNAVAILABLE'}});});
   if(rejoin)installEntryReplyFault(current.app,entryReplyFault,async c=>current.db.transaction(async tx=>!!(await tx.query(`SELECT 1 FROM startup_entry_operations o JOIN startup_entry_effects e ON e.pending_id=o.pending_id
    WHERE o.pending_id=$1 AND e.world_token=$2 AND o.request_id=$3 AND o.account_id=$4 AND o.device_id=$5 AND o.run_id=$6 AND o.kind='entryActivate'
    AND o.fingerprint=$7 AND o.original->>'originalSessionEpoch'=$8 AND o.original->>'originalServiceEpoch'=$9
    AND e.attachment_id IS NOT NULL AND o.effect_id=e.attachment_id::text`,[c.pendingId,c.worldToken,c.requestId,c.accountId,c.deviceId,c.runId,c.fingerprint,c.sessionEpoch,c.serviceEpoch])).rowCount,true,AbortSignal.timeout(3000)));
  };
  service=await createG3CandidateApp(options);installLoss(service);await service.app.listen({host:'127.0.0.1',port});
  let closing:Promise<void>|undefined;
  const close=()=>closing??= (async()=>{entryReplyFault.close();try{const drained=service?.app.close();service?.app.server.closeAllConnections();await drained;}finally{try{await cleanupSchema();}finally{if(directory)await rm(directory,{recursive:true,force:true});}}})();
  const call=(path:string,body?:unknown,token?:string,headers:Record<string,string>={},timeoutMs=5000)=>new Promise<{status:number;headers:import('node:http').IncomingHttpHeaders;body:string}>((resolve,reject)=>{
   if(!Number.isInteger(timeoutMs)||timeoutMs<1||timeoutMs>5000)return reject(Error('Fixture deadline bounds'));
   if(!/^\/(?:rpc\/g3\/race|v[13]\/[a-z-]+|v3\/(social|history|stats|hourglass|nemesis|live|rejoin|connection|waiting|lifecycle|c1|c2)\/(rpc|info|schema|catalogue)|v3\/social\/runtime|v1\/mock\/browser-login|signin\/[A-Za-z0-9_-]+|schema\.json)$/.test(path))return reject(Error('Fixture path outside allowed routes'));
   if(token&&lost.has(token))return reject(Error('Injected affected transport loss'));
   const payload=body===undefined?undefined:typeof body==='string'?body:JSON.stringify(body);
   if(payload&&Buffer.byteLength(payload)>(itemTransfers&&path==='/v3/rpc'?262144:65536))return reject(Error('Fixture request too large'));
   const req=(tls?httpsRequest:httpRequest)(origin+path,{agent:false,method:payload===undefined?'GET':'POST',...(ca?{ca,servername:'',checkServerIdentity:(_host:string,cert:import('node:tls').PeerCertificate)=>checkServerIdentity('127.0.0.1',cert)}:{}),headers:{...(payload===undefined?{}:{'content-type':'application/json'}),...(token?{authorization:'Bearer '+token}:{}),...headers}},res=>{
    const chunks:Buffer[]=[];let size=0;res.on('data',(chunk:Buffer)=>{size+=chunk.length;if(size>262144)req.destroy(Error('Fixture response too large'));else chunks.push(chunk);});
    res.on('error',reject);res.on('end',()=>resolve({status:res.statusCode!,headers:res.headers,body:Buffer.concat(chunks).toString('utf8')}));
   });const deadline=setTimeout(()=>req.destroy(Error('Fixture deadline exceeded')),timeoutMs);req.on('close',()=>clearTimeout(deadline));req.on('error',reject);req.end(payload);
  });
  return {origin,namespace,profile,rejoinEnabled:rejoin,caPath:directory?join(directory,'root.pem'):undefined,ca,call,close,
   async armEntryActivationReply(plan:EntryReplyFaultPlan){if(!rejoin)throw Error('Entry reply fault requires owned rejoin fixture');return entryReplyFault.arm(plan,async target=>service!.db.transaction(async tx=>!(await tx.query("SELECT 1 FROM startup_entry_effects WHERE pending_id=$1 OR world_token=$2 UNION ALL SELECT 1 FROM startup_pending WHERE id=$1 OR intent->>'worldToken'=$2 UNION ALL SELECT 1 FROM waiting_worlds WHERE intent->>'worldToken'=$2 UNION ALL SELECT 1 FROM world_observations WHERE world_token=$2 LIMIT 1",[target.pendingId,target.worldToken])).rowCount,true,AbortSignal.timeout(3000)));},
   entryActivationReplySnapshot:()=>entryReplyFault.snapshot(),releaseEntryActivationReply:(armId:string)=>entryReplyFault.release(armId),
   get service(){return service!;},setTransportLoss(tokens:string[],enabled:boolean){if(!rejoin||!Array.isArray(tokens)||tokens.length>8||tokens.some(t=>typeof t!=='string'||t.length>4096))throw Error('Bounded rejoin fixture loss only');for(const token of tokens)enabled?lost.add(token):lost.delete(token);},async setSessionLoss(desktopId:string,enabled:boolean){if(!rejoin||!/^[-0-9a-f]{36}$/.test(desktopId))throw Error('Owned fixture desktop required');const sessions=await service!.db.transaction(async tx=>(await tx.query('SELECT id FROM sessions WHERE id=$1 OR parent_id=$1',[desktopId])).rows);if(!sessions.length)throw Error('Unknown fixture desktop');for(const s of sessions)enabled?lostSessions.add(s.id):lostSessions.delete(s.id);},async loseParticipant(participantId:string){if(!rejoin||!/^[-0-9a-f]{36}$/.test(participantId))throw Error('Owned rejoin participant required');await service!.db.transaction(async tx=>{const authority=(service!.rooms as import('../src/rejoin/rooms.js').RejoinRooms).recovery;const p=await authority.read(tx,participantId);if(!p)throw Error('Unknown fixture participant');await authority.fence(tx,p);await tx.query("UPDATE rejoin_participants SET state='reconnecting',reason='fixture_connection_lost' WHERE id=$1 AND NOT terminal",[p.id]);});},async restart(){entryReplyFault.releaseAll('fixture-restart');const drained=service!.app.close();service!.app.server.closeAllConnections();await drained;if(persistentIdentity)options.keys=await persistentRejoinIdentity(databaseUrl,profile.serviceId,namespace,origin);service=await createG3CandidateApp(options);installLoss(service);await service.app.listen({host:'127.0.0.1',port});}};
 }catch(e){entryReplyFault.close();try{const drained=service?.app.close();service?.app.server.closeAllConnections();await drained;}finally{try{await cleanupSchema();}finally{if(directory)await rm(directory,{recursive:true,force:true});}}throw e;}
}

// Uses real HTTP routes, proof-bound sign-in, protected attachments and exact inspected Start.
// All game facts below are deliberately simulated; no native artifact receives approval.
export async function provisionG3Fixture(f:Awaited<ReturnType<typeof startG3Fixture>>,{consumerPlayer,seedPolicy=f.profile.seedLoot?'shared-random':undefined,randomizeLoot=!!f.profile.seedLoot,actualSeeds,automatic=!!f.profile.seedLoot,mode=f.profile.id==='g3-nemesis-simulated-candidate-1'?'nemesis':'core',settings,roomName='G3 simulated HTTP fixture',requiredCapabilities=f.profile.compatibility.capabilities,playerCount=2,worldTokens,displayNames,devicePrefix='g3-fixture',beforeRpc}:{consumerPlayer?:number;seedPolicy?:string;randomizeLoot?:boolean;actualSeeds?:string[];automatic?:boolean;mode?:string;settings?:{key:string;value:string}[];roomName?:string;requiredCapabilities?:string[];playerCount?:number;worldTokens?:string[];displayNames?:string[];devicePrefix?:string;beforeRpc?:()=>void}={}){
 if(consumerPlayer!==undefined&&(!Number.isInteger(consumerPlayer)||consumerPlayer<0||consumerPlayer>=playerCount))throw Error('Consumer player outside fixture roster');
 if(!Number.isInteger(playerCount)||playerCount<2||playerCount>32)throw Error('Fixture player count outside bounded roster');
 let room:Room|undefined;const commandNames:string[]=[];
 const rpc=async(c:Credentials|undefined,payload:Partial<Request>)=>{
  beforeRpc?.();
  const input={major:1,revision:lobbySchema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId:randomUUID(),...(c?{sessionEpoch:c.sessionEpoch!}:{}),...(room?{roomId:room.roomId!,runId:room.runId??'',runEpoch:room.runEpoch??'',configRevision:room.configRevision!,expectedRevision:room.revision!}:{}),...payload};
  commandNames.push(Object.keys(payload).find(key=>lobbySchema.messages.Request!.oneof?.includes(key))??'unknown');
  const response=await f.call('/v1/rpc',input,c?.accessToken);if(response.status!==200)throw Error('Fixture RPC HTTP '+response.status);
  const r=JSON.parse(response.body) as Response;if(r.error)throw Error('Fixture RPC '+r.error.code);if(r.room)room=r.room;return r;
 };
 const signIn=async(subject:'alex'|'sam')=>{
  const deviceId=devicePrefix+'-'+subject,appInstanceId=randomUUID(),initiatorProof=randomBytes(32).toString('base64url'),callbackState=randomBytes(32).toString('base64url');
  const begun=await rpc(undefined,{authBegin:{provider:'website',deviceId,appInstanceId,proofChallenge:hash(initiatorProof),callbackPort:49152,callbackState}});
  const path=new URL(begun.authAttempt!.launchUrl!).pathname,page=await f.call(path),cookie=String(page.headers['set-cookie']).split(';')[0]!;
  const csrf=page.body.match(/name="csrf" value="([^"]+)"/)?.[1];if(!csrf)throw Error('Missing fixture CSRF');
  const returned=await f.call(path,new URLSearchParams({csrf,subject}).toString(),undefined,{origin:f.origin,cookie,'content-type':'application/x-www-form-urlencoded'});
  const fields=Object.fromEntries([...returned.body.matchAll(/name="([^"]+)" value="([^"]+)"/g)].map(m=>[m[1],m[2]]));
  if(returned.status!==200||fields.callbackState!==callbackState||!fields.completionGrant)throw Error('Fixture callback rejected');
  return (await rpc(undefined,{authExchange:{attemptId:begun.authAttempt!.attemptId!,deviceId,appInstanceId,initiatorProof,completionGrant:fields.completionGrant}})).credentials!;
 };
 const hostResponse=await f.call('/v1/mock/browser-login',{subject:'website-local-alex',deviceId:devicePrefix+'-host',requestId:randomUUID()});
 const host=JSON.parse(hostResponse.body).credentials as Credentials;if(!host)throw Error('Fixture host rejected');
 await rpc(host,{roomCreate:{name:roomName,allowGuests:playerCount>2,config:{seed:seedPolicy&&seedPolicy!=='shared-fixed'?0:424242,...(seedPolicy?{seedPolicy,randomizeLoot:seedPolicy==='individual-random'?false:randomizeLoot}:{}),mode,requiredCapabilities,...(settings?{settings}:f.profile.id==='g3-nemesis-simulated-candidate-1'?{settings:[{key:'nemesis_abilities',value:'true'}]}:{})}}});
 const desktops=[await signIn('alex'),await signIn('sam')],injectedIndices:number[]=[],games:Credentials[]=[],scopes:Scope[]=[],observations:GameObservation[]=[];
 // Additional simulated players use the same public guest issuance and issuer;
 // no injected auth actor or alternate credential signer is introduced.
 for(let i=2;i<playerCount;i++)desktops.push((await rpc(undefined,{guestIssue:{deviceId:devicePrefix+'-guest-'+i,issuanceProof:randomBytes(32).toString('base64url'),displayName:displayNames?.[i]??'Fixture player '+i}})).credentials!);
 for(let i=0;i<2;i++)if(displayNames?.[i])await f.service.db.transaction(tx=>tx.query('UPDATE principal_profiles SET display_name=$2 WHERE principal_id=$1',[desktops[i]!.principal!.principalId,displayNames[i]]));
 const previews:Room[]=[];
 for(const [i,desktop] of desktops.entries()){
  if(automatic){const preview=await rpc(desktop,{roomPreview:{}});if(!preview.room?.config||preview.room.configRevision!==preview.admission?.configRevision||preview.room.revision!==preview.admission?.revision)throw Error('Incoherent fixture preview');previews.push(preview.room);}
  await rpc(desktop,{roomJoin:{}});if(!automatic)await rpc(desktop,{ready:{ready:true}});
 }
 for(const [i,desktop] of desktops.entries()){
  if(i===consumerPlayer)continue;
  const prep=(await rpc(desktop,automatic?{roomGet:{}}:{prepare:{}})).preparation!,gameInstanceId='g3-fixture-game-'+i;
  const game=(await rpc(desktop,{gameAttach:{preparationId:prep.preparationId!,gameInstanceId,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}})).credentials!;
  const worldToken=worldTokens?.[i]??'g3-fixture-world-'+i,observation:GameObservation={attachmentId:game.attachmentId!,gameInstanceId,worldToken,observationRevision:1,actualSeed:actualSeeds?.[i]??(seedPolicy==='individual-random'?String(123456+i):String(room!.config!.seed)),seedStatus:'known',activeMods:['noita_together2'],inventoryComplete:true,collectorFailure:'',beta:'unknown',protected:true,loaded:true,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,compatibilityProvenance:'game-thread-client-observation'};
  await rpc(game,{gameObservation:observation});injectedIndices.push(i);games.push(game);observations.push(observation);
  scopes.push({roomId:room!.roomId!,runId:room!.runId!,runEpoch:room!.runEpoch!,configRevision:room!.configRevision!,attachmentId:game.attachmentId!,worldToken});
 }
 const reports:Inspection[]=[];let cursor='';do{const r=await rpc(host,{inspectionList:{limit:1,afterDesktopId:cursor}});reports.push(...r.inspections!);cursor=r.inspectionNextDesktopId??'';}while(cursor);
 const start=async()=>{await rpc(host,{roomGet:{}});const currentReports:Inspection[]=[];let after='';do{const result=await rpc(host,{inspectionList:{limit:1,afterDesktopId:after}});currentReports.push(...result.inspections!);after=result.inspectionNextDesktopId??'';}while(after);return rpc(host,{finalStart:{participants:currentReports.map(r=>({desktopId:r.desktopId!,attachmentId:r.attachmentId!,worldToken:r.worldToken!,observationRevision:r.observationRevision!,contentDigest:r.contentDigest!})),excludedDesktopIds:[]}});};
 const recoverInjectedPeers=async()=>{
  if(!f.rejoinEnabled||consumerPlayer===undefined)throw Error('Explicit consumer-owned fixture required for injected peer recovery');
  await rpc(host,{roomGet:{}});
  const rejoinCall=async(c:Credentials,payload:Record<string,unknown>)=>{const reply=await f.call('/v3/rejoin/rpc',{major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),...payload},c.accessToken);const body=JSON.parse(reply.body);if(body.error)throw Error('Injected peer recovery '+body.error.code);return body;};
  for(const [index,desktopIndex]of injectedIndices.entries()){
   const desktop=desktops[desktopIndex]!,status=await rejoinCall(desktop,{status:{}});if(!status.participant?.returnEligible)throw Error('Injected peer terminal');if(status.participant.state==='active')continue;
   const p=status.participant,a=p.association,scope={roomId:a.roomId,runId:a.runId,participantId:a.participantId,worldToken:a.worldToken,configRevision:a.configRevision,generation:p.generation};
   const gameInstanceId='injected-peer-return-'+randomUUID(),resumed=await rejoinCall(desktop,{...scope,resume:{resumeProof:status.resumeProof,gameInstanceId,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}});
   const game=resumed.credentials as Credentials,observation={...observations[index]!,attachmentId:game.attachmentId!,gameInstanceId,observationRevision:1,protected:true};
   const accepted=await rejoinCall(game,{...scope,generation:resumed.participant.generation,observe:observation});if(!accepted.release)throw Error('Injected peer inspection did not authorize');
   await rejoinCall(game,{...scope,generation:resumed.participant.generation,observe:{...observation,observationRevision:2,protected:false}});
   games[index]=game;observations[index]=observation;scopes[index]={roomId:a.roomId,runId:a.runId,runEpoch:resumed.participant.runEpoch,configRevision:a.configRevision,attachmentId:game.attachmentId!,worldToken:a.worldToken};
  }
 };
 const playing=async()=>{for(const [i,game] of games.entries())await rpc(game,{gameObservation:{...observations[i]!,observationRevision:2,protected:false}});};
 const pulse=async()=>{for(const desktop of desktops)await rpc(desktop,{presence:{present:true}});for(const [i,game] of games.entries())await rpc(game,{gamePulse:{attachmentId:game.attachmentId!,gameInstanceId:observations[i]!.gameInstanceId!,worldToken:observations[i]!.worldToken!}});};
 return {host,desktops,games,scopes,observations,reports,previews,commandNames,consumerPlayer,injectedDesktopIds:desktops.filter((_d,i)=>i!==consumerPlayer).map(d=>d.sessionId),rpc,start,playing,pulse,recoverInjectedPeers,get room(){return room!;}};
}




