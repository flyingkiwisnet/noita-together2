import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {candidateLanConfig,validateCandidateLaunch,requireCandidateMock,ensureUnusedPort} from '../src/g2-next/launch.js';
import {inspectionMetadata,verifyInspectionArtifacts,validateCandidateMetadata,inspectionProfile} from '../src/g2-next/profile.js';
import {createServer} from 'node:net';
test('successor native profile matches current source-built manifest and rejects historical/simulated/native mixing',async()=>{
 const value=inspectionMetadata();validateCandidateMetadata(value);const manifestSha=createHash('sha256').update(await readFile('apps/desktop/g2-inspect-native-profile.json')).digest('hex');if(manifestSha!==inspectionProfile.provenance.manifest.sha256)await assert.rejects(verifyInspectionArtifacts(),/manifest changed/);else{const proof=await verifyInspectionArtifacts();assert.equal(proof.id,'nt2-g2-inspect-native-1');assert.equal(proof.evidenceStatus,'candidate-engine-evidence-pending');assert.equal(proof.artifacts.length,3);}
 for(const profile of [{...value.profile,id:'nt2-g2-native-1'},{id:'g2-next-simulated',evidenceStatus:'simulated-game-facts'},{...value.profile,bridgeSha256:'0'.repeat(64)},{...value.profile,evidenceStatus:'verified'}])assert.throws(()=>validateCandidateMetadata({...value,profile}));
 for(const compatibility of [{...value.compatibility,modVersion:'nt2-g2-1'},{...value.compatibility,bridgeVersion:'g2-pipe32-1'},{...value.compatibility,capabilities:['core.seed','nemesis']},{...value.compatibility,modes:['core','nemesis']}])assert.throws(()=>validateCandidateMetadata({...value,compatibility}));
 // No-profile fixture is distinctly advertised as simulated; it cannot pass the native launcher selection.
 validateCandidateMetadata({compatibility:value.compatibility});
 const historical=JSON.parse(await readFile('deploy/profiles/g2-candidate.json','utf8'));assert.equal(historical.profile.id,'nt2-g2-native-1');assert.notEqual(historical.profile.bridgeSha256,value.profile.bridgeSha256);
 for(const path of ['g2-inspect-native-profile.json','nt2_g2_inspect_bridge.dll','init-inspect.lua','inspection.lua','bridge.cpp','build.ps1'])await assert.rejects(verifyInspectionArtifacts(async url=>url.pathname.endsWith('/'+path)?Buffer.from(path.endsWith('.json')?'{}':'changed candidate bytes'):readFile(url)),/changed|differ/);
});
test('successor launch binds its own port/identity/namespace and pinned candidate draft; refuses reserved and public endpoints',async()=>{
 const example=JSON.parse(await readFile('deploy/g2-next/config.multibuild.pending.json','utf8'));assert.deepEqual(validateCandidateLaunch(example),example);
 for(const change of [{port:4352},{port:4351},{port:4355},{ip:'127.0.0.1'},{ip:'8.8.8.8'},{origin:'http://192.168.0.104:4356'},{serviceId:'nt2-g2-lan-192-168-0-104'},{namespace:'g2_lan_192_168_0_104_4352'},{profileId:'nt2-g2-native-1'},{contractRevision:'g2-draft-1'},{contractSha256:'0'.repeat(64)},{extra:true}])assert.throws(()=>validateCandidateLaunch({...example,...change}));
 assert.notEqual(candidateLanConfig(example.ip,4357).serviceId,example.serviceId);assert.notEqual(candidateLanConfig(example.ip,4357).namespace,example.namespace);
 requireCandidateMock({NT2_G2_NEXT_MOCK:'1',NODE_ENV:'development'});assert.throws(()=>requireCandidateMock({NT2_G2_MOCK:'1'}));assert.throws(()=>requireCandidateMock({NT2_G2_NEXT_MOCK:'1',NODE_ENV:'production'}));
 const blocker=createServer();await new Promise<void>(done=>blocker.listen(0,'127.0.0.1',done));try{const address=blocker.address();assert.ok(address&&typeof address!=='string');await assert.rejects(ensureUnusedPort('127.0.0.1',address.port),/unavailable/);}finally{await new Promise<void>((done,fail)=>blocker.close(e=>e?fail(e):done()));}
});
test('successor launcher refuses missing explicit mock flag/production before TLS, database or listener work',()=>{
 for(const env of [{NODE_ENV:'production',NT2_G2_NEXT_MOCK:'1'},{NODE_ENV:'development',NT2_G2_NEXT_MOCK:''}]){const result=spawnSync(process.execPath,['--import','tsx','apps/service/scripts/g2-next-lan.ts','missing-config.json','--check'],{env:{...process.env,...env},encoding:'utf8',timeout:10000});assert.equal(result.status,1);assert.match(result.stderr,/requires NT2_G2_NEXT_MOCK=1 and refuses production/);}
});
