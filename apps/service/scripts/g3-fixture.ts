import {verifyNativeSeedLootArtifacts} from '../src/g3/native-seed-loot-profile.js';
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
import {createG3CandidateApp} from '../src/g3/candidate.js';
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

export async function privateG3Directory(prefix:string){
 const directory=await mkdtemp(join(tmpdir(),prefix));
 try{
  if(process.platform==='win32'){
   const {stdout}=await promisify(execFile)('whoami.exe',['/user','/fo','csv','/nh'],{timeout:5000,maxBuffer:4096});
   const sid=stdout.match(/S-1-[0-9-]+/)?.[0];if(!sid)throw Error('Cannot resolve fixture owner SID');
   await promisify(execFile)('icacls.exe',[directory,'/inheritance:r','/grant:r',`*${sid}:(OI)(CI)F`],{timeout:5000,maxBuffer:4096});
  }
  return directory;
 }catch(e){await rm(directory,{recursive:true,force:true});throw e;}
}

// Disposable integration support only. Never accepts a caller-selected namespace/profile/key.
export async function startG3Fixture({debugDiagnostics=false,seedLootRuntime=false,potionCorrection=false,potionDiagnostic=false,c2Runtime=false,live=false,tls=false,social=false,hourglass=false,nemesis=false,native=false,nativeSocial=false,a1Runtime=false,a1RuntimeFix=false,a1RuntimeDiagnostic=false,a1HeartbeatBusy=false,artifactReader,now,databaseUrl=process.env.NT2_G3_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1'}:{debugDiagnostics?:boolean;seedLootRuntime?:boolean;potionCorrection?:boolean;potionDiagnostic?:boolean;c2Runtime?:boolean;live?:boolean;tls?:boolean;social?:boolean;hourglass?:boolean;nemesis?:boolean;native?:boolean;nativeSocial?:boolean;a1Runtime?:boolean;a1RuntimeFix?:boolean;a1RuntimeDiagnostic?:boolean;a1HeartbeatBusy?:boolean;artifactReader?:Parameters<typeof verifyNativeDevelopmentArtifacts>[0];now?:()=>number;databaseUrl?:string}={}){
 if(process.env.NODE_ENV==='production')throw Error('Fixture refuses production');
 if(native&&(!tls||social||hourglass))throw Error('Native development fixture requires TLS and cannot grant social capabilities');
 if(nemesis&&(native||hourglass||social))throw Error('Separate Nemesis simulated profile required');
 if(hourglass&&social)throw Error('Separate simulated profiles required');
 if(nativeSocial&&(!tls||social||native||hourglass||nemesis))throw Error('Native social source fixture requires TLS and a separate explicit profile');
 if(a1Runtime&&(!tls||!(native||nativeSocial)))throw Error('A1 runtime fixture requires explicit native profile and TLS');
 if(a1RuntimeFix&&!a1Runtime)throw Error('A1 runtime-fix fixture requires --a1-runtime');
 if(a1RuntimeDiagnostic&&(!a1Runtime||a1RuntimeFix))throw Error('A1 diagnostic fixture requires --a1-runtime and excludes --a1-runtime-fix');
 if(a1HeartbeatBusy&&(!a1Runtime||a1RuntimeFix||a1RuntimeDiagnostic))throw Error('A1 heartbeat-busy fixture requires --a1-runtime and excludes other runtime selectors');
 if([seedLootRuntime,potionCorrection,potionDiagnostic,c2Runtime].filter(Boolean).length>1)throw Error('Separate correction/diagnostic/regular C2 selector required');
 if((seedLootRuntime||potionCorrection||potionDiagnostic||c2Runtime)&&(!tls||!live||native||nativeSocial||social||hourglass||nemesis||a1Runtime||a1RuntimeFix||a1RuntimeDiagnostic||a1HeartbeatBusy))throw Error('C2 runtime requires --live --tls and excludes other native/profile selectors');
 const profile=seedLootRuntime?await verifyNativeSeedLootArtifacts(artifactReader):potionCorrection?await verifyNativePotionCorrectionArtifacts(artifactReader):potionDiagnostic?await verifyNativePotionDiagnosticArtifacts(artifactReader):c2Runtime?await verifyNativeC2DevelopmentArtifacts(artifactReader):a1HeartbeatBusy?await verifyNativeA1HeartbeatBusyDevelopmentArtifacts(nativeSocial,artifactReader):a1RuntimeDiagnostic?await verifyNativeA1DiagnosticDevelopmentArtifacts(nativeSocial,artifactReader):a1RuntimeFix?await verifyNativeA1RuntimeFixDevelopmentArtifacts(nativeSocial,artifactReader):a1Runtime?await verifyNativeA1DevelopmentArtifacts(nativeSocial,artifactReader):nativeSocial?await verifyNativeSocialDevelopmentArtifacts(artifactReader):native?await verifyNativeDevelopmentArtifacts(artifactReader):JSON.parse(await readFile(new URL('../../../deploy/profiles/'+(nemesis?'g3-nemesis-simulated-candidate.json':hourglass?'g3-hourglass-simulated-candidate-2.json':social?'g3-social-simulated-candidate.json':'g3-simulated-candidate.json'),import.meta.url),'utf8'));
 const namespace=(seedLootRuntime||potionCorrection?'g3_c2_potion_correction_fx_':potionDiagnostic?'g3_c2_potion_fixture_':c2Runtime?'g3_c2_fixture_':a1Runtime?(nativeSocial?'g3_social_a1_fixture_':'g3_a1_fixture_'):(nativeSocial?'g3_social_fixture_':'g3_fixture_'))+randomUUID().replaceAll('-','');
 const reservation=createServer();await new Promise<void>((resolve,reject)=>{reservation.once('error',reject);reservation.listen(0,'127.0.0.1',resolve);});
 const port=(reservation.address() as {port:number}).port;
 await new Promise<void>((resolve,reject)=>reservation.close(e=>e?reject(e):resolve()));
 const origin=`${tls?'https':'http'}://127.0.0.1:${port}`,pair=generateKeyPairSync('ed25519');
 let directory:string|undefined,ca:Buffer|undefined,transportTls:{pfx:Buffer;passphrase:string}|undefined;
 let service:Awaited<ReturnType<typeof createG3CandidateApp>>|undefined;
 const cleanupSchema=async()=>{const pool=new pg.Pool({connectionString:databaseUrl,connectionTimeoutMillis:3000,statement_timeout:5000});try{await pool.query(`DROP SCHEMA IF EXISTS "${namespace}" CASCADE`);}finally{await pool.end();}};
 try{
  if(tls){
   directory=await privateG3Directory('nt2-g3-fixture-');
   const openssl=process.env.NT2_G3_OPENSSL??(process.platform==='win32'?'C:/Program Files/Git/usr/bin/openssl.exe':'openssl');
   const run=async(args:string[])=>{await promisify(execFile)(openssl,args,{cwd:directory!,timeout:15000,maxBuffer:65536});};
   await run(['req','-x509','-newkey','rsa:2048','-nodes','-keyout','root.key','-out','root.pem','-days','1','-subj','/CN=NT2 disposable G3 fixture root','-addext','basicConstraints=critical,CA:TRUE']);
   await run(['req','-newkey','rsa:2048','-nodes','-keyout','leaf.key','-out','leaf.csr','-subj','/CN=127.0.0.1','-addext','subjectAltName=IP:127.0.0.1','-addext','extendedKeyUsage=serverAuth']);
   await run(['x509','-req','-in','leaf.csr','-CA','root.pem','-CAkey','root.key','-CAcreateserial','-out','leaf.pem','-days','1','-copy_extensions','copy']);
   // Secret is used only by this test process; no certificate-store installation.
   await run(['pkcs12','-export','-inkey','leaf.key','-in','leaf.pem','-certfile','root.pem','-out','server.pfx','-passout','pass:fixture-only']);
   ca=await readFile(join(directory,'root.pem'));transportTls={pfx:await readFile(join(directory,'server.pfx')),passphrase:'fixture-only'};
  }
  const options={debugDiagnostics,databaseUrl,namespace,serviceId:profile.serviceId,mockEnabled:true,website:true,seedLootRuntime,potionCorrection,potionDiagnostic,c2Runtime,live,social,hourglass,nemesis,native,nativeSocial,a1Runtime,a1RuntimeFix,a1RuntimeDiagnostic,a1HeartbeatBusy,...(artifactReader?{nativeArtifactReader:artifactReader}:{}),...(now?{now}:{}),compatibility:profile.compatibility,
   keys:{privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}).toString(),publicKey:pair.publicKey.export({type:'spki',format:'pem'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:origin},
   ...(transportTls?{tls:transportTls,canonicalOrigin:origin}:{})};
  service=await createG3CandidateApp(options);await service.app.listen({host:'127.0.0.1',port});
  let closing:Promise<void>|undefined;
  const close=()=>closing??= (async()=>{try{const drained=service?.app.close();service?.app.server.closeAllConnections();await drained;}finally{try{await cleanupSchema();}finally{if(directory)await rm(directory,{recursive:true,force:true});}}})();
  const call=(path:string,body?:unknown,token?:string,headers:Record<string,string>={})=>new Promise<{status:number;headers:import('node:http').IncomingHttpHeaders;body:string}>((resolve,reject)=>{
   if(!/^\/(?:v[13]\/[a-z-]+|v3\/(social|history|hourglass|nemesis|live)\/(rpc|info|schema)|v3\/social\/runtime|v1\/mock\/browser-login|signin\/[A-Za-z0-9_-]+|schema\.json)$/.test(path))return reject(Error('Fixture path outside allowed routes'));
   const payload=body===undefined?undefined:typeof body==='string'?body:JSON.stringify(body);
   if(payload&&Buffer.byteLength(payload)>65536)return reject(Error('Fixture request too large'));
   const req=(tls?httpsRequest:httpRequest)(origin+path,{agent:false,method:payload===undefined?'GET':'POST',...(ca?{ca,servername:'',checkServerIdentity:(_host:string,cert:import('node:tls').PeerCertificate)=>checkServerIdentity('127.0.0.1',cert)}:{}),headers:{...(payload===undefined?{}:{'content-type':'application/json'}),...(token?{authorization:'Bearer '+token}:{}),...headers}},res=>{
    const chunks:Buffer[]=[];let size=0;res.on('data',(chunk:Buffer)=>{size+=chunk.length;if(size>262144)req.destroy(Error('Fixture response too large'));else chunks.push(chunk);});
    res.on('error',reject);res.on('end',()=>resolve({status:res.statusCode!,headers:res.headers,body:Buffer.concat(chunks).toString('utf8')}));
   });const deadline=setTimeout(()=>req.destroy(Error('Fixture deadline exceeded')),5000);req.on('close',()=>clearTimeout(deadline));req.on('error',reject);req.end(payload);
  });
  return {origin,namespace,profile,caPath:directory?join(directory,'root.pem'):undefined,ca,call,close,
   get service(){return service!;},async restart(){const drained=service!.app.close();service!.app.server.closeAllConnections();await drained;service=await createG3CandidateApp(options);await service.app.listen({host:'127.0.0.1',port});}};
 }catch(e){try{const drained=service?.app.close();service?.app.server.closeAllConnections();await drained;}finally{try{await cleanupSchema();}finally{if(directory)await rm(directory,{recursive:true,force:true});}}throw e;}
}

// Uses real HTTP routes, proof-bound sign-in, protected attachments and exact inspected Start.
// All game facts below are deliberately simulated; no native artifact receives approval.
export async function provisionG3Fixture(f:Awaited<ReturnType<typeof startG3Fixture>>,{seedPolicy=f.profile.seedLoot?'shared-random':undefined,randomizeLoot=!!f.profile.seedLoot,actualSeeds,automatic=!!f.profile.seedLoot,mode=f.profile.id==='g3-nemesis-simulated-candidate-1'?'nemesis':'core',settings,roomName='G3 simulated HTTP fixture',requiredCapabilities=f.profile.compatibility.capabilities,playerCount=2,worldTokens,displayNames,devicePrefix='g3-fixture',beforeRpc}:{seedPolicy?:string;randomizeLoot?:boolean;actualSeeds?:string[];automatic?:boolean;mode?:string;settings?:{key:string;value:string}[];roomName?:string;requiredCapabilities?:string[];playerCount?:number;worldTokens?:string[];displayNames?:string[];devicePrefix?:string;beforeRpc?:()=>void}={}){
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
 const desktops=[await signIn('alex'),await signIn('sam')],games:Credentials[]=[],scopes:Scope[]=[],observations:GameObservation[]=[];
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
  const prep=(await rpc(desktop,automatic?{roomGet:{}}:{prepare:{}})).preparation!,gameInstanceId='g3-fixture-game-'+i;
  const game=(await rpc(desktop,{gameAttach:{preparationId:prep.preparationId!,gameInstanceId,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}})).credentials!;
  const worldToken=worldTokens?.[i]??'g3-fixture-world-'+i,observation:GameObservation={attachmentId:game.attachmentId!,gameInstanceId,worldToken,observationRevision:1,actualSeed:actualSeeds?.[i]??(seedPolicy==='individual-random'?String(123456+i):String(room!.config!.seed)),seedStatus:'known',activeMods:['noita_together2'],inventoryComplete:true,collectorFailure:'',beta:'unknown',protected:true,loaded:true,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,compatibilityProvenance:'game-thread-client-observation'};
  await rpc(game,{gameObservation:observation});games.push(game);observations.push(observation);
  scopes.push({roomId:room!.roomId!,runId:room!.runId!,runEpoch:room!.runEpoch!,configRevision:room!.configRevision!,attachmentId:game.attachmentId!,worldToken});
 }
 const reports:Inspection[]=[];let cursor='';do{const r=await rpc(host,{inspectionList:{limit:1,afterDesktopId:cursor}});reports.push(...r.inspections!);cursor=r.inspectionNextDesktopId??'';}while(cursor);
 const start=()=>rpc(host,{finalStart:{participants:reports.map(r=>({desktopId:r.desktopId!,attachmentId:r.attachmentId!,worldToken:r.worldToken!,observationRevision:r.observationRevision!,contentDigest:r.contentDigest!})),excludedDesktopIds:[]}});
 const playing=async()=>{for(const [i,game] of games.entries())await rpc(game,{gameObservation:{...observations[i]!,observationRevision:2,protected:false}});};
 const pulse=async()=>{for(const desktop of desktops)await rpc(desktop,{presence:{present:true}});for(const [i,game] of games.entries())await rpc(game,{gamePulse:{attachmentId:game.attachmentId!,gameInstanceId:observations[i]!.gameInstanceId!,worldToken:observations[i]!.worldToken!}});};
 return {host,desktops,games,scopes,observations,reports,previews,commandNames,rpc,start,playing,pulse,get room(){return room!;}};
}
