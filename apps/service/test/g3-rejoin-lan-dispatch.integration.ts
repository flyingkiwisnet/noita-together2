import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,rm} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {resolve,join} from 'node:path';
import {randomUUID,createHash} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import pg from 'pg';
import {prepareNativeRejoinLanLaunch} from '../src/rejoin/lan.js';
import {lanConfig} from '../src/g2/lan.js';
import {persistentRejoinIdentity} from '../src/rejoin/host-identity.js';
import {createG3CandidateApp} from '../src/g3/candidate-loot01.js';
import {RejoinDatabase} from '../src/rejoin/database.js';
import {privateG3Directory,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';

test('RJ3 real prepared LAN options select final discovery/rejoin factory and retain recovery after restart without binding',async()=>{
 const root=fileURLToPath(new URL('../../../',import.meta.url)),configPath=resolve(root,'deploy/g3-rejoin/config.lan-dev.json');
 const namespace='g3_rejoin_lan_dispatch_'+randomUUID().replaceAll('-','');
 const databaseUrl=process.env.NT2_G3_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
 const directory=await privateG3Directory('nt2-g3-lan-dispatch-'),passphrase='disposable-lan-dispatch-only-32chars';
 let service:Awaited<ReturnType<typeof createG3CandidateApp>>|undefined;
 try{
  const openssl=process.env.NT2_G3_OPENSSL??(process.platform==='win32'?'C:/Program Files/Git/usr/bin/openssl.exe':'openssl');
  const run=async(args:string[])=>{await promisify(execFile)(openssl,args,{cwd:directory,timeout:15000,maxBuffer:65536});};
  await run(['req','-x509','-newkey','rsa:2048','-nodes','-keyout','leaf.key','-out','leaf.pem','-days','1','-subj','/CN=Disposable LAN dispatch']);
  await run(['pkcs12','-export','-inkey','leaf.key','-in','leaf.pem','-out','server.pfx','-passout','pass:'+passphrase]);
  // Only environment inputs are injected: real config/resource verification and
  // returned selector options reach the real factory unchanged. Never live bind.
  const reader=(async(path:any,encoding?:any)=>{
   const name=path instanceof URL?fileURLToPath(path):String(path);
   if(name===resolve(root,'secrets/g2-lan-192-168-0-104-4352/config.json'))return JSON.stringify(lanConfig('192.168.0.104',4352));
   if(name===resolve(root,'secrets/g2-lan-192-168-0-104-4352/tls.json'))return JSON.stringify({passphrase});
   if(name===resolve(root,'secrets/g2-lan-192-168-0-104-4352/server.pfx'))return readFile(join(directory,'server.pfx'));
   if(name===resolve(root,'secrets/g3-native-social-lan/database.json'))return JSON.stringify({databaseUrl});
   if(name===configPath||name.startsWith(resolve(root,'deploy/g2-lan/public')))return encoding?readFile(name,'utf8'):readFile(name);
   throw Error('Unexpected dispatch fixture read');
  }) as typeof readFile;
  const p=await prepareNativeRejoinLanLaunch(configPath,{read:reader,interfaceCheck:()=>{},portCheck:async()=>{},certificateCheck:()=>({} as any)});
  // Sole factory override is the owned disposable namespace, never selectors.
  const options={...p.options,namespace};
  const launch=async()=>createG3CandidateApp({...options,keys:await persistentRejoinIdentity(databaseUrl,options.serviceId,namespace,p.config.issuer)});
  service=await launch();
  assert.equal(Object.hasOwn(p.options,'rejoin'),true);assert.equal(Object.hasOwn(p.options,'rejoinPackage'),true);
  assert.ok(service.db instanceof RejoinDatabase);
  const call=async(path:string,body?:unknown,token?:string,headers:Record<string,string>={})=>{
   const reply=await service!.app.inject({method:body===undefined?'GET':'POST',url:path,headers:{host:new URL(p.config.origin).host,...(token?{authorization:'Bearer '+token}:{}),...headers},...(body===undefined?{}:{payload:body as any})});
   return {status:reply.statusCode,body:reply.body,headers:reply.headers};
  };
  const runtime=await call('/v3/social/runtime');assert.equal(runtime.status,200);
  const digest=(x:string)=>createHash('sha256').update(x).digest('hex');assert.equal(digest(runtime.body),p.metadata.runtimeApproval.sha256);
  const info=JSON.parse((await call('/v3/social/info')).body);assert.deepEqual(info.runtimeApproval,p.metadata.runtimeApproval);
  assert.deepEqual(info.desktopApproval,p.metadata.desktopApproval);assert.equal(info.runtimeProfile.id,'nt2-g3-rejoin-dev-1');
  const fixture={origin:p.config.origin,namespace,profile:p.metadata,rejoinEnabled:true,call,get service(){return service!;}} as Parameters<typeof provisionG3Fixture>[0];
  const s=await provisionG3Fixture(fixture);await s.start();await s.playing();
  const auth=s.desktops[0]!,request={major:1,revision:'g3-rejoin-1',serviceId:options.serviceId,serviceEpoch:service.db.epoch,sessionEpoch:auth.sessionEpoch,requestId:'prepared-before',status:{}};
  const beforeReply=await call('/v3/rejoin/rpc',request,auth.accessToken);assert.equal(beforeReply.status,200);const before=JSON.parse(beforeReply.body);assert.ok(before.resumeProof);
  const oldEpoch=service.db.epoch,keyDigest=digest(JSON.stringify(service.auth.security.keys));
  await service.app.close();service=undefined;service=await launch();
  assert.notEqual(service.db.epoch,oldEpoch);assert.equal(digest(JSON.stringify(service.auth.security.keys)),keyDigest);
  const afterReply=await call('/v3/rejoin/rpc',{...request,serviceEpoch:service.db.epoch,requestId:'prepared-after'},auth.accessToken);assert.equal(afterReply.status,200);
  const after=JSON.parse(afterReply.body);assert.equal(digest(after.resumeProof),digest(before.resumeProof));assert.equal(after.participant.returnEligible,true);assert.equal(after.participant.state,'reconnecting');
  assert.equal(service.app.server.listening,false);
 }finally{
  await service?.app.close();const pool=new pg.Pool({connectionString:databaseUrl,connectionTimeoutMillis:3000,statement_timeout:5000});
  try{assert.match(namespace,/^g3_rejoin_lan_dispatch_[0-9a-f]{32}$/);await pool.query(`DROP SCHEMA IF EXISTS "${namespace}" CASCADE`);}finally{await pool.end();await rm(directory,{recursive:true,force:true});}
 }
});
