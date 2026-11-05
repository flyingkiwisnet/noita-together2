import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
import {verifyNativeC2DevelopmentArtifacts,nativeC2RuntimeResource,nativeC2SourceResource} from '../src/g3/native-c2-profile.js';
import {validateNativeC2LanConfig} from '../src/g3/native-c2-lan.js';
import {createG3CandidateApp} from '../src/g3/candidate.js';import {startG3Fixture} from '../scripts/g3-fixture.js';
const root=new URL('../../../',import.meta.url);

test('C3 rematch preserves record1 and unchanged semantics; old entry and old host configs cannot satisfy record2',async()=>{
 const p=await verifyNativeC2DevelopmentArtifacts(),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
 const oldBytes=await readFile(new URL('deploy/profiles/g3-native-c2-runtime-approval.record1.json',root)),old=JSON.parse(oldBytes.toString());
 assert.equal(sha(oldBytes),'ae6dbf9c2f9ead92cf9cd71310f03aa1e652406cb42b31e5acc2795a9062ff64');
 assert.equal(sha(await readFile(new URL('deploy/profiles/g3-native-c2-source-dependency.record1.json',root))),'03d2ce203e0e1974580d2a059207efcc34ba37dbe7f08581310e76cf55ab11c7');
 assert.equal(p.id,'nt2-g3-native-c2-development-2');assert.deepEqual(p.compatibility,old.compatibility);assert.deepEqual(p.nativeArtifacts,old.nativeArtifacts);assert.deepEqual(p.localProtocol,old.localProtocol);assert.deepEqual(p.live,old.live);
 const entry='mods/noita_together2/init.lua';assert.deepEqual(p.runtimeMappings.filter((m:any)=>m.path!==entry),old.runtimeMappings.filter((m:any)=>m.path!==entry));
 const oldEntry=await readFile(new URL('apps/desktop/archive/init-g3-c2-before-C3-lifetime-20261010.lua',root));
 assert.equal(sha(oldEntry),old.profile.modSha256);assert.notEqual(p.profile.modSha256,old.profile.modSha256);
 await assert.rejects(verifyNativeC2DevelopmentArtifacts(url=>url.pathname.endsWith('/mod/noita_together2/init-g3-c2.lua')?Promise.resolve(oldEntry):readFile(url)),/dependency changed/);
 for(const suffix of ['','-4357']){
  const config=JSON.parse(await readFile(new URL('deploy/g3-native-c2-live/config.lan-test'+suffix+'.json',root),'utf8'));
  assert.throws(()=>validateNativeC2LanConfig(config));
 }
});

test('C2 complete production closure and every source/runtime/metadata drift refuse before DB',async()=>{
 const p=await verifyNativeC2DevelopmentArtifacts();assert.equal(p.requiredLocalFiles.length,22);assert.equal(p.sourceReview.pins.length,27);
 assert.deepEqual(p.nativeArtifacts.map((a:any)=>[a.mode,a.exports.length]),[['broker',1],['primary',13],['social',16]]);
 assert.equal(p.profile.bridgeSha256,p.nativeArtifacts.find((a:any)=>a.mode==='primary').sha256);
 assert.deepEqual(p.runtimeApproval,nativeC2RuntimeResource);assert.deepEqual(p.desktopApproval,nativeC2SourceResource);
 const targets=[nativeC2RuntimeResource,nativeC2SourceResource,p.sourceReview.snapshotRequest,p.sourceReview.originalBuildRegistry,...p.sourceReview.pins];
 for(const target of targets){let reached=false;await assert.rejects(startG3Fixture({c2Runtime:true,live:true,tls:true,databaseUrl:'not-reached',artifactReader:async url=>{
  const bytes=await readFile(url);if(url.pathname.endsWith('/'+target.path)){reached=true;const changed=Buffer.from(bytes);changed[0]=changed[0]!^1;return changed;}return bytes;
 }}),/dependency changed/);assert.equal(reached,true,target.path);}
});

test('C2 live/TLS/profile/host capability selectors cannot bypass canonical metadata',async()=>{
 const p=await verifyNativeC2DevelopmentArtifacts(),base:any={c2Runtime:true,live:true,tls:{pfx:Buffer.alloc(0)},databaseUrl:'not-reached',namespace:'g3_c2_not_reached',serviceId:p.serviceId,mockEnabled:true,keys:{},compatibility:p.compatibility};
 for(const patch of [{live:false},{tls:undefined},{native:true},{nativeSocial:true},{social:true},{hourglass:true},{nemesis:true},{a1Runtime:true},{a1RuntimeFix:true},{a1RuntimeDiagnostic:true},{a1HeartbeatBusy:true},{serviceId:'wrong'},{namespace:'g3_social_wrong'},{profile:p.profile},{compatibility:{...p.compatibility,bridgeVersion:'g3-a1-bridge-1'}},{compatibility:{...p.compatibility,capabilities:[...p.compatibility.capabilities,'core.gold']}}])await assert.rejects(createG3CandidateApp({...base,...patch}),/FORBIDDEN/);
 for(const patch of [{live:false},{tls:false},{native:true},{nativeSocial:true},{social:true},{hourglass:true},{nemesis:true},{a1Runtime:true},{a1RuntimeFix:true},{a1RuntimeDiagnostic:true},{a1HeartbeatBusy:true}])await assert.rejects(startG3Fixture({c2Runtime:true,live:true,tls:true,databaseUrl:'not-reached',...patch}));
 for(const suffix of ['','-4357']){
  const config=JSON.parse(await readFile(new URL('deploy/g3-native-c2-live/config.lan-test'+suffix+'-record2.json',root),'utf8')),c=validateNativeC2LanConfig(config);
  assert.equal(c.namespace,'g3_c2_lan_192_168_0_104_'+c.port);assert.equal(c.serviceId,p.serviceId);assert.equal(c.live,true);
  for(const patch of [{live:false},{c2Runtime:false},{a1Runtime:true},{serviceId:'old'},{namespace:'g3_social_old'},{issuer:'https://localhost:4356'},{origin:'http://192.168.0.104:4356'},{ip:'127.0.0.1'},{port:4352},{runtimeApprovalSha256:'a'.repeat(64)},{sourceReviewSha256:'b'.repeat(64)},{tlsSourceConfig:'other'},{databaseSecretConfig:'other'},{compatibility:{capabilities:['core.gold']}},{privateKey:'caller'}])assert.throws(()=>validateNativeC2LanConfig({...config,...patch}));
 }
});
