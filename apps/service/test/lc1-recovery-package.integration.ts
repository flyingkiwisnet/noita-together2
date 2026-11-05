// Focused matching-artifact smoke; actual frozen Node entry and browser correction.
import test from 'node:test';import assert from 'node:assert/strict';import {spawn} from 'node:child_process';import {join,dirname} from 'node:path';
import {startG3Fixture} from '../scripts/g3-loot01-fixture.js';
test('recovery normal source and frozen host agree on profile, canonical routes, browser policy and existing116 schema',async()=>{
 const databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',f=await startG3Fixture({databaseUrl,rejoin:true,rejoinPackage:true,loot01Runtime:true,live:true,tls:true});
 try{
  assert.match(f.namespace,/^g3_rejoin_[a-f0-9]{32}$/);assert.equal(f.profile.id,'nt2-g3-lc1-recovery-dev-20261020');assert.equal(f.profile.runtimeMappings.length,50);for(const image of f.profile.nativeArtifacts){assert.equal(image.private,false);assert.equal(image.testFlags,false);}
  assert.equal(JSON.parse((await f.call('/v3/lifecycle/info')).body).revision,'g3-lifecycle-lc1-2');const info=JSON.parse((await f.call('/v3/social/info')).body);assert.deepEqual(info.runtimeApproval,f.profile.runtimeApproval);assert.equal(info.runtimeProfile.id,f.profile.id);
  const migrationVersions=await f.service.db.transaction(async tx=>(await tx.query('SELECT version FROM schema_migrations ORDER BY version')).rows.map(r=>Number(r.version)));assert.equal(Math.max(...migrationVersions),116);const keys=f.service.auth.security.keys;
  await f.service.app.close();const child=spawn(process.execPath,['apps/service/scripts/verify-lc1-recovery-frozen-service.mjs'],{windowsHide:true,stdio:['pipe','pipe','pipe']});let output='',errors='';child.stdout.on('data',b=>output+=b);child.stderr.on('data',b=>errors+=b);const exit=new Promise<number|null>((resolve,reject)=>{child.once('close',resolve);child.once('error',reject);});
  child.stdin.end(JSON.stringify({databaseUrl,namespace:f.namespace,keys,compatibility:f.profile.compatibility,origin:f.origin,pfxPath:join(dirname(f.caPath!),'server.pfx'),caPath:f.caPath,runtimeApproval:f.profile.runtimeApproval,migrationVersions}));assert.equal(await exit,0,errors);const result=JSON.parse(output.trim());assert.equal(result.abandonedBrowserReclaimed,true);assert.equal(result.migrationVersionsUnchanged,true);console.log(JSON.stringify(result));
 }finally{await f.close();}
});
