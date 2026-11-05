// Immutable, source-only A1 review. Never rebuild candidates or replace approvals.
import {readFile,writeFile,readdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const root=new URL('../../../',import.meta.url),sha=b=>createHash('sha256').update(b).digest('hex');
const bytes=p=>readFile(new URL(p,root)),json=async p=>JSON.parse(await bytes(p));
const requestPath='apps/desktop/g3-native-review-request.record17.json',requestBytes=await bytes(requestPath);
assert.equal(sha(requestBytes),'7c37e02ca5ee106104004e629f163d2e824d13d379b46c0f4dd0da5c662e5e0e');
const request=JSON.parse(requestBytes),frozen=request.frozenRoot;
assert.equal(request.pins.length,198);assert.equal(new Set(request.pins.map(p=>p.path)).size,198);
for(const p of request.pins){assert.ok(p.path.startsWith(frozen+'/')&&!p.path.includes('..'));assert.equal(sha(await bytes(p.path)),p.sha256,p.path);}
// Compare the actual source copies too, without granting mutable working-tree bytes.
let current=0;for(const p of request.pins){if(sha(await bytes(p.path.slice(frozen.length+1)))===p.sha256)current++;}
const old=[];for(const name of await readdir(new URL('deploy/profiles/',root))){if(/\.json$/.test(name)){const path='deploy/profiles/'+name;old.push({path,sha256:sha(await bytes(path))});}}
const pin=path=>{const found=request.pins.find(p=>p.path===path);assert.ok(found,'Missing frozen pin '+path);return found;};
const write=async(path,value)=>{const data=Buffer.from(JSON.stringify(value,null,2)+'\n');await writeFile(new URL(path,root),data,{flag:'wx'});return {path,sha256:sha(data)};};
const localProtocol={requestMagic:'NTG3',requestVersion:1,requestBytes:64,replyMagic:'NG3R',replyVersion:2,replyBytes:864,maxRoster:32,maxPoses:4,maxAuthorityLeaseMs:200,leaseOrigin:'native request start',canonicalProtocolChanged:false};
const resources=[];
for(const social of [false,true]){
 const base=await json('deploy/profiles/'+(social?'g3-native-social-runtime-approval.record1.json':'g3-native-runtime-approval.record2-complete.json'));
 const candidate=social?request.additionalUnapprovedCandidates[0]:request;
 const id=social?'nt2-g3-native-social-development-2':'nt2-g3-native-development-3';
 const mappings=base.requiredLocalFiles.map(p=>{
  const suffix=p.path.slice('mods/noita_together2/'.length);
  const sourcePath=suffix==='init.lua'?candidate.entrySource:suffix.endsWith('.dll')?candidate.nativeArtifact:frozen+'/mod/noita_together2/'+suffix;
  return {path:p.path,sourcePath,sha256:pin(sourcePath).sha256};
 });
 const registry={...structuredClone(base.buildRegistry??base.profile),id,bridgeSha256:pin(candidate.nativeArtifact).sha256,modSha256:pin(candidate.entrySource).sha256};
 assert.deepEqual(registry.supportedBuilds.map(({id,store,gameExeSha256,originalLuaSha256})=>({id,store,gameExeSha256,originalLuaSha256})),request.requestedBuildPairs.map(({id,store,gameExeSha256,originalLuaSha256})=>({id,store,gameExeSha256,originalLuaSha256})));
 const refs=[],external=new Set();
 for(const m of mappings.filter(m=>/\.(lua|xml)$/.test(m.path))){
  const text=(await bytes(m.sourcePath)).toString('utf8');
  for(const match of text.matchAll(/["']((?:mods\/noita_together2\/|data\/)[^"'\s]+)["']/g)){
   const to=match[1];if(to.startsWith('data/'))external.add(to);else {assert.ok(mappings.some(p=>p.path===to),'Unmapped closure reference '+to);refs.push({from:m.path,to});}
  }
 }
 const runtime={revision:'nt2-g3-a1-runtime-source-review-1',id,status:'source-reviewed-development-consumer-rebuild-pending',sourceApproved:true,developmentOnly:true,installApproved:false,packaged:false,actualNoitaEvidence:false,
  serviceId:social?'nt2-g3-native-social-development-a1':'nt2-g3-native-development-a1',namespacePrefix:social?'g3_social_a1_':'g3_a1_',pipePrefix:social?candidate.pipePrefix:'nt2-g3-dev-v1-',
  lobbyRevision:base.lobbyRevision,gameplayRevision:base.gameplayRevision,schemaSha256:base.schemaSha256,...(social?{socialRevision:base.socialRevision,socialSchemaSha256:base.socialSchemaSha256,replayPolicy:base.replayPolicy}:{}),
  compatibility:base.compatibility,buildRegistry:registry,profile:social?{...registry,gameVersion:base.compatibility.gameVersion,modVersion:base.compatibility.modVersion,bridgeVersion:base.compatibility.bridgeVersion,capabilities:base.compatibility.capabilities}:registry,
  requiredLocalFiles:mappings.map(({path,sha256})=>({path,sha256})),runtimeMappings:mappings,
  provenance:{frozenRoot:frozen,pins:[...new Map([...mappings.map(m=>pin(m.sourcePath)),...request.pins.filter(p=>{const s=p.path.slice(frozen.length+1);return s.startsWith('native/')||s==='packages/contracts/g3/schema.json'||(social&&s.startsWith('packages/contracts/g3-social/'));})].map(p=>[p.path,p])).values()]},
  localProtocol,closure:{localDependencies:mappings.length,references:refs,baseGameDependencies:[...external].sort(),baseGameEvidence:'Original executable/Lua pairs retained; data assets not independently hashed or engine verified'},
  clientConsumption:{metadataPath:social?'/v3/social/info':'/v1/info',metadataField:social?'runtimeProfile':'profile',approvalPath:social?'/v3/social/runtime':null,consumerRebuildRequired:true,finalRebuiltDesktopReviewRequired:true,policy:'Explicit sourceApproved/developmentOnly policy; retain installApproved=false. Replace old compiled resource pin and exact profile identity only in a separately assigned client checkpoint.'},
  limitations:['Only existing seed/ghost'+(social?'/HUD/POI':'')+' development capability closure','No gold/emote/cosmetic/menu/hourglass/Nemesis grant','Request17 desktop bytes still consume older runtime pins; this is not usable-kit approval','Actual Noita, LAN smoothness, base-game assets, input, save/Continue/crash and G3-R13 remain pending']};
 // Runtime contains no desktop executable or manifest digest (no consumer digest cycle).
 assert.ok(!JSON.stringify(runtime).includes('candidate.exe'));
 const resource=await write('deploy/profiles/'+(social?'g3-native-social-runtime-approval.record2-a1.json':'g3-native-runtime-approval.record3-a1.json'),runtime);
 resources.push({...resource,runtimeId:id,capabilities:base.compatibility.capabilities});
}
const review=await write('deploy/profiles/g3-native-a1-source-review.record17.json',{revision:'nt2-g3-a1-request17-source-review-1',activeGate:'G3',section:'G3A',checkpoint:'A1',sourceApproved:true,developmentOnly:true,installApproved:false,packaged:false,actualNoitaEvidence:false,usableCandidate:false,consumerRebuildRequired:true,finalRebuiltDesktopReviewRequired:true,frozenRoot:frozen,snapshotRequest:{path:requestPath,sha256:sha(requestBytes)},runtimeApprovals:resources,pins:request.pins,currentSourceMatches:current,localProtocol,desktopExecutables:[pin(request.candidateExecutable),pin(request.additionalUnapprovedCandidates[0].candidateExecutable)],scope:'Source dependency review only; does not replace final desktop approval or authorize packaging/install'});
for(const p of old)assert.equal(sha(await bytes(p.path)),p.sha256);
await write('docs/g3/server-native-profile-review.a1.record1.json',{activeGate:'G3',section:'G3A',checkpoint:'A1 runtime/profile',frozenPinsVerified:198,currentSourceMatches:current,resources,sourceReview:review,preservedProfiles:old,localProtocol,actualGameAcceptance:false});
console.log(JSON.stringify({resources,review,frozenPins:198,currentSourceMatches:current,preservedProfiles:old.length},null,2));
