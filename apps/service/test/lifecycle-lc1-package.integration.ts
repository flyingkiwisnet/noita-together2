// Package cutover smoke, not another complete Client/native composition.
import test from 'node:test';import assert from 'node:assert/strict';import {join,dirname} from 'node:path';import {randomUUID} from 'node:crypto';import {spawn} from 'node:child_process';
import {startG3Fixture} from '../scripts/g3-loot01-fixture.js';
const databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
const frozen=new URL('../../../deploy/g3-rejoin/lc1-host-runtime-20261018/',import.meta.url);
test('normal source and frozen LC1 package select matching profile, actual lifecycle routes and additive116 migration',async()=>{
 const f=await startG3Fixture({databaseUrl,rejoin:true,rejoinPackage:true,loot01Runtime:true,live:true,tls:true});
 try{
  assert.match(f.namespace,/^g3_rejoin_[a-f0-9]{32}$/);assert.equal(f.profile.id,'nt2-g3-lc1-dev-20261018');assert.equal(f.profile.runtimeMappings.length,50);assert.equal(f.profile.fixtureOnly,undefined);
  for(const a of f.profile.nativeArtifacts){assert.equal(a.private,false);assert.equal(a.testFlags,false);assert.ok(!a.exports.includes('nt2_g3_private_api'));}
  const check=async(label:string)=>{
   const info=JSON.parse((await f.call('/v3/lifecycle/info')).body);assert.equal(info.revision,'g3-lifecycle-lc1-2');assert.equal(info.endpoint,'/v3/lifecycle/rpc');
   const schema=JSON.parse((await f.call('/v3/lifecycle/schema')).body);assert.equal(schema.revision,info.revision);
   const social=JSON.parse((await f.call('/v3/social/info')).body);assert.equal(social.runtimeProfile.id,f.profile.id);assert.deepEqual(social.runtimeApproval,f.profile.runtimeApproval);assert.equal(social.sourceApproved,true);
   const invalid=JSON.parse((await f.call('/v3/lifecycle/rpc',{major:1,revision:info.revision,serviceId:info.serviceId,serviceEpoch:info.serviceEpoch,sessionEpoch:randomUUID(),requestId:'package-route-probe',prepare:{worldId:randomUUID(),roomId:randomUUID(),proof:'X'.repeat(43),configRevision:1,configDigest:'0'.repeat(64)}})).body);assert.equal(invalid.error?.code,'UNAUTHENTICATED');assert.equal(invalid.requestId,'package-route-probe');
   assert.equal((await f.call('/v3/waiting/rpc')).status,404);
   console.log(JSON.stringify({packageCutover:label,schema:f.namespace,normalRuntime:f.profile.id,lifecycleRevision:info.revision,canonicalRpcReached:true,competingWaitingRouteAbsent:true,matchingSocialProfile:true}));
  };
  await check('source-normal-package');const keys=f.service.auth.security.keys;
  await f.service.app.close();
  const {RejoinDatabase}=await import(new URL('apps/service/src/rejoin/database.js',frozen).href);
  const db=new RejoinDatabase(databaseUrl,f.profile.serviceId,f.namespace,{},true);
  try{
   await db.pool.query('INSERT INTO rejoin_host_identity(service_id,issuer,keys) VALUES($1,$2,$3)',[f.profile.serviceId,f.origin,keys]);
   const signer=(await db.pool.query('SELECT service_id,issuer,keys FROM rejoin_host_identity')).rows;
   const original=(await db.pool.query('SELECT service_id,epoch FROM service_identity')).rows;
   const tx=await db.pool.connect();try{await tx.query('BEGIN');try{await tx.query('DROP TABLE world_associations,world_creation_intents,world_lineages');await tx.query('DELETE FROM schema_migrations WHERE version=116');await tx.query('COMMIT');}catch(e){await tx.query('ROLLBACK');throw e;}}finally{tx.release();}
   await db.migrate();await db.migrate();assert.deepEqual((await db.pool.query('SELECT service_id,epoch FROM service_identity')).rows,original);
   assert.deepEqual((await db.pool.query('SELECT service_id,issuer,keys FROM rejoin_host_identity')).rows,signer);
   for(const name of ['world_lineages','world_creation_intents','world_associations'])assert.ok((await db.pool.query('SELECT to_regclass($1) AS relation',[name])).rows[0].relation);
   assert.equal((await db.pool.query('SELECT count(*)::int AS count FROM schema_migrations WHERE version=116')).rows[0].count,1);
   console.log(JSON.stringify({packageMigration116:true,schema:f.namespace,pre116RecreatedForTest:true,existingIdentityPreserved:true,persistentSignerPreserved:true,idempotent:true,serviceStarted:false}));
  }finally{await db.close();}
  const child=spawn(process.execPath,['apps/service/scripts/verify-lc1-frozen-service.mjs'],{windowsHide:true,stdio:['pipe','pipe','pipe']});let output='',errors='';child.stdout.on('data',b=>output+=b);child.stderr.on('data',b=>errors+=b);
  const exited=new Promise<number|null>((resolve,reject)=>{child.once('close',resolve);child.once('error',reject);});
  child.stdin.end(JSON.stringify({databaseUrl,namespace:f.namespace,keys,compatibility:f.profile.compatibility,origin:f.origin,pfxPath:join(dirname(f.caPath!),'server.pfx'),caPath:f.caPath,runtimeApproval:f.profile.runtimeApproval}));
  assert.equal(await exited,0,errors);const result=JSON.parse(output.trim());assert.equal(result.packageCutover,'frozen-normal-package-plain-node');console.log(JSON.stringify(result));
 }finally{await f.close();}
});
