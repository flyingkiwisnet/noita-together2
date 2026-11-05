import test from 'node:test';import assert from 'node:assert/strict';import {readFile,mkdtemp,writeFile,rm} from 'node:fs/promises';import {tmpdir} from 'node:os';import {join} from 'node:path';
import {nativeA1LanConfig,validateNativeA1LanConfig,prepareNativeA1LanLaunch} from '../src/g3/native-a1-lan.js';import {startG3Fixture} from '../scripts/g3-fixture.js';
test('A1 manual host exact config rejects old identity/profile/capability/TLS namespace substitutions before DB work',async()=>{
 const root=new URL('../../../',import.meta.url),value=JSON.parse(await readFile(new URL('deploy/g3-native-a1/config.lan-test.json',root),'utf8'));
 const config=validateNativeA1LanConfig(value);assert.equal(config.origin,'https://192.168.0.104:4356');assert.equal(config.namespace,'g3_social_a1_lan_192_168_0_104_4356');assert.equal(config.serviceId,'nt2-g3-native-social-development-a1');assert.equal(nativeA1LanConfig('192.168.0.104',4357).port,4357);
 const alternate=validateNativeA1LanConfig(JSON.parse(await readFile(new URL('deploy/g3-native-a1/config.lan-test-4357.json',root),'utf8')));assert.notEqual(alternate.namespace,config.namespace);
 const folder=await mkdtemp(join(tmpdir(),'nt2-a1-config-test-'));try{
  for(const patch of [{serviceId:'nt2-g3-native-social-development'},{namespace:'g3_social_lan_192_168_0_104_4356_cp13'},{origin:'http://192.168.0.104:4356'},{issuer:'https://localhost:4356'},{port:4352},{ip:'127.0.0.1'},{a1Runtime:false},{runtimeApprovalSha256:'f1e4c251d0feb5caea64f645f8b60f74040744857831a89414202c8d17d1357e'},{sourceReviewSha256:'a'.repeat(64)},{tlsSourceConfig:'unreviewed.json'},{databaseSecretConfig:'arbitrary.json'},{compatibility:{capabilities:['core.gold']}},{privateKey:'injected'}]){
   const v={...value,...patch};assert.throws(()=>validateNativeA1LanConfig(v));const file=join(folder,'config.json');await writeFile(file,JSON.stringify(v));await assert.rejects(prepareNativeA1LanLaunch(file));
  }
 }finally{await rm(folder,{recursive:true,force:true});}
 const script=await readFile(new URL('apps/service/scripts/g3-native-a1-lan.ts',root),'utf8');assert.match(script,/if\(check\)console.log/);assert.match(script,/serviceStarted:false,databaseTouched:false/);assert.match(script,/--check-only/);
});
test('A1 fixture selector refuses missing TLS/non-native/mixed capability profiles before DB work',async()=>{
 for(const flags of [{native:true},{tls:true},{tls:true,social:true},{tls:true,native:true,nativeSocial:true},{tls:true,nativeSocial:true,nemesis:true},{tls:true,nativeSocial:true,social:true}])await assert.rejects(startG3Fixture({a1Runtime:true,databaseUrl:'invalid-database-not-reached',...flags}));
});
