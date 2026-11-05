import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs/promises';import os from 'node:os';import path from 'node:path';import {pathToFileURL} from 'node:url';import {randomUUID,createHash} from 'node:crypto';import {execFile} from 'node:child_process';import {promisify} from 'node:util';import pg from 'pg';
const run=promisify(execFile),candidate=path.resolve(process.env.NT2_HOST_PACKAGE??'.build/host-package-ready/host');
test('Packaged normal host outside checkout migrates fresh isolated DB, serves browser login/RPC and retains signer on restart',async()=>{
 const scratch=await fs.mkdtemp(path.join(os.tmpdir(),'nt2-host-smoke-')),host=path.join(scratch,'host'),namespace='g3_rejoin_'+randomUUID().replaceAll('-',''),databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';let service;
 console.log(JSON.stringify({allocatedFixtureSchema:namespace,databaseTarget:'127.0.0.1:54328/nt2_g1',engineFacts:'none; website/service smoke only'}));
 try{
  await fs.cp(candidate,host,{recursive:true});
  const openssl=process.env.NT2_G3_OPENSSL??(process.platform==='win32'?'C:/Program Files/Git/usr/bin/openssl.exe':'openssl'),passphrase='disposable-package-smoke-only-32chars';
  await run(openssl,['req','-x509','-newkey','rsa:2048','-nodes','-keyout','leaf.key','-out','leaf.pem','-days','1','-subj','/CN=Disposable host package'],{cwd:scratch,windowsHide:true,maxBuffer:65536});
  await run(openssl,['pkcs12','-export','-inkey','leaf.key','-in','leaf.pem','-out','server.pfx','-passout','pass:'+passphrase],{cwd:scratch,windowsHide:true,maxBuffer:65536});
  const load=p=>import(pathToFileURL(path.join(host,p)).href),{createG3CandidateApp}=await load('apps/service/src/g3/candidate-loot01.js'),{verifyRejoinPackage}=await load('apps/service/src/rejoin/package-profile.js'),{persistentRejoinIdentity}=await load('apps/service/src/rejoin/host-identity.js'),{HostClient}=await load('apps/web/candidate/client.js');
  const metadata=await verifyRejoinPackage(),origin='https://127.0.0.1:4358',tls={pfx:await fs.readFile(path.join(scratch,'server.pfx')),passphrase};
  const launch=async()=>createG3CandidateApp({databaseUrl,namespace,serviceId:metadata.serviceId,mockEnabled:true,website:true,rejoin:true,rejoinPackage:true,rejoinPackageVerifier:verifyRejoinPackage,loot01Runtime:true,live:true,debugDiagnostics:false,testingStartingLoadout:true,canonicalOrigin:origin,tls,compatibility:metadata.compatibility,keys:await persistentRejoinIdentity(databaseUrl,metadata.serviceId,namespace,origin)});
  service=await launch();const epoch=service.db.epoch,keys=createHash('sha256').update(JSON.stringify(service.auth.security.keys)).digest('hex');
  const transport=async(url,payload,token)=>{const r=await service.app.inject({method:payload===undefined?'GET':'POST',url,headers:{host:'127.0.0.1:4358',...(token?{authorization:'Bearer '+token}:{})},...(payload===undefined?{}:{payload})});assert.equal(r.statusCode,200,r.body);return JSON.parse(r.body);};
  const client=new HostClient(transport,()=>randomUUID()),info=await client.initialize();assert.equal(info.developmentFeatures.testingStartingLoadout.default,false);assert.equal(info.developmentFeatures.testingStartingLoadout.available,true);
  await client.login('website-local-alex');assert.equal(client.credentials.kind,'browser');
  const created=await client.rpc({roomCreate:{name:'Packaged host smoke',config:{seed:424242,seedPolicy:'shared-fixed',randomizeLoot:true,mode:'core',requiredCapabilities:metadata.compatibility.capabilities},allowGuests:false,visibility:'public'}});assert.ok(created.room.roomId);
  for(const url of ['/','/app.js','/client.js','/style.css','/statistics','/g3/page.js','/g3/client.js','/g3/statistics.js','/room-semantics.js']){const r=await service.app.inject({url,headers:{host:'127.0.0.1:4358'}});assert.equal(r.statusCode,200,url);assert.ok(r.body.length>0);}
  await service.app.close();service=undefined;service=await launch();assert.notEqual(service.db.epoch,epoch);assert.equal(createHash('sha256').update(JSON.stringify(service.auth.security.keys)).digest('hex'),keys);assert.equal(service.app.server.listening,false);
  await client.initialize();await client.login('website-local-alex');const listed=await client.rpc({roomList:{}});assert.ok(listed.rooms.some(r=>r.roomId===created.room.roomId));client.dispose();
  console.log(JSON.stringify({packagedFactory:true,closureOutsideCheckout:true,actualDatabase:true,websiteDiscoveryLoginRpc:true,websiteAssets:9,signerRetained:true,listenerStarted:false}));
 }finally{
  await service?.app.close();const pool=new pg.Pool({connectionString:databaseUrl,connectionTimeoutMillis:3000,statement_timeout:5000});try{assert.match(namespace,/^g3_rejoin_[a-f0-9]{32}$/);await pool.query('DROP SCHEMA IF EXISTS "'+namespace+'" CASCADE');assert.equal((await pool.query('SELECT to_regnamespace($1) AS schema',[namespace])).rows[0].schema,null);console.log(JSON.stringify({verifiedAbsentSchema:namespace}));}finally{await pool.end();}
  assert.ok(path.resolve(scratch).startsWith(path.resolve(os.tmpdir())+path.sep+'nt2-host-smoke-'));await fs.rm(scratch,{recursive:true,force:true});
 }
});
