import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {verifyNativeA1HeartbeatBusyDevelopmentArtifacts,nativeA1HeartbeatBusyResources,nativeA1HeartbeatBusySourceResource} from '../src/g3/native-a1-heartbeat-busy-profile.js';
import {validateNativeA1HeartbeatBusyLanConfig} from '../src/g3/native-a1-heartbeat-busy-lan.js';
import {nativeA1RuntimeFixResources} from '../src/g3/native-a1-runtime-fix-profile.js';
import {createG3CandidateApp} from '../src/g3/candidate.js';
import {startG3Fixture} from '../scripts/g3-fixture.js';
const root=new URL('../../../',import.meta.url);

test('heartbeat-busy runtime bytes and explicit config/composition remain exact without DB work',async()=>{
 for(const social of [false,true]){
  const p=await verifyNativeA1HeartbeatBusyDevelopmentArtifacts(social);
  assert.equal(p.id,social?'nt2-g3-native-social-development-5':'nt2-g3-native-development-6');
  assert.equal(p.requiredLocalFiles.length,social?20:14);
  assert.deepEqual(p.runtimeApproval,nativeA1HeartbeatBusyResources[social?'social':'primary']);
  assert.deepEqual(p.desktopApproval,nativeA1HeartbeatBusySourceResource);
  assert.equal(p.sourceReview.finalRebuiltDesktopReviewRequired,false);
  const dll=p.runtimeMappings.find((m:any)=>m.path.endsWith('.dll'));
  await assert.rejects(verifyNativeA1HeartbeatBusyDevelopmentArtifacts(social,async url=>url.pathname.endsWith('/'+dll.sourcePath)?Buffer.from('mismatch'):readFile(url)),/dependency changed/);
  const options:any={a1Runtime:true,a1HeartbeatBusy:true,native:!social,nativeSocial:social,tls:{pfx:Buffer.alloc(0)},databaseUrl:'not-reached',namespace:p.namespacePrefix+'not_reached',serviceId:p.serviceId,mockEnabled:true,keys:{},compatibility:p.compatibility};
  for(const patch of [{a1Runtime:false},{a1RuntimeFix:true},{a1RuntimeDiagnostic:true},{tls:undefined},{serviceId:'wrong'},{namespace:'g3_wrong'},{compatibility:{...p.compatibility,capabilities:[...p.compatibility.capabilities,'core.gold']}}])await assert.rejects(createG3CandidateApp({...options,...patch}),/FORBIDDEN/);
 }
 for(const suffix of ['','-4357']){
  const config=JSON.parse(await readFile(new URL('deploy/g3-native-a1-heartbeat-busy/config.lan-test'+suffix+'.json',root),'utf8'));
  const c=validateNativeA1HeartbeatBusyLanConfig(config);
  assert.equal(c.namespace,'g3_social_a1_lan_192_168_0_104_'+c.port);
  assert.equal(c.serviceId,'nt2-g3-native-social-development-a1');
  for(const patch of [{runtimeApprovalSha256:nativeA1RuntimeFixResources.social.sha256},{a1HeartbeatBusy:false},{a1RuntimeFix:true},{namespace:'other'},{databaseSecretConfig:'other'},{tlsSourceConfig:'other'}])assert.throws(()=>validateNativeA1HeartbeatBusyLanConfig({...config,...patch}));
 }
 await assert.rejects(startG3Fixture({a1HeartbeatBusy:true,tls:true,nativeSocial:true,databaseUrl:'not-reached'}),/requires/);
 await assert.rejects(startG3Fixture({a1HeartbeatBusy:true,a1RuntimeFix:true,a1Runtime:true,tls:true,nativeSocial:true,databaseUrl:'not-reached'}),/excludes/);
 await assert.rejects(startG3Fixture({a1HeartbeatBusy:true,a1RuntimeDiagnostic:true,a1Runtime:true,tls:true,nativeSocial:true,databaseUrl:'not-reached'}),/excludes/);
});
