// Exact request3 source review. No build, installation or runtime capability grant.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const root=new URL('../../../',import.meta.url),sha=b=>createHash('sha256').update(b).digest('hex');
const legacyPath='deploy/profiles/g3-native-development-candidate.json',legacyBytes=await readFile(new URL(legacyPath,root));
if(sha(legacyBytes)!=='4f59fe2f8d96736da1a0a678a2c8d330d600946c0d88b296f09f1318416e9faf')throw Error('Original runtime review changed');
const original=JSON.parse(legacyBytes),requestPath='apps/desktop/g3-native-review-request.record3.json',requestBytes=await readFile(new URL(requestPath,root));
if(sha(requestBytes)!=='82fc0160248f63d1903732daaad3d8ac956ae87e840ed890fb5ff92b9cbfd215')throw Error('Request3 changed');
const request=JSON.parse(requestBytes),frozen='spikes/noita-entry/.build/g3-candidates/nt2-g3-cp7-20261006-130304-853ad4a7';
if(!request.frozen||request.frozenRoot!==frozen||request.pins.length!==74||new Set(request.pins.map(p=>p.path)).size!==74||request.pins.some(p=>!p.path.startsWith(frozen+'/')||p.path.includes('/../')))throw Error('Request3 closure changed');
const snapshotPath=frozen+'/review-request.json';
if(sha(await readFile(new URL(snapshotPath,root)))!==sha(requestBytes))throw Error('Snapshot request changed');
for(const pin of request.pins)if(sha(await readFile(new URL(pin.path,root)))!==pin.sha256)throw Error('Desktop dependency changed: '+pin.path);
for(const source of request.sourceProvenance)if(!request.pins.some(p=>p.path===source.frozenPath&&p.sha256===source.sha256))throw Error('Provenance differs');
if(request.actualNoitaEvidence||request.installApproved||request.packaged||JSON.stringify(request.requestedCapabilities)!==JSON.stringify(original.compatibility.capabilities))throw Error('Request3 approval scope changed');
if(request.gameVersion!==original.compatibility.gameVersion||request.modVersion!==original.compatibility.modVersion||request.bridgeVersion!==original.compatibility.bridgeVersion||request.schemaRevision!==original.gameplayRevision||request.schemaSha256!==original.schemaSha256)throw Error('Runtime metadata changed');
for(const pair of request.requestedBuildPairs)if(!original.profile.supportedBuilds.some(p=>p.id===pair.id&&p.store===pair.store&&p.gameExeSha256===pair.gameExeSha256&&p.originalLuaSha256===pair.originalLuaSha256))throw Error('Runtime pair changed');
if(request.requestedBuildPairs.length!==original.profile.supportedBuilds.length)throw Error('Runtime pairs differ');
// Conservatively retain ALL previously reviewed mod assets, including unloaded hooks,
// and native implementation/build sources. New unloaded files are desktop provenance only.
const runtimePins=original.provenance.pins.filter(p=>p.path.includes('/mod/noita_together2/')||p.path.includes('/native/')||p.path.endsWith('/nt2_g3_transport.dll'));
for(const pin of runtimePins){
 if(sha(await readFile(new URL(pin.path,root)))!==pin.sha256)throw Error('Runtime dependency changed: '+pin.path);
 const relative=pin.path.slice(original.provenance.frozenRoot.length+1);
 if(!request.pins.some(p=>p.path===frozen+'/'+relative&&p.sha256===pin.sha256))throw Error('Request3 changes approved runtime: '+relative);
}
const loggingPath='deploy/profiles/g3-runtime-logging.record2.xml',logging=await readFile(new URL('mod/noita_together2/files/logging.xml',root));
if(sha(logging)!=='b13a3dcd127a921f081f80f24cc369618d008efb7de53784f7e82860be05ebac')throw Error('Logging asset changed');
const loggingExisting=await readFile(new URL(loggingPath,root)).catch(()=>null);
if(loggingExisting&&!loggingExisting.equals(logging))throw Error('Logging snapshot changed');
if(!loggingExisting)await writeFile(new URL(loggingPath,root),logging,{flag:'wx'});
runtimePins.push({path:loggingPath,sha256:sha(logging)});
const requiredLocalFiles=runtimePins.flatMap(p=>p.path===loggingPath?[{path:'mods/noita_together2/files/logging.xml',sha256:p.sha256}]:p.path.endsWith('/nt2_g3_transport.dll')?[{path:'mods/noita_together2/files/nt2_g3_transport.dll',sha256:p.sha256}]:p.path.includes('/mod/noita_together2/')?[{path:'mods/noita_together2/'+p.path.split('/mod/noita_together2/')[1].replace(/^init-g3.lua$/,'init.lua'),sha256:p.sha256}]:[]);
const runtimePath='deploy/profiles/g3-native-runtime-approval.record2-complete.json';
const runtime={revision:'nt2-g3-runtime-approval-2-complete',...Object.fromEntries(Object.entries(original).filter(([key])=>key!=='provenance')),requiredLocalFiles,provenance:{originalReview:{path:legacyPath,sha256:sha(legacyBytes)},pins:runtimePins,supplementalAsset:{source:'mod/noita_together2/files/logging.xml',path:loggingPath,sha256:sha(logging),reason:'Entry ModMagicNumbersFileAdd dependency absent from request3; exact server-owned snapshot, not an installed asset change'}},policy:'Immutable exact runtime approval. Desktop provenance is independently reviewed. Additional unloaded modules confer no capability. No installation or actual engine approval.'};
const runtimeBytes=Buffer.from(JSON.stringify(runtime,null,2)+'\n');
const desktop={revision:'nt2-g3-desktop-review-3',runtimeApproval:{path:runtimePath,sha256:sha(runtimeBytes),runtimeId:original.id},manifest:{path:requestPath,sha256:sha(requestBytes)},snapshotRequest:{path:snapshotPath,sha256:sha(requestBytes)},frozenRoot:frozen,pins:request.pins,sourceProvenance:request.sourceProvenance,framing:request.framing,review:'docs/g3/coordinator-review-20261006-overnight6.md',status:'source-reviewed-development-only',installApproved:false,actualNoitaEvidence:false,limitations:['R07/R08 in unloaded HUD/POI hooks remain client work; no runtime social grant','Frozen desktop is an independent reviewed artifact, never a compiled self-executable digest requirement']};
for(const [path,bytes] of [[runtimePath,runtimeBytes],['deploy/profiles/g3-native-desktop-review.record3-complete.json',Buffer.from(JSON.stringify(desktop,null,2)+'\n')]]){
 const existing=await readFile(new URL(path,root)).catch(()=>null);
 if(existing&&!existing.equals(bytes))throw Error('Immutable review would be replaced: '+path);
 if(!existing)await writeFile(new URL(path,root),bytes,{flag:'wx'});
 console.log(path+' '+sha(bytes));
}
console.log('Exact request3: 74 pins; 17 unchanged runtime pins + 1 supplemental logging pin');
