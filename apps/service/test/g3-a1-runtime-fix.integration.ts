import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID,createHash} from 'node:crypto';
import {readFile,access} from 'node:fs/promises';import {spawn} from 'node:child_process';import {fileURLToPath} from 'node:url';
import {verifyNativeA1RuntimeFixDevelopmentArtifacts,nativeA1RuntimeFixResources,nativeA1RuntimeFixSourceReviewResource} from '../src/g3/native-a1-runtime-fix-profile.js';
import {verifyNativeA1DevelopmentArtifacts,nativeA1RuntimeResources,nativeA1SourceReviewResource} from '../src/g3/native-a1-profile.js';
import {validateNativeA1RuntimeFixLanConfig} from '../src/g3/native-a1-runtime-fix-lan.js';
import {validateNativeA1LanConfig} from '../src/g3/native-a1-lan.js';
import {createG3CandidateApp} from '../src/g3/candidate.js';import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
const root=new URL('../../../',import.meta.url),sha=(b:Buffer|string)=>createHash('sha256').update(b).digest('hex');

test('runtime-fix loader independently verifies frozen request/closure and preserves old A1 pins',async()=>{
 for(const social of [false,true]){
  const current=await verifyNativeA1RuntimeFixDevelopmentArtifacts(social),old=await verifyNativeA1DevelopmentArtifacts(social);
  assert.equal(current.id,social?'nt2-g3-native-social-development-3':'nt2-g3-native-development-4');
  assert.equal(current.runtimeApproval.sha256,nativeA1RuntimeFixResources[social?'social':'primary'].sha256);
  assert.deepEqual(current.desktopApproval,nativeA1RuntimeFixSourceReviewResource);assert.equal(current.sourceReview.pins.length,215);
  assert.equal(current.requiredLocalFiles.length,social?20:14);assert.equal(current.serviceId,old.serviceId);assert.equal(current.namespacePrefix,old.namespacePrefix);
  assert.deepEqual(current.compatibility,old.compatibility);assert.deepEqual(current.localProtocol,old.localProtocol);assert.deepEqual(current.profile.supportedBuilds,old.profile.supportedBuilds);
  assert.deepEqual(old.runtimeApproval,nativeA1RuntimeResources[social?'social':'primary']);assert.deepEqual(old.desktopApproval,nativeA1SourceReviewResource);
  assert.equal(current.sourceReview.usableCandidate,false);assert.equal(current.installApproved,false);assert.equal(current.actualNoitaEvidence,false);
  const targets=[current.runtimeApproval.path,current.desktopApproval.path,current.sourceReview.snapshotRequest.path,current.runtimeMappings.find((p:any)=>p.path.endsWith('.dll')).sourcePath,current.runtimeMappings.find((p:any)=>p.path.endsWith('/init.lua')).sourcePath,current.sourceReview.pins.find((p:any)=>p.path.endsWith('/native/g2-bridge/bridge.cpp')).path];
  for(const target of targets){let reached=false;await assert.rejects(verifyNativeA1RuntimeFixDevelopmentArtifacts(social,async url=>{if(url.pathname.endsWith('/'+target)){reached=true;return Buffer.from('changed');}return readFile(url);}),/changed/);assert.equal(reached,true);}
 }
});

test('runtime-fix explicit candidate and fixture selector refuses invalid compositions before DB work',async()=>{
 const m=await verifyNativeA1RuntimeFixDevelopmentArtifacts();
 const base:any={a1Runtime:true,a1RuntimeFix:true,native:true,databaseUrl:'invalid-database-not-reached',namespace:'g3_a1_unreached',serviceId:m.serviceId,mockEnabled:true,compatibility:m.compatibility,keys:{},tls:{pfx:Buffer.alloc(0),passphrase:''}};
 for(const patch of [{a1Runtime:false},{tls:undefined},{native:false},{nativeSocial:true},{namespace:'g3_fixture_wrong'},{serviceId:'other'},{profile:m.profile},{compatibility:{...m.compatibility,capabilities:[...m.compatibility.capabilities,'core.gold']}}])await assert.rejects(createG3CandidateApp({...base,...patch}),/FORBIDDEN/);
 for(const flags of [{a1Runtime:false,native:true,tls:true},{a1Runtime:true,native:true},{a1Runtime:true,tls:true},{a1Runtime:true,tls:true,social:true},{a1Runtime:true,tls:true,native:true,nativeSocial:true}])await assert.rejects(startG3Fixture({a1RuntimeFix:true,databaseUrl:'invalid-database-not-reached',...flags}));
});

