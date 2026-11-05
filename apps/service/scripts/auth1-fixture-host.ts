// Client-owned component driver entry point. Private files carry fixture-only
// local passwords; stdout contains readiness/operation metadata, never secrets.
import {createInterface} from 'node:readline';import {writeFile} from 'node:fs/promises';import {join} from 'node:path';import {randomUUID} from 'node:crypto';
import {startAuth1Fixture} from './auth1-fixture.js';import {AUTH1_ROLES} from '../../../packages/domain/src/auth1.js';
const f=await startAuth1Fixture(),descriptor=join(f.directory,'ready.json');let closed=false;
const output=(v:unknown)=>process.stdout.write(JSON.stringify(v)+'\n');
await writeFile(descriptor,JSON.stringify({revision:'auth1-fixture-1',origin:f.origin,caPath:f.caPath,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,namespace:f.namespace,providerFacts:'synthetic HTTP/JWKS providers; real service validation',browserTraversal:'GET launchUrl, follow provider302 then service callback302/HTML; parse form POST to initiating loopback callback; cookies required',localLogin:'/auth/login/local or local desktop launchUrl; POST /auth/local; forced replacement POST /auth/password',noRealProviders:true}),{mode:0o600,flag:'wx'});
output({ready:true,descriptor});const lines=createInterface({input:process.stdin,crlfDelay:Infinity});
const close=async()=>{if(closed)return;closed=true;lines.close();await f.close();};
try{for await(const line of lines){let request:any;try{
 request=JSON.parse(line);if(!request||typeof request!=='object'||typeof request.requestId!=='string'||!/^[A-Za-z0-9_-]{1,80}$/.test(request.requestId))throw Error('Invalid request');const accounts=f.service.authentication!.accounts;let result:unknown;
 if(request.op==='stop'){output({requestId:request.requestId,ok:true,stopping:true});break;}
 if(request.op==='local-create'){if(Object.keys(request).some(k=>!['requestId','op','username','displayName'].includes(k)))throw Error('Invalid fields');result=await accounts.issueLocal(undefined,{username:request.username,displayName:request.displayName,operationId:randomUUID()},'disposable-fixture-driver');}
 else if(['role','reset-local','logout'].includes(request.op)){
  if(!/^[a-f0-9-]{36}$/.test(request.accountId??''))throw Error('Immutable fixture account ID required');const target=await f.service.db.transaction(tx=>accounts.role(tx,request.accountId));
  if(request.op==='role'){if(!AUTH1_ROLES.includes(request.role))throw Error('Invalid role');result=await f.service.db.transaction(tx=>accounts.setRole(tx,undefined,{accountId:target.id,role:request.role,revision:target.role_revision,operationId:randomUUID()},'disposable-fixture-driver'));}
  if(request.op==='reset-local')result=await accounts.issueLocal(undefined,{accountId:target.id,operationId:randomUUID()},'disposable-fixture-driver');
  if(request.op==='logout'){await f.service.db.transaction(tx=>accounts.revokeAccount(tx,target.id));result={revoked:true,accountId:target.id};}
 }
 else if(request.op==='advance'){if(!Number.isInteger(request.ms)||request.ms<0||request.ms>86400000)throw Error('Bounded fixture duration required');f.advance(request.ms);result={advanced:true};}
 else if(request.op==='provider-fault'){if(!['google','twitch'].includes(request.provider)||!['outage','audience','issuer','nonce','expired'].includes(request.fault))throw Error('Invalid provider fault');f.fault(request.provider,request.fault);result={armed:true};}
 else if(request.op==='provider-clear'){f.clearFault();result={cleared:true};}
 else if(request.op==='create-lobby'){
  const principal=await f.service.db.transaction(tx=>accounts.provision(tx,'google','fixture-lobby-owner','Fixture host')),credentials=await f.service.db.transaction(async tx=>{const c=await f.service.auth.family(tx,principal,randomUUID(),'browser');const family=(await tx.query('SELECT family_id FROM sessions WHERE id=$1',[c.sessionId])).rows[0];await accounts.markSignIn(tx,principal,family.family_id);return c;}),projected=await f.service.db.transaction(tx=>accounts.current(tx,{accountId:principal,kind:'browser'} as any));
  const r=await f.service.app.inject({method:'POST',url:'/v1/rpc',headers:{host:new URL(f.origin).host,authorization:'Bearer '+credentials.accessToken},payload:{major:1,revision:'g2-draft-5',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:credentials.sessionEpoch,requestId:randomUUID(),roomCreate:{name:projected.entitlements.generatedLobbyName,config:{seed:424242,seedPolicy:'shared-fixed',randomizeLoot:true,mode:'core',requiredCapabilities:f.profile.compatibility.capabilities},allowGuests:false,visibility:'public'}}});result=JSON.parse(r.body);if((result as any).error)throw Error('Fixture lobby refused');
 }
 else throw Error('Unknown fixture action');
 const responsePath=join(f.directory,'response-'+request.requestId+'.json');await writeFile(responsePath,JSON.stringify(result),{mode:0o600,flag:'wx'});output({requestId:request.requestId,ok:true,responsePath});
 }catch{output({requestId:typeof request?.requestId==='string'?request.requestId:'invalid',ok:false,error:'fixture-action-refused'});}}}finally{await close();}
