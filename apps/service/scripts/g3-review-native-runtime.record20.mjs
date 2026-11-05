// Exact frozen source/native review only; no builds, DLL loading or network operations.
import fs from 'node:fs';import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex'),demand=(v,m)=>{if(!v)throw Error(m);};
const checked=p=>{const b=fs.readFileSync(p.path);demand(sha(b)===p.sha256,'Pin drift: '+p.path);return b;};
const request={path:'apps/desktop/g3-native-review-request.record20.json',sha256:'c8e7cd4ca44f4839d699279948490fa66ffc3772723f70130cab54c8098ee1d4'};
const m=JSON.parse(checked(request)),root='spikes/noita-entry/.build/g3-candidates/nt2-g3-cp18-20261007-233152-0798e3d6';
demand(m.frozen&&m.frozenRoot===root&&m.pins.length===215&&m.sourceProvenance.length===215,'Frozen scope');
demand(m.candidateExecutable===null&&m.additionalUnapprovedCandidates.length===1&&m.additionalUnapprovedCandidates[0].candidateExecutable===null,'Source/native only');
demand(m.approved===false&&m.installApproved===false&&m.packaged===false&&m.actualNoitaEvidence===false,'Request policy');
demand(new Set(m.pins.map(p=>p.path)).size===215,'Duplicate pins');
for(const p of m.pins){demand(p.path.startsWith(root+'/')&&!p.path.split('/').includes('..'),'Unsafe frozen path');checked(p);}
for(const p of m.sourceProvenance)demand(m.pins.some(x=>x.path===p.frozenPath&&x.sha256===p.sha256),'Frozen provenance missing');
checked({path:root+'/review-request.json',sha256:request.sha256});
const frozen=path=>{const p=m.sourceProvenance.find(p=>p.sourcePath===path);demand(p,'Missing frozen dependency '+path);return {path:p.frozenPath,sha256:p.sha256};};
const text=path=>checked(frozen(path)).toString('utf8');
const build=JSON.parse(text('apps/desktop/g3-client-native-build-check-results.record20-final.json'));
demand(build.sourceOnly&&build.usableCandidate===false&&build.checks.builds===2&&build.checks.exportsExactOldAbi&&build.checks.privateFlagsAndTraceAbsent,'Native evidence');
for(const p of [...build.sourcePins,...build.evidencePins,...build.acceptedEvidence])demand(frozen(p.path).sha256===p.sha256,'Frozen build evidence mismatch');
for(const path of ['mod/noita_together2/init-g3.lua','mod/noita_together2/init-g3-social.lua'])demand(text(path).includes('restore();released=true;log("released",seed);observe()'),'Immediate playing report');
const base=text('native/g2-bridge/bridge.cpp');
for(const guard of ['NT2_G3_VERIFIED_HEARTBEAT_TIMEOUT','NT2_G3_BOOTSTRAP_WAIT','if(op==1||op==3||op==4)bootstrap_wait=250;','if(!timed_out)InterlockedExchange(&state->status,0);','if(!app(pipe))','memcmp(reply+16,context,16)','CancelIoEx'])demand(base.includes(guard),'Native guard '+guard);
for(const path of m.sourceProvenance.filter(p=>p.sourcePath.endsWith('/compile.cmd')).map(p=>p.sourcePath))demand(!/\/D|NT2_TEST|INITIAL_TRACE/.test(text(path)),'Private compile flag');
const dlls=[m.nativeArtifact,m.additionalUnapprovedCandidates[0].nativeArtifact];
const hashes=['c7118effab4b0c1d5361a4d23dc64386d0ce4eb1fc3eeceafed754b28fa4b447','cad4e5aab13efb06c917fcfeff9b956f3159ddf9add7ab64943ef2ec03d87978'];
const binaryChecks=dlls.map((path,i)=>{
 const pin=m.pins.find(p=>p.path===path);demand(pin&&pin.sha256===hashes[i],'Native identity');const b=checked(pin),pe=b.readUInt32LE(60);
 demand(b.toString('ascii',0,2)==='MZ'&&b.toString('ascii',pe,pe+4)==='PE\0\0'&&b.readUInt16LE(pe+4)===0x14c&&(b.readUInt16LE(pe+22)&0x2000)&&b.readUInt16LE(pe+24)===0x10b,'PE32 x86 DLL');
 // Independently read export names from the exact PE instead of trusting dump text.
 const sections=pe+24+b.readUInt16LE(pe+20),count=b.readUInt16LE(pe+6);
 const offset=rva=>{for(let n=0;n<count;n++){const s=sections+n*40,va=b.readUInt32LE(s+12),size=Math.max(b.readUInt32LE(s+8),b.readUInt32LE(s+16));if(rva>=va&&rva<va+size)return b.readUInt32LE(s+20)+rva-va;}throw Error('Unmapped RVA');};
 const exp=offset(b.readUInt32LE(pe+24+96)),names=offset(b.readUInt32LE(exp+32)),exports=[];
 for(let n=0;n<b.readUInt32LE(exp+24);n++){const at=offset(b.readUInt32LE(names+n*4));exports.push(b.toString('ascii',at,b.indexOf(0,at)));}
 demand(JSON.stringify(exports.sort())===JSON.stringify([...build.nativeArtifacts[i].exports].sort()),'Export ABI');
 demand(!b.includes(Buffer.from('NT2_PRIVATE_CONTACT'))&&!b.includes(Buffer.from('NT2_INITIAL_CONNECT_RACE')),'Private diagnostics included');
 return {...pin,machine:'I386',format:'PE32',exports,loaded:false};
});
const oldResources=[
 {path:'deploy/profiles/g3-native-runtime-approval.record3-a1.json',sha256:'959fddf3a0d5948709e685fdfda325de96004d98b798e377b70f65b5ad8502dd'},
 {path:'deploy/profiles/g3-native-social-runtime-approval.record2-a1.json',sha256:'b0ce032108612ff1d2540008ae9a96f746748243857d5d6efaa8537b7be934c0'}
];
const outputPaths=['deploy/profiles/g3-native-runtime-approval.record4-a1-runtime-fix.json','deploy/profiles/g3-native-social-runtime-approval.record3-a1-runtime-fix.json'];
const runtimeApprovals=[];
for(let i=0;i<2;i++){
 const r=JSON.parse(checked(oldResources[i]));r.revision='nt2-g3-a1-runtime-fix-source-review-1';r.id=i?'nt2-g3-native-social-development-3':'nt2-g3-native-development-4';
 r.runtimeMappings=r.runtimeMappings.map(p=>{const pin=p.path.endsWith('.dll')?binaryChecks[i]:frozen(p.sourcePath.slice(r.provenance.frozenRoot.length+1));return {path:p.path,sourcePath:pin.path,sha256:pin.sha256};});
 r.requiredLocalFiles=r.runtimeMappings.map(p=>({path:p.path,sha256:p.sha256}));
 const entry=r.runtimeMappings.find(p=>p.path==='mods/noita_together2/init.lua');
 for(const p of [r.profile,r.buildRegistry]){p.id=r.id;p.bridgeSha256=binaryChecks[i].sha256;p.modSha256=entry.sha256;}
 r.provenance={frozenRoot:root,pins:[...new Map([...r.runtimeMappings.map(p=>({path:p.sourcePath,sha256:p.sha256})),...m.sourceProvenance.filter(p=>p.sourcePath.startsWith('native/')||p.sourcePath.includes('g3-runtime-fix-production-')||build.acceptedEvidence.some(x=>x.path===p.sourcePath)||p.sourcePath==='apps/desktop/g3-client-native-build-check-results.record20-final.json').map(p=>({path:p.frozenPath,sha256:p.sha256}))].map(p=>[p.path,p])).values()]};
 r.limitations=r.limitations.map(s=>s.replace('Request17 desktop bytes still consume older runtime pins','Request20 contains no matching desktop executable'));
 demand(r.requiredLocalFiles.length===(i?20:14),'Local file count');
 // Every retained closure edge still resolves to one of the exact local files.
 for(const edge of r.closure.references)demand(r.requiredLocalFiles.some(p=>p.path===edge.from)&&r.requiredLocalFiles.some(p=>p.path===edge.to),'Closure edge');
 fs.writeFileSync(outputPaths[i],JSON.stringify(r,null,2)+'\n',{flag:'wx'});
 runtimeApprovals.push({path:outputPaths[i],sha256:sha(fs.readFileSync(outputPaths[i])),runtimeId:r.id,capabilities:r.compatibility.capabilities});
}
const review={revision:'nt2-g3-a1-request20-source-review-1',activeGate:'G3A/A1',sourceApproved:true,developmentOnly:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,usableCandidate:false,consumerRebuildRequired:true,finalRebuiltDesktopReviewRequired:true,frozenRoot:root,snapshotRequest:request,runtimeApprovals,pins:m.pins,sourceProvenance:m.sourceProvenance,binaryChecks,retainedResources:oldResources,identityDecision:'New runtime profile IDs; unchanged A1 service IDs/prefixes and wire/authority/schema/versions. Existing owner4357 namespace/history retained; no migration or service action.',reviewedBehavior:['G3-only initial op1/3/4 pre-write acquisition and reply budget250ms; writes80ms; no transmitted request replay','Verified heartbeat I/O timeout retains only existing cached status/age; negative/identity/malformed/other failure clears;12s age unchanged','Both Lua entries publish playing observation immediately on protection release','Root-reviewed same-world phase permit continuity and social lane isolation; genuine scope/content/terminal fences retained'],limitations:['Exact source/native bytes only. Both candidateExecutable fields null; no usable/new/future app approval.','Production-prefix DLLs statically inspected, never loaded. Retained root42 private-native cases and injected engine native/TLS evidence are not actual Noita acceptance.','Original source fixture initial failure attribution remains unproven. Actual ghosts/two-PC stability/HUD/A2 and later gates remain pending.']};
const out='deploy/profiles/g3-native-a1-source-review.record20.json';fs.writeFileSync(out,JSON.stringify(review,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({review:{path:out,sha256:sha(fs.readFileSync(out))},runtimeApprovals,pins:m.pins.length,binaryChecks}));
