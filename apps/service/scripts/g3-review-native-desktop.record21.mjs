// Exact request21 DEVELOPMENT artifact review. No builds/install/network/GUI operations.
import fs from 'node:fs';import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex'),demand=(v,m)=>{if(!v)throw Error(m);};
const checked=p=>{const b=fs.readFileSync(p.path);demand(sha(b)===p.sha256,'Pin drift: '+p.path);return b;};
const manifest={path:'apps/desktop/g3-native-review-request.record21.json',sha256:'60cbf88080576db404fae0142648031b3b719320f7e523697b43b7bbac080e4c'};
const m=JSON.parse(checked(manifest)),root='spikes/noita-entry/.build/g3-candidates/nt2-g3-cp20-20261008-002509-3c5c0e73';
const out='deploy/profiles/g3-native-desktop-review.record21.json',evidencePath='docs/g3/server-check-results.a1-desktop.record21.json';
demand(!fs.existsSync(out)&&!fs.existsSync(evidencePath),'Review/evidence already exists');
demand(m.frozen&&m.frozenRoot===root&&m.pins.length===312&&m.sourceProvenance.length===312&&new Set(m.pins.map(p=>p.path)).size===312,'Frozen scope');
demand(m.approved===false&&m.installApproved===false&&m.packaged===false&&m.actualNoitaEvidence===false&&m.buildVariant==='g3-a1-runtime-fix','Request policy');
for(const p of m.pins){demand(p.path.startsWith(root+'/')&&!p.path.split('/').includes('..'),'Unsafe frozen path');checked(p);}
for(const p of m.sourceProvenance){demand(m.pins.some(x=>x.path===p.frozenPath&&x.sha256===p.sha256),'Provenance missing');checked({path:p.sourcePath,sha256:p.sha256});}
checked({path:root+'/review-request.json',sha256:manifest.sha256});
const frozen=path=>{const p=m.sourceProvenance.find(p=>p.sourcePath===path);demand(p,'Missing input '+path);return {path:p.frozenPath,sha256:p.sha256};};
const text=path=>checked(frozen(path)).toString('utf8');
const resourcePins=[
 {path:'deploy/profiles/g3-native-runtime-approval.record4-a1-runtime-fix.json',sha256:'301b1a8bd0d88804303ea7d9a0d199c71a014ae38fe6c8528704b862a76f1eb8'},
 {path:'deploy/profiles/g3-native-social-runtime-approval.record3-a1-runtime-fix.json',sha256:'fe726f360170d2d738b91a5c95d0f787a5df87f0759b368d4c8d93dd193f0077'}
];
const sourceReview={path:'deploy/profiles/g3-native-a1-source-review.record20.json',sha256:'4c84e37e56c7bd5b87df07749996083539fa04277e3dd5684782a0453b2114f7'};
const resources=resourcePins.map(p=>JSON.parse(checked(p))),prior=JSON.parse(checked(sourceReview));
demand(prior.usableCandidate===false&&prior.consumerRebuildRequired&&prior.finalRebuiltDesktopReviewRequired,'Source-only review');
for(const p of prior.pins)checked(p);
for(const [i,r] of resources.entries()){
 demand(r.id===(i?'nt2-g3-native-social-development-3':'nt2-g3-native-development-4')&&r.serviceId===(i?'nt2-g3-native-social-development-a1':'nt2-g3-native-development-a1')&&r.namespacePrefix===(i?'g3_social_a1_':'g3_a1_'),'Runtime identity');
 demand(r.sourceApproved===true&&r.developmentOnly===true&&r.installApproved===false&&r.packaged===false&&r.actualNoitaEvidence===false,'Runtime policy');
 demand(JSON.stringify(r.compatibility.capabilities)===JSON.stringify(i?['core.seed','core.ghost','core.hud','core.poi']:['core.seed','core.ghost']),'Capability expansion');
 demand(r.requiredLocalFiles.length===(i?20:14)&&r.runtimeMappings.length===r.requiredLocalFiles.length,'Local closure');
 demand(m.runtimeResources.some(p=>p.path===resourcePins[i].path&&p.sha256===resourcePins[i].sha256)&&m.sourceDependencyReview.sha256===sourceReview.sha256,'Manifest resource link');
 demand(frozen(resourcePins[i].path).sha256===resourcePins[i].sha256,'Frozen runtime bytes');
 for(const p of r.provenance.pins)checked(p);
 for(const p of r.runtimeMappings){checked({path:p.sourcePath,sha256:p.sha256});demand(frozen(p.sourcePath).sha256===p.sha256&&r.requiredLocalFiles.some(x=>x.path===p.path&&x.sha256===p.sha256),'Mapping provenance');}
}
demand(m.schemaSha256===resources[0].schemaSha256&&m.modVersion===resources[0].compatibility.modVersion&&m.bridgeVersion===resources[0].compatibility.bridgeVersion,'Primary contract');
demand(m.requestedBuildPairs.length===2&&m.additionalUnapprovedCandidates.length===1,'Candidate count');
for(const p of m.requestedBuildPairs)demand(resources.every(r=>r.profile.supportedBuilds.some(x=>x.id===p.id&&x.store===p.store&&x.gameExeSha256===p.gameExeSha256&&x.originalLuaSha256===p.originalLuaSha256)),'Exact original pair');
const extra=m.additionalUnapprovedCandidates[0];demand(extra.modVersion===resources[1].compatibility.modVersion&&extra.bridgeVersion===resources[1].compatibility.bridgeVersion&&extra.approved===false&&extra.installApproved===false&&extra.productionCapabilities.length===0,'Social scope');
const expected=['8e7c41cd3f6531de1d83e4439030adb11bd559708d7167d309ca894eea388e38','644af34a58a80a24c0493830229fd3cce2fd69ff41722aeadb8b3a28a21cacc0','c7118effab4b0c1d5361a4d23dc64386d0ce4eb1fc3eeceafed754b28fa4b447','cad4e5aab13efb06c917fcfeff9b956f3159ddf9add7ab64943ef2ec03d87978'];
const binaries=[m.candidateExecutable,extra.candidateExecutable,m.nativeArtifact,extra.nativeArtifact].map((path,i)=>{
 const pin=m.pins.find(p=>p.path===path);demand(pin&&pin.sha256===expected[i],'Binary identity');const b=checked(pin),at=b.readUInt32LE(60);
 demand(b.toString('ascii',0,2)==='MZ'&&b.toString('ascii',at,at+4)==='PE\0\0','PE signature');const machine=b.readUInt16LE(at+4),dll=Boolean(b.readUInt16LE(at+22)&0x2000);
 demand(i<2?machine===0x8664&&!dll:machine===0x14c&&dll,'PE architecture/type');
 if(i<2)demand(b.includes(checked(resourcePins[i]))&&b.includes(Buffer.from(resourcePins[i].sha256)),'Compiled exact resource missing');
 return {...pin,machine:machine.toString(16),dll,exactRuntimeEmbedded:i<2};
});
// Feature/default pins coexist; CLI exits before all GUI/pipe/hint startup.
for(const [i,path] of ['apps/desktop/src-tauri/src/g3_installation.rs','apps/desktop/src-tauri/src/g3_social_installation.rs'].entries()){
 const t=text(path);demand(t.includes('#[cfg(feature="g3-a1-runtime-fix")]')&&t.includes('#[cfg(not(feature="g3-a1-runtime-fix"))]')&&t.includes(resourcePins[i].sha256),'Matched/default feature');
}
demand(text('apps/desktop/src-tauri/Cargo.toml').includes('g3-a1-runtime-fix = []'),'Build feature');
for(const path of ['apps/desktop/src-tauri/examples/g3_candidate.rs','apps/desktop/src-tauri/examples/g3_social_candidate.rs']){
 const t=text(path);demand(t.includes('Some("--verify-kit-root")')&&t.indexOf('std::process::exit')<t.indexOf('next_app::run_g3'),'CLI early exit');
}
const old20=JSON.parse(fs.readFileSync('apps/desktop/g3-native-review-request.record20.json'));
const retained=m.sourceProvenance.filter(p=>p.sourcePath.startsWith('native/')||p.sourcePath.startsWith('mod/')||p.sourcePath==='apps/desktop/src-tauri/src/next_client.rs'||p.sourcePath==='apps/desktop/src-tauri/src/pipe.rs');
for(const p of retained)demand(old20.sourceProvenance.some(x=>x.sourcePath===p.sourcePath&&x.sha256===p.sha256),'Reviewed authority/native/entry changed');
const clientEvidence={path:'apps/desktop/g3-client-runtime-fix-consumer-check-results.record21.json',sha256:'8b044bb794cc26d7f4ded7e36823064d3032fe19512a3213a8ac6f8653df0b95'},cr=JSON.parse(checked(clientEvidence));
demand(frozen(clientEvidence.path).sha256===clientEvidence.sha256&&cr.passed&&cr.buildVariant==='g3-a1-runtime-fix'&&cr.cliChecks===156,'Consumer evidence');
for(const p of cr.evidencePins){checked(p);demand(frozen(p.path).sha256===p.sha256,'Evidence not frozen');}
demand(cr.cli.results.reduce((sum,r)=>sum+r.checks,0)===156&&cr.cli.results.every(r=>r.eachRuntimeDriftRefused&&r.wrongOriginalRefused&&r.oldCompleteRuntimeRefused&&r.restorationVerified&&r.readOnlyCli&&!r.actualNoita),'Retained CLI156');
demand(cr.focusedVerifierTests.passed===5&&cr.focusedVerifierTests.failed===0&&text(cr.focusedVerifierTests.log).includes('5 passed; 0 failed'),'Verifier5');
const passedLog='spikes/noita-entry/.build/g3-runtime-fix-app21-06d01316082ae8/joined-checked.txt';demand(text(passedLog).includes('1 passed; 0 failed')&&text(passedLog).includes('66.54s'),'Checked joined fixture');
const joinedRoot='spikes/noita-entry/.build/g3-joined/216e61e5-33e9-417a-ac7f-58a2cc30ba5d';
for(const path of [joinedRoot+'/result.json',joinedRoot+'/runtime-stability.json'])demand(JSON.parse(text(path)).passed??path.endsWith('runtime-stability.json'),'Joined result');
const stability=JSON.parse(text(joinedRoot+'/runtime-stability.json'));demand(stability.initialPeerGhosts===2&&stability.stableCycles===5&&stability.delayedSocialCaptureMs>=650&&stability.unexpectedHolds===0&&stability.actualNoita===false,'Measured stability');
const test=text('apps/desktop/src-tauri/src/next_joined_test.rs');for(const guard of ['presence(c).await','delayed_social_ms>=650','Late fault requires a valid playing binding','assert_eq!(held["kind"],"ghosts")','Reached ghost response must change the exact original generation'])demand(test.includes(guard),'Strict fixture guard');
demand(cr.fixtureCorrection.productionAuthorityChanged===false&&cr.fixtureCorrection.historicalFailureRetained&&cr.fixtureCorrection.originalRetirementCausePreciselyAttributed===false,'Retained failure limitation');
const freezeEvidence={path:'apps/desktop/g3-client-runtime-fix-freeze-check-results.record21.json',sha256:'2332da4b9fc4f1e9e1e51d7dd64bb17b535d65405e321934c670857db3a614d8'},fr=JSON.parse(checked(freezeEvidence));demand(fr.passed&&fr.requestSha256===manifest.sha256&&fr.pins===312&&fr.runtimeMappingsChecked===34,'Freeze evidence');
// Review package predicate linkage without executing assembler or installer.
const assembler=text('apps/desktop/tools/package-g3-runtime-fix-test.ps1');for(const term of ["'source-reviewed-development-only'",'$taskReview.manifest.sha256','$taskReview.pins','$taskReview.selectedTestExecutable.path','$taskReview.selectedTestExecutable.sha256','$taskReview.supplementalRuntimeApproval.sha256','Get-ReviewedFrozenSource','Package assembler changed after exact review'])demand(assembler.includes(term),'Package predicate');
const installer=text('apps/desktop/tools/install-g3-runtime-fix-test-mod.ps1');demand(installer.includes(resourcePins[1].sha256)&&installer.includes('if($ValidateOnly)')&&installer.includes('nt2-mod-backups')&&installer.includes('Installer containment refused'),'Installer linkage');
const guide=text('apps/desktop/G3-runtime-fix-test-README.md');demand(guide.includes('https://192.168.0.104:4357')&&guide.includes('Full Players/Messages lists')&&guide.includes('same-run rejoin'),'Owner guide');
for(const path of ['deploy/g2-lan/public/ca.cer','deploy/g2-lan/public/server.cer','deploy/g3-native-a1-runtime-fix/config.lan-test-4357.json','deploy/g3-native-a1-runtime-fix/README.md','deploy/g3-native-a1-runtime-fix/Start-G3-A1-Runtime-Fix.ps1','mod/noita_together2/mod-g3-social.xml'])frozen(path);
const cliPin={path:'spikes/noita-entry/.build/g3-server-request21-readonly/checks.json',sha256:'ca65137546d198f4bf37412fef40eb60d0e29efe6e6c4e78eb389e648d56e724'},cli=JSON.parse(checked(cliPin));
demand(cli.results.length===2,'Fresh CLI count');for(const [i,r] of cli.results.entries()){checked(r);demand(r.executable===binaries[i].path&&r.executableSha256===expected[i]&&r.exitCode===0&&r.result.passed&&r.result.readOnly&&r.result.actualNoita===false&&r.result.installApproved===false,'Fresh exact read-only CLI');}
let oldFrozenPins=0;for(const n of [18,19,20]){const old=JSON.parse(fs.readFileSync('apps/desktop/g3-native-review-request.record'+n+'.json'));for(const p of old.pins){checked(p);oldFrozenPins++;}}
const evidence={activeGate:'G3A/A1',checkpoint:'exact request21 development artifact review',manifest,frozenRoot:root,frozenPins:312,currentSourcePins:312,evidencePins:cr.evidencePins.length,oldFrozenPins,runtimeMappingInputs:34,binaryChecks:binaries,freshReadOnlyCli:cliPin,clientEvidence,freezeEvidence,reviewer:{path:'apps/service/scripts/g3-review-native-desktop.record21.mjs',sha256:sha(fs.readFileSync('apps/service/scripts/g3-review-native-desktop.record21.mjs'))},checks:{retainedCli:156,retainedVerifierTests:5,retainedCheckedFixtureSeconds:66.54,initialGhosts:2,measuredSocialDelayMs:stability.delayedSocialCaptureMs,stableCycles:5,unexpectedHolds:0,exactPackagePredicate:true,sourceAuthorityNativeEntryUnchanged:true},limitations:['All fixture engine/PID/APIs injected; no actual Noita/two-PC acceptance.','joined-checked.txt is the successful66.54s run. joined-final.txt contains an earlier failure; retained and not counted as passing.','Earlier450ms automatic release mislabeled650ms and original retirement attribution remain unproven; strict public-presence/measured-delay/reached-ghost/exact-generation fixture correction is test-only.','No ZIP export/installer invocation/service preflight or future executable approval.'],installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,blockingFindings:[]};
fs.writeFileSync(evidencePath,JSON.stringify(evidence,null,2)+'\n',{flag:'wx'});
const review={revision:'nt2-g3-desktop-review-21',status:'source-reviewed-development-only',activeGate:'G3A/A1',sourceApproved:true,developmentOnly:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,manifest,snapshotRequest:{path:root+'/review-request.json',sha256:manifest.sha256},frozenRoot:root,runtimeApproval:resourcePins[0],supplementalRuntimeApproval:resourcePins[1],sourceDependencyReview:sourceReview,primaryTestExecutable:m.pins.find(p=>p.path===m.candidateExecutable),selectedTestExecutable:m.pins.find(p=>p.path===extra.candidateExecutable),pins:m.pins,sourceProvenance:m.sourceProvenance,framing:m.framing,buildVariant:m.buildVariant,binaryChecks:binaries,clientEvidence,freezeEvidence,reviewEvidence:{path:evidencePath,sha256:sha(fs.readFileSync(evidencePath))},blockingFindings:[],limitations:evidence.limitations};
demand(review.pins.length===312&&review.selectedTestExecutable.path===extra.candidateExecutable&&review.manifest.sha256===manifest.sha256&&review.supplementalRuntimeApproval.sha256===m.runtimeResources[1].sha256,'Package acceptance predicate');
fs.writeFileSync(out,JSON.stringify(review,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({path:out,sha256:sha(fs.readFileSync(out)),evidence:review.reviewEvidence,pins:312,oldFrozenPins,freshCli:2}));
