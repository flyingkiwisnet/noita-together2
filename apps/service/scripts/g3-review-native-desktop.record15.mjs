// Independent frozen request15 review; keep runtime1 and all older reviews immutable.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const requestPath='apps/desktop/g3-native-review-request.record15.json',requestSha='15b27d34a5169b3df489694c880dc68aee917ff503a98e44288579acaad5f665',frozen='spikes/noita-entry/.build/g3-candidates/nt2-g3-cp14-20261006-182234-77f547f9';
const checked=async p=>{const bytes=await readFile(p.path);if(sha(bytes)!==p.sha256)throw Error('Independent pin drift: '+p.path);return bytes;};
const m=JSON.parse(await checked({path:requestPath,sha256:requestSha}));
if(!m.frozen||m.frozenRoot!==frozen||m.pins.length!==195||new Set(m.pins.map(p=>p.path)).size!==195||m.approved!==false||m.installApproved!==false||m.packaged!==false||m.actualNoitaEvidence!==false||m.pins.some(p=>!p.path.startsWith(frozen+'/')||p.path.includes('/../')))throw Error('Request15 scope changed');
for(const p of m.pins)await checked(p);
for(const p of m.sourceProvenance)if(!m.pins.some(pin=>pin.path===p.frozenPath&&pin.sha256===p.sha256))throw Error('Frozen provenance differs');
await checked({path:frozen+'/review-request.json',sha256:requestSha});
const primaryPin={path:'deploy/profiles/g3-native-runtime-approval.record2-complete.json',sha256:'64095b1f054f793088572dc5751ae578d91e65ec0f3e5f4e51e4eaa853faea72'},runtimePin={path:'deploy/profiles/g3-native-social-runtime-approval.record1.json',sha256:'f1e4c251d0feb5caea64f645f8b60f74040744857831a89414202c8d17d1357e'};
const primary=JSON.parse(await checked(primaryPin)),runtime=JSON.parse(await checked(runtimePin));
if(JSON.stringify(m.requestedCapabilities)!==JSON.stringify(primary.compatibility.capabilities)||m.gameVersion!==primary.compatibility.gameVersion||m.modVersion!==primary.compatibility.modVersion||m.bridgeVersion!==primary.compatibility.bridgeVersion||m.schemaSha256!==primary.schemaSha256||m.schemaRevision!==primary.gameplayRevision||m.requestedBuildPairs.length!==2)throw Error('Primary metadata changed');
for(const pair of m.requestedBuildPairs)if(!primary.profile.supportedBuilds.some(p=>p.id===pair.id&&p.store===pair.store&&p.gameExeSha256===pair.gameExeSha256&&p.originalLuaSha256===pair.originalLuaSha256))throw Error('Original build pair changed');
if(!Array.isArray(m.additionalUnapprovedCandidates)||m.additionalUnapprovedCandidates.length!==1)throw Error('Unexpected supplemental candidates');
const social=m.additionalUnapprovedCandidates[0];
if(social.modVersion!==runtime.compatibility.modVersion||social.bridgeVersion!==runtime.compatibility.bridgeVersion||social.pipePrefix!==runtime.pipePrefix||social.installApproved!==false||social.approved!==false||social.productionCapabilities.length)throw Error('Supplemental scope changed');
const oldRoot='spikes/noita-entry/.build/g3-candidates/nt2-g3-cp13-20261006-172633-5e8ca466';
if(runtime.requiredLocalFiles.length!==20||runtime.runtimeMappings.length!==20||runtime.provenance.pins.length!==31)throw Error('Runtime closure changed');
for(const p of runtime.provenance.pins)await checked(p);
for(const mapping of runtime.runtimeMappings){
 if(!mapping.sourcePath.startsWith(oldRoot+'/'))throw Error('Runtime source origin changed');
 const mirror=frozen+mapping.sourcePath.slice(oldRoot.length),pin=m.pins.find(p=>p.path===mirror);
 if(!pin||pin.sha256!==mapping.sha256||!runtime.requiredLocalFiles.some(p=>p.path===mapping.path&&p.sha256===mapping.sha256))throw Error('Request15 changed runtime local bytes: '+mapping.path);
}
if(social.nativeArtifact!==frozen+'/spikes/noita-entry/.build/g3-social/nt2_g3_social.dll'||social.entrySource!==frozen+'/mod/noita_together2/init-g3-social.lua')throw Error('Wrong social native/entry selection');
const source=await readFile(frozen+'/apps/desktop/src-tauri/src/g3_social_installation.rs','utf8');
for(const predicate of ['approval["sourceApproved"]!=true','approval["developmentOnly"]!=true','approval["installApproved"]!=false','approval["packaged"]!=false','approval["actualNoitaEvidence"]!=false',runtimePin.sha256])if(!source.includes(predicate))throw Error('Development verifier policy missing: '+predicate);
const selected=m.pins.find(p=>p.path===social.candidateExecutable),base=m.pins.find(p=>p.path===m.candidateExecutable);
if(!selected||!base||selected.path!==frozen+'/apps/desktop/src-tauri/target/debug/examples/g3_social_candidate.exe'||selected.path===base.path||selected.sha256===base.sha256)throw Error('Separate frozen social test executable missing');
for(const p of [selected,base]){const bytes=await checked(p);if(bytes.toString('ascii',0,2)!=='MZ'||bytes.toString('ascii',bytes.readUInt32LE(60),bytes.readUInt32LE(60)+4)!=='PE\u0000\u0000')throw Error('Candidate is not a PE executable');}
const kitReadme=await readFile(frozen+'/apps/desktop/G3-test-kit-README.md','utf8'),packager=await readFile(frozen+'/apps/desktop/tools/package-g3-test.ps1','utf8'),core=await readFile(frozen+'/apps/desktop/src-tauri/src/next_client.rs','utf8');
if(!kitReadme.includes('Preferred port is4356')||!kitReadme.includes('-Port 4357 -Check')||!kitReadme.includes('Prepare protected worlds')||kitReadme.includes('-Port4357')||!packager.includes("origin='https://192.168.0.104:4356'")||!core.includes('if self.social_requested&&self.social_enabled{vec!["core.seed".into(),"core.ghost".into(),"core.hud".into(),"core.poi".into()]}'))throw Error('Final host instructions or actual source room capabilities changed');
const output={revision:'nt2-g3-desktop-review-15',status:'source-reviewed-development-only',installApproved:false,packaged:false,actualNoitaEvidence:false,runtimeApproval:{...primaryPin,runtimeId:primary.id},supplementalRuntimeApproval:{...runtimePin,runtimeId:runtime.id,capabilities:runtime.compatibility.capabilities},manifest:{path:requestPath,sha256:requestSha},snapshotRequest:{path:frozen+'/review-request.json',sha256:requestSha},frozenRoot:frozen,selectedTestExecutable:selected,primaryTestExecutable:base,pins:m.pins,sourceProvenance:m.sourceProvenance,framing:m.framing,limitations:['Exact request15/195 frozen pins independently verified; separate opt-in development launcher is the selected test executable','Runtime1 retains its original request10/31 provenance pins; all20 local bytes also match request15 exactly; no runtime/self-hash/identity churn','Compiled development-source policy preserves installApproved/packaged/actualNoitaEvidence=false; production verifier remains closed','Uninstalled package/manual human test preparation only; no actual Noita/upstream HUD/POI/two-PC/save/crash acceptance','No emote/cosmetic/gold/hourglass/Nemesis grant; primary resource/default desktop9 unchanged; future changed snapshot needs review']};
const path='deploy/profiles/g3-native-desktop-review.record15.json',bytes=Buffer.from(JSON.stringify(output,null,2)+'\n'),old=await readFile(path).catch(()=>null);
if(old&&!old.equals(bytes))throw Error('Immutable desktop12 review would change');if(!old)await writeFile(path,bytes,{flag:'wx'});
console.log(path+' '+sha(bytes));console.log('195 independently verified frozen pins; exact20 runtime mirror and separate selected social executable '+selected.sha256+'; source flags remain false for installation/game acceptance.');

