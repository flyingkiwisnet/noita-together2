import {TESTING_STARTING_LOADOUT,testingLoadoutCompatibility} from '../../../../packages/domain/src/testing-starting-loadout.js';
import {credentialValidity} from '../g3/connection.js';
import Fastify,{type FastifyInstance} from 'fastify';
import {readFile} from 'node:fs/promises';
import {CandidateDatabase} from './database.js';
import {PrincipalAuth} from './auth.js';
import {CandidateRooms} from './rooms.js';
import {MockSignIn} from './signin.js';
import {Auth1,registerAuth1} from '../auth1/routes.js';
import type {AuthenticationOptions} from '../auth1/providers.js';
import {parseJson as parseAuth1} from '../../../../packages/contracts/src/g3-auth1.js';
import {registerMockSignInPage} from './signin-page.js';
import {validateCandidateMetadata} from './profile.js';
import {Security,PolicyError,demand,fingerprint} from '../g1/security.js';
import {WorkGate} from '../g1/work.js';
import {declareRoomSemantics} from '../../../../packages/contracts/src/room-semantics.js';
import {installTrafficBudgets,explicitTrustedProxies,type TrafficBudgets} from './traffic.js';
import type {Options} from '../g1/app.js';
import {schema,parseJson,decode,encode,validate} from '../../../../packages/contracts/src/g2-next.js';
import type {Request,Response} from '../../../../packages/contracts/g2-next/g2-next.js';
import type {CompatibilityProfile} from '../../../../packages/contracts/g2-next/g2-next.js';
export type CandidateOptions=Omit<Options,'profile'>&{profile?:CompatibilityProfile;trustedProxies?:string[];testingStartingLoadout?:boolean;authentication?:AuthenticationOptions};
// Internal composition seam for the separately versioned G3 candidate. No launcher switch.
export interface CandidateExtension {
 namespacePrefix:'g3_';
 infoProfile?:CompatibilityProfile;
 database:(options:CandidateOptions)=>CandidateDatabase;
 rooms:(db:CandidateDatabase,auth:PrincipalAuth,options:CandidateOptions)=>CandidateRooms;
 auth?:((db:CandidateDatabase,security:Security,options:CandidateOptions)=>PrincipalAuth)|undefined;
 requestHousekeeping?:((request:Request,rooms:CandidateRooms)=>Promise<void>)|undefined;
 parseJson?:(url:string,text:string)=>unknown;
 decorate?:((tx:import('../g1/database.js').Tx,input:Request,value:Partial<Response>,token?:string)=>Promise<Partial<Response>>)|undefined;
 register:(app:FastifyInstance,components:{db:CandidateDatabase;auth:PrincipalAuth;rooms:CandidateRooms;traffic:TrafficBudgets;webToken?:(req:import('fastify').FastifyRequest)=>Promise<string|undefined>})=>Promise<void>;
}
// Explicit candidate factory only; no accepted service launcher switches to it.
export async function createCandidateApp(options:CandidateOptions,extension?:CandidateExtension){
 demand(options.authentication?!options.mockEnabled:options.mockEnabled&&process.env.NODE_ENV!=='production','FORBIDDEN');demand(options.namespace?.startsWith(extension?.namespacePrefix??'g2_next_'),'FORBIDDEN');
 if(options.authentication){demand(options.authentication.origin===options.keys.issuer,'INVALID_AUTH_CONFIGURATION');if(options.authentication.fixture){const db=new URL(options.databaseUrl);demand(/^g3_rejoin_[a-f0-9]{32}$/.test(options.namespace??'')&&db.hostname==='127.0.0.1'&&db.port==='54328'&&db.pathname==='/nt2_g1','FORBIDDEN');}}
 validateCandidateMetadata(options);
 demand(options.testingStartingLoadout===undefined||typeof options.testingStartingLoadout==='boolean','INVALID_MESSAGE');
 options={...options,compatibility:testingLoadoutCompatibility(options.compatibility,options.testingStartingLoadout===true)};
 const roomSemantics=declareRoomSemantics(options.compatibility),trustProxy=explicitTrustedProxies(options.trustedProxies);
 if(options.canonicalOrigin)demand(options.tls&&options.canonicalOrigin===options.keys.issuer,'FORBIDDEN');
 const db=extension?extension.database(options):new CandidateDatabase(options.databaseUrl,options.serviceId,options.namespace,options.databaseBudget,true),security=new Security(options.keys,options.serviceId),auth=extension?.auth?extension.auth(db,security,options):new PrincipalAuth(db,security,options.now,options.mockEnabled,options.providerAvailable),rooms=extension?extension.rooms(db,auth,options):new CandidateRooms(db,auth,options.compatibility),signin=new MockSignIn(auth,options.authentication?p=>p==='local'||p==='twitch'&&!!options.authentication?.twitch||p==='google'&&!!options.authentication?.google:undefined),gate=new WorkGate(16);
 auth.auth1Enabled=!!options.authentication;const authentication=options.authentication?new Auth1(auth,signin,options.authentication,options.testingStartingLoadout===true):undefined;rooms.accountEntitlements=authentication?.accounts;
 rooms.testingStartingLoadoutEnabled=options.testingStartingLoadout===true;
 try{await db.migrate();await db.start();}catch(e){await db.close();throw e;}
 const app=Fastify({logger:false,bodyLimit:schema.limits.bytes,requestTimeout:5000,trustProxy,...(options.tls?{https:{...options.tls,minVersion:'TLSv1.2' as const}}:{})});
 app.removeContentTypeParser('application/json');app.addContentTypeParser('application/json',{parseAs:'buffer'},(req,body,done)=>{try{const text=new TextDecoder('utf-8',{fatal:true}).decode(body as Buffer),authType=({'/v1/auth/users/role':'RoleChange','/v1/auth/users/local':'LocalCreate','/v1/auth/users/reset':'LocalReset','/v1/web/logout':'Empty'} as Record<string,string>)[req.url];done(null,authentication&&authType?parseAuth1(authType,text):extension?.parseJson?extension.parseJson(req.url,text):parseJson(req.url==='/v1/mock/browser-login'?'BrowserLogin':'Request',text));}catch(e){done(e as Error);}});
 if(authentication)app.addContentTypeParser('application/x-www-form-urlencoded',{parseAs:'string'},(_req,body,done)=>{try{demand(Buffer.byteLength(body as string)<=4096,'INVALID_MESSAGE');const p=new URLSearchParams(body as string),entries=[...p];demand(entries.length<=6&&new Set(entries.map(([k])=>k)).size===entries.length,'INVALID_MESSAGE');done(null,Object.fromEntries(entries));}catch(e){done(e as Error);}});
 app.addContentTypeParser('application/x-protobuf',{parseAs:'buffer'},(_req,body,done)=>done(null,body));
 app.setErrorHandler((e,_req,reply)=>{const code=e instanceof PolicyError?e.code:'INVALID_MESSAGE';void reply.code(code==='FORBIDDEN'?403:400).send({error:{code}});});
 const traffic=installTrafficBudgets(app,security,auth.now);rooms.roomBudget=(id,tx)=>traffic.authorizedRoom(id,tx);
 app.addHook('onRequest',async(req,reply)=>{
  if(req.url.includes('?')&&!(authentication&&(req.url.startsWith('/auth/')||req.url.startsWith('/v1/auth/')))||(req.headers.origin&&req.headers.origin!==options.keys.issuer)||(options.canonicalOrigin&&req.headers.host!==new URL(options.canonicalOrigin).host)){await reply.code(403).send({error:{code:'FORBIDDEN'}});return;}
 });
 app.get('/schema.json',async(_req,reply)=>reply.header('cache-control','no-store').send(schema));
 if(!authentication)app.get('/v1/auth/info',async(_req,reply)=>reply.code(404).send({error:{code:'NOT_FOUND'}}));
 app.get('/v1/info',async()=>({major:1,revision:schema.revision,connectionFlowRevision:'g3-flow-1',serviceId:db.serviceId,serviceEpoch:db.epoch,compatibility:options.compatibility,capabilities:options.compatibility.capabilities,roomSemantics,...(options.testingStartingLoadout?{developmentFeatures:{testingStartingLoadout:{available:true,settingKey:TESTING_STARTING_LOADOUT.settingKey,default:false}}}:{}),mockProvider:options.mockEnabled&&!authentication,contractStatus:'candidate-not-accepted',profile:extension?.infoProfile??options.profile??{id:'g2-next-simulated',evidenceStatus:'simulated-game-facts'}}));
 app.post('/v1/mock/browser-login',async req=>{
  demand(!authentication&&options.mockEnabled&&process.env.NODE_ENV!=='production','FORBIDDEN');
  const release=gate.enter();try{return await db.transaction(async tx=>{
   const input=validate('BrowserLogin',req.body),subject=input.subject as string;demand(['twitch-local-alex','website-local-alex'].includes(subject),'INVALID_MESSAGE');
   const scope='g2-next-browser:'+String(input.deviceId),fp=fingerprint(input),old=(await tx.query('SELECT * FROM auth_requests WHERE scope=$1 AND request_id=$2',[scope,input.requestId])).rows[0];
   if(old){demand(old.fingerprint===fp&&+old.recovery_until>auth.now(),'CONFLICT');const value=security.open<{credentials:import('../../../../packages/contracts/g2-next/g2-next.js').Credentials}>(old.recovery_ciphertext),session=(await tx.query('SELECT s.revoked_at,f.revoked_at AS family_revoked FROM sessions s JOIN session_families f ON f.id=s.family_id WHERE s.id=$1',[value.credentials.sessionId])).rows[0];demand(session&&!session.revoked_at&&!session.family_revoked,'UNAUTHENTICATED');return {...value,credentialValidity:(await credentialValidity(tx,auth,value.credentials.accessToken!,input.requestId as string)).value};}
   const id=await auth.mockRegistered(tx,subject.startsWith('twitch-')?'twitch':'website','alex','Alex');const value={credentials:await auth.family(tx,id,input.deviceId as string,'browser')};
   await tx.query('INSERT INTO auth_requests(scope,request_id,fingerprint,recovery_ciphertext,recovery_until) VALUES($1,$2,$3,$4,$5)',[scope,input.requestId,fp,security.seal(value),new Date(auth.now()+30000)]);return {...value,credentialValidity:(await credentialValidity(tx,auth,value.credentials.accessToken!,input.requestId as string)).value};
  });}finally{release();}
 });
 app.post('/v1/rpc',async(req,reply)=>{
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf';let input:Request={};const base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});let result:Response;let currentToken:string|undefined;
  try{input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===1&&input.revision===schema.revision,'VERSION_MISMATCH');demand(input.serviceId===db.serviceId,'SERVICE_MISMATCH');demand(/^[A-Za-z0-9_-]{1,128}$/.test(input.requestId??''),'INVALID_MESSAGE');
   const release=gate.enter(),controller=new AbortController(),abort=()=>controller.abort(),deadline=setTimeout(abort,3000);req.raw.once('aborted',abort);reply.raw.once('close',abort);try{if(extension?.requestHousekeeping)await extension.requestHousekeeping(input,rooms);else await rooms.sweep();result=await db.transaction(async tx=>{
    const execute=async():Promise<Response>=>{
    if(input.guestIssue)return {...base(),credentials:await auth.guest(tx,input.guestIssue,input.requestId!)};
    if(input.authBegin)return {...base(),authAttempt:await signin.begin(tx,input.authBegin,input.requestId!,req.ip)};
    if(input.authStatus)return {...base(),authAttempt:await signin.status(tx,input.authStatus)};
    if(input.authCancel)return {...base(),authAttempt:await signin.cancel(tx,input.authCancel)};
    if(input.authExchange)return {...base(),credentials:await signin.exchange(tx,input.authExchange,input.requestId!)};
    if(input.refresh)return {...base(),...await auth.refresh(tx,input)};
    const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');currentToken=token;const actor=await auth.authenticate(tx,token,input);
    if(input.logout){demand(actor.kind!=='game','FORBIDDEN');const cached=(await tx.query('SELECT * FROM command_results WHERE scope=$1 AND request_id=$2',[actor.sessionId,input.requestId])).rows[0];if(cached){demand(cached.fingerprint===fingerprint(input)&&+cached.created_at+30000>auth.now(),'CONFLICT');return base();}demand(input.sessionEpoch===actor.sessionEpoch&&input.serviceEpoch===db.epoch,'STALE_EPOCH');await tx.query('INSERT INTO command_results(scope,request_id,fingerprint,result,created_at) VALUES($1,$2,$3,$4,$5)',[actor.sessionId,input.requestId,fingerprint(input),{sealed:security.seal({})},new Date(auth.now())]);await auth.revoke(tx,actor.familyId);return base();}
    const result=await rooms.execute(tx,input,actor);if(result.room&&actor.kind!=='game'&&result.room.hostAccountId===actor.accountId){const r=(await tx.query('SELECT visibility FROM rooms WHERE id=$1',[result.room.roomId])).rows[0],admitted=(await tx.query('SELECT account_id FROM room_admissions WHERE room_id=$1 ORDER BY account_id LIMIT 33',[result.room.roomId])).rows;demand(admitted.length<=32,'CAPACITY');result.access={visibility:r.visibility,admittedAccountIds:admitted.map(a=>a.account_id)};}return {...base(),...result};
    };let value=await execute();const token=value.credentials?.accessToken??currentToken;
    if(!input.logout&&token){if(extension?.decorate)value={...value,...await extension.decorate(tx,input,value,token)};else value={...value,credentialValidity:(await credentialValidity(tx,auth,token,input.requestId!)).value};}return value;
   },true,controller.signal,
    // Parsed canonical command at this trusted Server call site; no wire
    // priority field or successful scheduling grants gameplay authority.
    input.refresh||input.logout||input.presence||input.gamePulse||input.gameRenew||input.gameObservation||input.gameAttach||input.prepare||input.preparationRenew||input.preparationCancel||input.ready||input.roomJoin||input.roomLeave||input.roomModerate||input.roomAccess||input.roomPolicy||input.configure||input.finalStart||input.cancelRun||input.endRun||input.roomDelete||input.roomCreate?'control':'optional');}finally{clearTimeout(deadline);req.raw.removeListener('aborted',abort);reply.raw.removeListener('close',abort);release();}
  }catch(e){const code=e instanceof PolicyError?e.code:'INVALID_MESSAGE';result={...base(),error:{code,message:code}};}
  try{validate('Response',result);}catch{result={...base(),error:{code:'CAPACITY',message:'CAPACITY'}};}
  reply.header('cache-control','no-store');if(binary)return reply.type('application/x-protobuf').send(Buffer.from(encode('Response',result)));return result;
 });
 if(authentication)await registerAuth1(app,authentication);else await registerMockSignInPage(app,signin,!!options.tls);
 if(options.website)app.get('/packages/contracts/src/lobby-setting-hints.js',async(_req,reply)=>reply.type('text/javascript; charset=utf-8').headers({'cache-control':'no-store','x-content-type-options':'nosniff'}).send(await readFile(new URL('../../../../packages/contracts/src/lobby-setting-hints.js',import.meta.url))));
 if(options.website){for(const [path,file,type] of [['/releases/latest.json','apps/web/releases/latest.json','application/json'],['/packages/contracts/src/release-metadata.js','packages/contracts/src/release-metadata.js','text/javascript']] as const){const body=await readFile(new URL('../../../../'+file,import.meta.url));app.get(path,async(_req,reply)=>reply.type(type+'; charset=utf-8').headers({'cache-control':'no-store','x-content-type-options':'nosniff'}).send(body));}}
 if(options.website){const raceEditor=await readFile(new URL('../../../../packages/contracts/src/race-goal-editor.js',import.meta.url));app.get('/packages/contracts/src/race-goal-editor.js',async(_req,reply)=>reply.type('text/javascript; charset=utf-8').header('cache-control','no-store').send(raceEditor));const raceUi=await readFile(new URL('../../../../packages/contracts/src/race-goal-ui.js',import.meta.url));app.get('/packages/contracts/src/race-goal-ui.js',async(_req,reply)=>reply.type('text/javascript; charset=utf-8').header('cache-control','no-store').send(raceUi));}
 if(options.website){for(const [path,file,type]of [['/','index.html','text/html'],['/download','download.html','text/html'],['/help','help.html','text/html'],['/credits','credits.html','text/html'],['/downloads.js','downloads.js','text/javascript'],['/downloads.css','downloads.css','text/css'],['/app.js','app.js','text/javascript'],['/client.js','client.js','text/javascript'],['/sidebar.js','sidebar.js','text/javascript'],['/statistics-view.js','statistics-view.js','text/javascript'],['/race-view.js','race-view.js','text/javascript'],['/chat-view.js','chat-view.js','text/javascript'],['/mods-view.js','mods-view.js','text/javascript'],['/readiness-view.js','readiness-view.js','text/javascript'],['/style.css','style.css','text/css']] as const){const body=await readFile(new URL('../../../../apps/web/candidate/'+file,import.meta.url));app.get(path,async(_req,reply)=>reply.type(type+'; charset=utf-8').headers({'content-security-policy':"default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",'x-content-type-options':'nosniff','cache-control':'no-store','referrer-policy':'no-referrer'}).send(body));}}
 if(extension)await extension.register(app,{db,auth,rooms,traffic,...(authentication?{webToken:async req=>req.headers.cookie?.includes(authentication.sessionCookie+'=')?(await authentication.web(req)).credentials.accessToken:undefined}:{})});
 if(!app.hasRoute({method:'GET',url:'/v3/c1/info'}))app.get('/v3/c1/info',async()=>({implemented:false,serviceId:db.serviceId,serviceEpoch:db.epoch}));
 if(options.website){const body=await readFile(new URL('../../../../packages/contracts/src/room-semantics.js',import.meta.url));app.get('/room-semantics.js',async(_req,reply)=>reply.type('application/javascript').header('cache-control','no-store').send(body));}
 const expiry=setInterval(()=>{void rooms.sweep().catch(()=>{});},1000);expiry.unref();app.addHook('preClose',async()=>{clearInterval(expiry);await db.close();});return {app,db,auth,rooms,signin,traffic,authentication};
}
