// Refresh the selected normal development approvals from concrete Client build
// inputs. No compilation, installation, service/DB start or trust changes.
// Run: node --import tsx apps/service/scripts/prepare-testing-approval.mjs --input <runtime-inputs.json>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';
import {inspectNativeExports} from '../src/rejoin/composition-profile.ts';
import {validateBuildRegistry} from '../src/g2-next/build-registry.ts';
const args=process.argv.slice(2);assert.equal(args.length,2);assert.equal(args[0],'--input');
const sha=b=>createHash('sha256').update(b).digest('hex'),read=p=>fs.readFileSync(p),json=p=>JSON.parse(read(p)),pin=p=>({path:p,sha256:sha(read(p))});
const safe=p=>{assert.equal(typeof p,'string');assert.ok(!path.isAbsolute(p)&&!/[\\:]/.test(p)&&p.split('/').every(v=>v&&v!=='.'&&v!=='..'),'repository-relative input required');return p;};
const checked=p=>{safe(p.path);assert.match(p.sha256,/^[a-f0-9]{64}$/);const b=read(p.path);assert.equal(sha(b),p.sha256,'input drift '+p.path);return b;};
const runtimePath='deploy/profiles/g3-testing-loadout-runtime.20261030.json',sourcePath='deploy/profiles/g3-testing-loadout-source.20261030.json',configPath='deploy/g3-rejoin/config.testing-loadout-20261030.json';
const inputPin=pin(safe(args[1])),c=JSON.parse(checked(inputPin)),base=json(runtimePath),source=json(sourcePath),config=json(configPath),before={runtime:pin(runtimePath),source:pin(sourcePath),config:pin(configPath)};
assert.equal(c.runtimeId,base.id);assert.equal(base.sourceApproved,true);assert.equal(base.developmentOnly,true);for(const key of ['installApproved','productionApproved','packaged','actualNoitaEvidence','fixtureOnly'])assert.equal(base[key],false);
const report=JSON.parse(checked(c.clientReport));assert.equal(report.passed,true);for(const p of report.sourceInputs)checked(p);checked(c.packagingVerifierDelta);if(c.buildLog)checked(c.buildLog);
const reviewed=new Map(report.sourceInputs.map(p=>[p.path,p.sha256]));
const mapped=new Map();assert.ok(Array.isArray(c.runtimeMappings)&&c.runtimeMappings.length>=base.runtimeMappings.length&&c.runtimeMappings.length<=512);
for(const m of c.runtimeMappings){safe(m.path);assert.ok(m.path.startsWith('mods/noita_together2/')&&!mapped.has(m.path));checked({path:m.sourcePath,sha256:m.sha256});mapped.set(m.path,m);}
for(const m of base.runtimeMappings)assert.ok(mapped.has(m.path),'existing runtime omitted '+m.path);
assert.equal(c.nativeArtifacts.length,3);assert.deepEqual(c.nativeArtifacts.map(a=>a.mode).sort(),['broker','primary','social']);
const native=c.nativeArtifacts.map(a=>{
 assert.equal(a.private,false);assert.equal(a.testFlags,false);const exports=inspectNativeExports(checked(a)),prior=base.nativeArtifacts.find(p=>p.mode===a.mode);
 const expected=[...new Set([...prior.exports,...(a.mode==='broker'?[]:['nt2_g3_lobby_status'])])].sort();assert.deepEqual(exports,expected,'reviewed native export closure '+a.mode);assert.deepEqual(exports,[...a.exports].sort(),'declared native exports');
 const command=checked(a.buildCommand).toString();assert.ok(command.includes('/Brepro')&&/[-/]arch[:=]x86/.test(command)&&command.includes('/W4')&&command.includes('/WX')&&!/\/DNT2_(IPC_PRIVATE|TEST_)/.test(command),'normal x86 build required');
 assert.ok([...mapped.values()].some(m=>m.sourcePath===a.path&&m.sha256===a.sha256),'native outside runtime mapping');return {...a,exports,loaded:false,installed:false};
});
const delta=[...mapped.values()].filter(m=>{const old=base.runtimeMappings.find(p=>p.path===m.path);return !old||old.sha256!==m.sha256||old.sourcePath!==m.sourcePath;});
for(const m of delta)assert.ok(reviewed.get(m.sourcePath)===m.sha256||native.some(a=>a.path===m.sourcePath&&a.sha256===m.sha256),'runtime delta outside reviewed source/build '+m.path);
const entry=mapped.get(base.closure.entry),primary=native.find(a=>a.mode==='primary'),registry={...base.buildRegistry,modSha256:entry.sha256,bridgeSha256:primary.sha256};validateBuildRegistry(registry);
const catalogue=pin('packages/contracts/g3/testing-starting-loadout.json'),definition=JSON.parse(checked(catalogue));assert.equal(definition.default,false);assert.equal(definition.pickups.noMoreShufflePerk,'NO_MORE_SHUFFLE');assert.equal(definition.pickups.noMoreShuffleCount,1);
const sourcePins=[...new Map([...report.sourceInputs,c.packagingVerifierDelta,...(c.sourcePins??[])].map(p=>{checked(p);return [p.path,p];})).values()];
const testingStartingLoadout={...base.testingStartingLoadout,catalogue};
const runtime={...base,status:'reviewed-protected-lobby-and-testing-loadout-development-owner-engine-smoke-pending',buildRegistry:registry,profile:{...base.profile,...registry},runtimeMappings:c.runtimeMappings,requiredLocalFiles:c.runtimeMappings.map(({path,sha256})=>({path,sha256})),nativeArtifacts:native,closure:{...base.closure,localDependencies:mapped.size},compositionInput:inputPin,testingStartingLoadout,localProtocol:{...base.localProtocol,lobbyStatus:{operation:19,bytes:256,magic:'NLD1',revision:1,presentationOnly:true}},provenance:{...base.provenance,clientDependency:inputPin,protectedLobbyClientReport:c.clientReport,...(c.buildLog?{normalNativeBuildLog:c.buildLog}:{}),pins:sourcePins},evidence:{...base.evidence,protectedLobbyStatusSourceReviewed:true,noMoreShuffleFixedPickupSourceReviewed:true},remainingEvidence:'Actual Noita protected lobby text/layout, pickup placement and Continue remain owner tests; focused source/component evidence uses injected engine facts.'};
const encode=v=>JSON.stringify(v,null,2)+'\n',runtimeBytes=encode(runtime),runtimeApproval={path:runtimePath,sha256:sha(runtimeBytes)};
const review={...source,checkpoint:'matching-reviewed-protected-lobby-and-testing-loadout-development-build',input:inputPin,runtimeApproval,pins:sourcePins,nativeArtifacts:native,runtimeDelta:source.input?.sha256===inputPin.sha256?source.runtimeDelta:delta,testingStartingLoadout,protectedLobbyClientReport:c.clientReport,...(c.buildLog?{normalNativeBuildLog:c.buildLog}:{})};
const sourceBytes=encode(review),sourceApproval={path:sourcePath,sha256:sha(sourceBytes)},nextConfig={...config,runtimeApprovalSha256:runtimeApproval.sha256,sourceReviewSha256:sourceApproval.sha256};
// All validation completes before changing the selected canonical files. The
// installed host remains frozen independently until Root's reversible publish.
for(const [p,b]of [[runtimePath,runtimeBytes],[sourcePath,sourceBytes],[configPath,encode(nextConfig)]])fs.writeFileSync(p,b);
console.log(JSON.stringify({input:inputPin,before,runtimeApproval,sourceApproval,config:pin(configPath),runtimeId:runtime.id,runtimeFiles:mapped.size,nativeArtifacts:native.map(({mode,path,sha256,exports})=>({mode,path,sha256,exports})),runtimeDelta:delta,serviceStarted:false,databaseTouched:false,installed:false}));
