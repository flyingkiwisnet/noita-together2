import fs from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const read=p=>fs.readFileSync(p),json=p=>JSON.parse(read(p));
const hash=p=>createHash('sha256').update(read(p)).digest('hex'),pin=p=>({path:p,sha256:hash(p)});
const folder='deploy/g3-rejoin/a3-fix1-host-runtime-20261023',prior='deploy/g3-rejoin/a3-host-runtime-20261023';
const manifest=json(folder+'/manifest.json'),old=json(prior+'/manifest.json');
const runtime=json(manifest.runtimeApproval.path),input=json(runtime.compositionInput.path);
for(const [root,m] of [[folder,manifest],[prior,old]])for(const p of m.files)assert.equal(hash(root+'/'+p.path),p.sha256,p.path);
for(const p of manifest.sourcePins)assert.equal(hash(p.path),p.sha256,p.path);
for(const p of [manifest.config,manifest.runtimeApproval,manifest.sourceApproval,runtime.compositionInput,input.acceptedFlow,input.acceptedLog,runtime.provenance.serverReview])assert.equal(hash(p.path),p.sha256,p.path);
const migrations=manifest.files.filter(p=>p.path.startsWith('db/migrations/'));
const previous=old.files.filter(p=>p.path.startsWith('db/migrations/'));
for(const p of previous)assert.equal(migrations.find(v=>v.path===p.path)?.sha256,p.sha256,'historical migration changed '+p.path);
const delta=migrations.filter(p=>!previous.some(v=>v.path===p.path));
assert.deepEqual(delta,[]);
for(const path of ['apps/service/src/g1/work.ts','apps/service/src/g2-next/traffic.ts'])assert.equal(hash(path),old.sourcePins.find(p=>p.path===path).sha256,'capacity policy changed '+path);
assert.equal(runtime.id,'nt2-g3-a3-fix1-dev-20261023');
assert.equal(runtime.runtimeMappings.length,170);assert.equal(input.normalArtifacts.length,3);
assert.ok(input.normalArtifacts.every(a=>a.private===false&&a.testFlags===false));assert.equal(input.nativeReuse?.rebuilt,false);
assert.equal(runtime.fixtureNative,undefined);assert.equal(runtime.fixtureOnly,undefined);
assert.equal(runtime.socialRevision,'g3-social-draft-3');assert.equal(runtime.socialSchemaSha256,hash('packages/contracts/g3-social/schema.json'));
for(const c of ['core.emotes','core.cosmetics']){assert.ok(runtime.compatibility.capabilities.includes(c));assert.ok(runtime.profile.capabilities.includes(c));}
assert.equal(runtime.evidence.sourceA3Reviewed,true);assert.equal(runtime.evidence.normalBrowserCorrection,true);assert.equal(runtime.evidence.scopedRecoveryCorrection,true);assert.equal(runtime.evidence.strictEngineApiCorrection,true);assert.equal(runtime.evidence.matchedCompositionPassed,true);
assert.equal(manifest.files.some(p=>/lifecycle-lc1-fixture|lc1-live-capacity-diagnostic|lc1-lifecycle-capacity-diagnostic|lc1-live-roster-diagnostic|lc1-social-rate-precondition|lifecycle-lc1-social-controls/.test(p.path)||p.path.endsWith('.dll')),false);
const config=json(manifest.config.path),priorConfig=json(old.config.path);
assert.deepEqual(Object.keys(config).sort(),Object.keys(priorConfig).sort());
for(const k of Object.keys(priorConfig).filter(k=>!['runtimeApprovalSha256','sourceReviewSha256'].includes(k)))assert.equal(config[k],priorConfig[k],k);
const accepted=json(input.acceptedFlow.path);assert.equal(accepted.status,'accepted-bounded-source-corrections');// Correction review and normal-browser service log are independently pinned by the profile.
assert.equal(accepted.actualNoita,false);assert.equal(accepted.rootReview.rootIntegration.passed,4);assert.equal(accepted.rootReview.rootIntegration.cleanupVerified,true);
const dependency={
 gate:'G3A/A3 owner correction Fix1',owner:'Server',assignment:'docs/active-work.md',checkpoint:'matching-A3-fix1-normal-host-ready',status:'prepared-unpublished',runtimeId:runtime.id,
 input:pin(runtime.compositionInput.path),runtimeApproval:pin(manifest.runtimeApproval.path),sourceApproval:pin(manifest.sourceApproval.path),config:pin(manifest.config.path),manifest:pin(folder+'/manifest.json'),bootstrap:pin(folder+'/boot.mjs'),launcherCandidate:pin('deploy/g3-rejoin/Start-Host.A3-Fix1.candidate.ps1'),publishLauncherTo:'testing/Start-Host.ps1',
 acceptedFlow:input.acceptedFlow,acceptedSourceReview:input.acceptedFlow,acceptedLog:input.acceptedLog,acceptedLogRole:'Root accepted corrected normal-profile browser-to-service checks; prior239.29s composition historical, not actual Noita acceptance',acceptedServerDependency:runtime.provenance.serverReview,acceptedServerDependencyRole:'Reviewed canonical room semantics and scoped browser recovery correction',
 origin:manifest.origin,namespace:manifest.namespace,database:'127.0.0.1:54328/nt2_g1',serviceId:manifest.serviceId,runtimeFiles:170,artifactFiles:manifest.files.length,sourceInputs:manifest.sourcePins.length,thirdPartyPackages:manifest.thirdPartyDependencies.length,nodeVersion:manifest.nodeVersion,normalNativeImages:3,nativeRebuilt:false,nativeReuse:input.nativeReuse,fixtureHooksShipped:0,lifecycle:runtime.lifecycle,canonicalSocial:{revision:runtime.socialRevision,schemaSha256:runtime.socialSchemaSha256},
 migration:{baseline:119,required:[],schemaChange:false,delta:delta.map(p=>pin(p.path)),performedByServer:false,liveRead:false,apply:'No new migration beyond119; Server did not access or migrate the live DB'},
 preserved:{priorA3Manifest:pin(prior+'/manifest.json'),priorA3Files:old.files.length,originIdentityTlsSignerNamespace:true},
 checks:{runtimeAndReusedNativeClosure:true,frozenSourceAndFiles:true,migrationHistoryThrough118Unchanged:true,migrationSetUnchangedThrough119:true,priorFrozenFilesUnchanged:true,fixtureInstrumentationAndPrivateRuntimeExcluded:true,capacityPolicyUnchanged:true},
 rootOnly:['Matching kit and stable launcher publication with recoverable backups','Fresh listener/app/Noita closure check','Installed matching files and actual LAN CheckOnly'],
 remaining:['Client matching corrected A3 GUI/kit/laptop ZIP','Root matching review and publication','Owner actual Noita emotes/cosmetics/input and two-PC recovery smoke'],
 notPerformed:['DB/livehost/migration/GUI/Noita launch','installed launcher/kit/junction publication','credential/signer/TLS/save changes']
};
const output='docs/g3/server-a3-fix1-package-dependency-20261023.json',bytes=JSON.stringify(dependency,null,2)+'\n';
if(fs.existsSync(output))assert.equal(read(output).toString(),bytes,'sealed dependency drift');else fs.writeFileSync(output,bytes,{flag:'wx'});
console.log(JSON.stringify({dependency:pin(output),runtimeId:runtime.id,files:manifest.files.length,priorVerified:old.files.length,socialRevision:runtime.socialRevision,migrationDelta:delta.map(p=>p.path),liveTouched:false}));
