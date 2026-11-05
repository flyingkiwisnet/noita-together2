import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
import {verifyNativePotionCorrectionArtifacts,nativePotionCorrectionRuntimeResource,nativePotionCorrectionSourceResource} from '../src/g3/native-potion-correction-profile.js';
import {verifyNativeC2DevelopmentArtifacts} from '../src/g3/native-c2-profile.js';
import {validateNativePotionCorrectionLanConfig} from '../src/g3/native-potion-correction-lan.js';
import {createG3CandidateApp} from '../src/g3/candidate.js';import {startG3Fixture} from '../scripts/g3-fixture.js';
const root=new URL('../../../',import.meta.url),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
test('separate potion resource checks complete closure and refuses baseline/changed source before DB; regular record2 remains exact',async()=>{
 const p=await verifyNativePotionCorrectionArtifacts(),base=await verifyNativeC2DevelopmentArtifacts();
 assert.equal(base.id,'nt2-g3-native-c2-development-2');assert.equal(p.id,'nt2-g3-c2-potion-correction-1');assert.equal(p.requiredLocalFiles.length,22);assert.equal(p.sourceReview.pins.length,31);
 assert.deepEqual(p.compatibility,base.compatibility);assert.deepEqual(p.nativeArtifacts,base.nativeArtifacts);assert.deepEqual(p.localProtocol,base.localProtocol);assert.deepEqual(p.profile.supportedBuilds,base.profile.supportedBuilds);
 assert.deepEqual(p.runtimeMappings.slice(1),base.runtimeMappings.slice(1));assert.equal(p.diagnosticOnly,false);assert.equal(p.gameplaySemanticsChanged,true);assert.equal(p.observer.scope,'unchanged observer block only');assert.equal(p.observer.getterOnly,true);assert.equal(p.observer.observationalOnly,true);assert.equal(p.correction.actualEngineFixProven,false);assert.equal(p.correction.reviewCorrection.id,'POTION-C01');assert.equal(p.correction.freshRoomRequired,true);
 const oldDiagnosticEntry=await readFile(new URL('mod/noita_together2/init-g3-c2-potion-diagnostic.lua',root));
 await assert.rejects(verifyNativePotionCorrectionArtifacts(url=>url.pathname.endsWith('/'+p.runtimeMappings[0].sourcePath)?Promise.resolve(oldDiagnosticEntry):readFile(url)),/dependency changed/);
 const baselineEntry=await readFile(new URL(base.runtimeMappings[0].sourcePath,root));
 await assert.rejects(verifyNativePotionCorrectionArtifacts(url=>url.pathname.endsWith('/'+p.runtimeMappings[0].sourcePath)?Promise.resolve(baselineEntry):readFile(url)),/dependency changed/);
 for(const target of [nativePotionCorrectionRuntimeResource,nativePotionCorrectionSourceResource,p.sourceReview.snapshotRequest,p.sourceReview.originalBuildRegistry,p.sourceReview.baselineRuntimeApproval,p.sourceReview.baselineDependency,p.sourceReview.baselineDiagnosticRuntimeApproval,p.sourceReview.baselineDiagnosticDependency,...p.sourceReview.pins]){
  let reached=false;await assert.rejects(startG3Fixture({potionCorrection:true,live:true,tls:true,databaseUrl:'not-reached',artifactReader:async url=>{const b=await readFile(url);if(url.pathname.endsWith('/'+target.path)){reached=true;const changed=Buffer.from(b);changed[0]=changed[0]!^1;return changed;}return b;}}),/dependency changed/);assert.equal(reached,true,target.path);
 }
 const saved=JSON.parse(await readFile(new URL('docs/g3/server-c3-potion-correction-dependency.json',root),'utf8'));
 for(const p of saved.preservedBaselineFiles)assert.equal(sha(await readFile(new URL(p.path,root))),p.sha256,p.path);
});
test('explicit diagnostic selector/4358 host rejects regular flags, old metadata, old namespace and caller grants',async()=>{
 const p=await verifyNativePotionCorrectionArtifacts(),base:any={potionCorrection:true,live:true,tls:{pfx:Buffer.alloc(0)},databaseUrl:'not-reached',namespace:'g3_c2_potion_correction_not_reached',serviceId:p.serviceId,mockEnabled:true,keys:{},compatibility:p.compatibility};
 for(const patch of [{potionDiagnostic:true},{live:false},{tls:undefined},{c2Runtime:true},{native:true},{nativeSocial:true},{social:true},{hourglass:true},{nemesis:true},{a1Runtime:true},{a1RuntimeFix:true},{a1RuntimeDiagnostic:true},{a1HeartbeatBusy:true},{serviceId:'nt2-g3-native-c2-development'},{namespace:'g3_c2_regular'},{profile:p.profile},{compatibility:{...p.compatibility,capabilities:[...p.compatibility.capabilities,'core.gold']}}])await assert.rejects(createG3CandidateApp({...base,...patch}),/FORBIDDEN/);
 for(const patch of [{live:false},{tls:false},{c2Runtime:true},{native:true},{nativeSocial:true},{social:true},{hourglass:true},{nemesis:true},{a1Runtime:true}])await assert.rejects(startG3Fixture({potionCorrection:true,live:true,tls:true,databaseUrl:'not-reached',...patch}));
 const config=JSON.parse(await readFile(new URL('deploy/g3-c3-potion-correction/config.lan-test.json',root),'utf8'));assert.equal(validateNativePotionCorrectionLanConfig(config).port,4358);
 for(const patch of [{potionDiagnostic:true},{port:4357},{port:4356},{live:false},{potionCorrection:false},{c2Runtime:true},{namespace:'g3_c2_lan_192_168_0_104_4357'},{serviceId:'nt2-g3-native-c2-development'},{runtimeApprovalSha256:'ccd0d38d806e04f3e02df840cc524e958ca288e19f5de2473330912bd363accb'},{sourceReviewSha256:'0'.repeat(64)},{tlsSourceConfig:'other'},{databaseSecretConfig:'other'},{capabilities:['core.gold']}])assert.throws(()=>validateNativePotionCorrectionLanConfig({...config,...patch}));
});