test('runtime-fix host exact configs keep4357 namespace and reject old resource/identity substitution',async()=>{
 for(const suffix of ['','-4357']){
  const value=JSON.parse(await readFile(new URL('deploy/g3-native-a1-runtime-fix/config.lan-test'+suffix+'.json',root),'utf8'));
  const c=validateNativeA1RuntimeFixLanConfig(value),old=validateNativeA1LanConfig(JSON.parse(await readFile(new URL('deploy/g3-native-a1/config.lan-test'+suffix+'.json',root),'utf8')));
  for(const key of ['origin','namespace','serviceId','issuer','tlsSourceConfig','databaseSecretConfig'] as const)assert.equal(c[key],old[key]);
  assert.throws(()=>validateNativeA1LanConfig(value));assert.throws(()=>validateNativeA1RuntimeFixLanConfig(old));
  for(const patch of [{a1RuntimeFix:false},{a1Runtime:false},{port:4352},{ip:'127.0.0.1'},{namespace:'g3_social_new'},{serviceId:'other'},{runtimeApprovalSha256:nativeA1RuntimeResources.social.sha256},{sourceReviewSha256:nativeA1SourceReviewResource.sha256},{databaseSecretConfig:'other.json'},{tlsSourceConfig:'other.json'},{capabilities:['core.gold']}])assert.throws(()=>validateNativeA1RuntimeFixLanConfig({...value,...patch}));
 }
});

for(const social of [false,true])test('runtime-fix '+(social?'social':'primary')+' TLS metadata and selected two-peer movement/End',async()=>{
 const f=await startG3Fixture({tls:true,a1Runtime:true,a1RuntimeFix:true,...(social?{nativeSocial:true}:{native:true})});
 try{
  assert.deepEqual(JSON.parse((await f.call('/v1/info')).body).profile,f.profile.buildRegistry);
  if(social){const info=JSON.parse((await f.call('/v3/social/info')).body),runtime=await f.call('/v3/social/runtime');assert.deepEqual(info.runtimeApproval,nativeA1RuntimeFixResources.social);assert.deepEqual(info.desktopApproval,nativeA1RuntimeFixSourceReviewResource);assert.equal(sha(runtime.body),nativeA1RuntimeFixResources.social.sha256);assert.equal(runtime.headers['cache-control'],'no-store');}
  const s=await provisionG3Fixture(f),call=async(i:number,payload:object)=>{
   const c=s.games[i]!,r=await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope:s.scopes[i],...payload},c.accessToken);assert.equal(r.status,200);return JSON.parse(r.body);
  };
  assert.ok((await call(0,{ghostList:{}})).error);await s.start();assert.ok((await call(0,{ghostList:{}})).error);await s.playing();
  for(let i=0;i<2;i++)assert.equal((await call(i,{movement:{frames:[{sequence:1,xMilli:String(1000+i),yMilli:'2000',vxMilli:'0',vyMilli:'0',facingLeft:false}]}})).error,undefined);
  for(let i=0;i<2;i++)assert.equal((await call(i,{ghostList:{}})).ghosts.length,1);
  await s.rpc(s.host,{endRun:{}});assert.ok((await call(0,{ghostList:{}})).error);
 }finally{await f.close();}
});

test('runtime-fix CLI emits exact social bundle and cleans private fixture',async()=>{
 const child=spawn(process.execPath,['--import','tsx','apps/service/scripts/g3-client-fixture.ts','--tls','--playing','--native-social','--a1-runtime','--a1-runtime-fix'],{cwd:fileURLToPath(root),windowsHide:true,stdio:['pipe','pipe','pipe']});
 let output='',timer:ReturnType<typeof setTimeout>|undefined;child.stderr.resume();
 const exited=new Promise<number|null>((resolve,reject)=>{child.once('error',reject);child.once('exit',resolve);});
 try{
  const summary:any=await new Promise((resolve,reject)=>{timer=setTimeout(()=>reject(Error('Fixture summary timeout')),30000);child.stdout.on('data',b=>{output+=b.toString();for(const line of output.split(/\r?\n/)){try{const v=JSON.parse(line);if(v.bundlePath){clearTimeout(timer);resolve(v);}}catch{}}});child.once('exit',code=>reject(Error('Fixture exited before summary '+code)));});
  const b=JSON.parse(await readFile(summary.bundlePath,'utf8')),p=b.nativeSocialDevelopmentProfile;
  assert.equal(b.a1Runtime,true);assert.equal(b.a1RuntimeFix,true);assert.equal(b.evidenceStatus,'simulated-game-facts');assert.equal(b.serviceId,'nt2-g3-native-social-development-a1');assert.equal(p.id,'nt2-g3-native-social-development-3');assert.deepEqual(p.runtimeApproval,nativeA1RuntimeFixResources.social);assert.deepEqual(p.desktopApproval,nativeA1RuntimeFixSourceReviewResource);assert.equal(p.requiredLocalFiles.length,20);assert.deepEqual(b.social.runtimeProfile,p.profile);assert.equal(b.games.length,2);assert.equal(b.scopes.length,2);
  child.stdin.end('quit\n');assert.equal(await exited,0);await assert.rejects(access(summary.bundlePath));await assert.rejects(access(summary.caPath));assert.ok(!output.includes(b.games[0].accessToken));
 }finally{clearTimeout(timer);if(child.exitCode===null){child.stdin.end('quit\n');await exited;}}
});
