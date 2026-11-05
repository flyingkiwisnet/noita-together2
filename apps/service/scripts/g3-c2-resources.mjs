// Match accepted C2 client inputs. Reads/hashes normal artifacts; never loads DLLs.
import fs from 'node:fs';import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex'),demand=(v,m)=>{if(!v)throw Error(m);};
const input={path:'apps/desktop/g3-client-c2-native-dependency.json',sha256:'81c71bfcba6ee8f5ee4e866c66bbe91a4acd1dce6c80cc4a3b2d4dc38d65abad'};
const registry={path:'deploy/profiles/g3-native-social-runtime-approval.record5-a1-heartbeat-busy.json',sha256:'758b597a0deace517b4891fe1c6460ed55696e736aa0c03123bc5f1139595217'};
const read=p=>{demand(p.path&&!p.path.includes('..')&&!/[\\:]/.test(p.path)&&!p.path.startsWith('/'),'Unsafe dependency');const b=fs.readFileSync(p.path);demand(sha(b)===p.sha256,'Input changed: '+p.path);return b;};
const client=JSON.parse(read(input)),old=JSON.parse(read(registry));
demand(client.activeGate==='G3A/A1'&&client.nativeArtifacts.length===3&&client.runtimeInputs.length===22&&client.sourcePins.length===6,'C2 input shape');
demand(JSON.stringify(client.compatibilityInputs.capabilities)===JSON.stringify(['core.seed','core.ghost','core.hud','core.poi'])&&client.compatibilityInputs.modVersion==='nt2-g3-c2-dev-1'&&client.compatibilityInputs.bridgeVersion==='g3-c2-ipc-1','C2 semantic inputs');
const pins=new Map();for(const p of [...client.sourcePins,...client.runtimeInputs.map(p=>({path:p.sourcePath,sha256:p.sha256}))]){read(p);if(pins.has(p.path))demand(pins.get(p.path).sha256===p.sha256,'Conflicting pin');pins.set(p.path,p);}
const binaryChecks=client.nativeArtifacts.map(p=>{
 demand(p.testFlags===false&&p.loaded===false&&p.installed===false&&p.path.startsWith('spikes/noita-entry/.build/g3-connection-c2-20261010-01/production-05/'),'Normal uninstalled native artifact required');
 const b=read(p),pe=b.readUInt32LE(60);demand(b.toString('ascii',0,2)==='MZ'&&b.toString('ascii',pe,pe+4)==='PE\0\0'&&b.readUInt16LE(pe+4)===0x14c&&(b.readUInt16LE(pe+22)&0x2000)&&b.readUInt16LE(pe+24)===0x10b,'PE32 I386 DLL required');
 const sections=pe+24+b.readUInt16LE(pe+20),count=b.readUInt16LE(pe+6),offset=rva=>{for(let n=0;n<count;n++){const s=sections+n*40,va=b.readUInt32LE(s+12),size=Math.max(b.readUInt32LE(s+8),b.readUInt32LE(s+16));if(rva>=va&&rva<va+size)return b.readUInt32LE(s+20)+rva-va;}throw Error('Unmapped export RVA');};
 const exp=offset(b.readUInt32LE(pe+24+96)),names=offset(b.readUInt32LE(exp+32)),exports=[];for(let n=0;n<b.readUInt32LE(exp+24);n++){const at=offset(b.readUInt32LE(names+n*4));exports.push(b.toString('ascii',at,b.indexOf(0,at)));}demand(JSON.stringify(exports.sort())===JSON.stringify([...p.exports].sort()),'Exact native export ABI');
 return {...p,exports,sourceApproved:true,developmentOnly:true,installApproved:false};
});
const id='nt2-g3-native-c2-development-1',runtimePath='deploy/profiles/g3-native-c2-runtime-approval.record1.json',dependencyPath='deploy/profiles/g3-native-c2-source-dependency.record1.json';
const buildRegistry={...old.buildRegistry,id,bridgeSha256:client.nativeArtifacts.find(p=>p.mode==='primary').sha256,modSha256:client.runtimeInputs.find(p=>p.path==='mods/noita_together2/init.lua').sha256};
const profile={...buildRegistry,gameVersion:old.compatibility.gameVersion,modVersion:client.compatibilityInputs.modVersion,bridgeVersion:client.compatibilityInputs.bridgeVersion,capabilities:client.compatibilityInputs.capabilities};
const mappings=client.runtimeInputs.map(p=>({path:p.path,sourcePath:p.sourcePath,sha256:p.sha256}));demand(new Set(mappings.map(p=>p.path)).size===22,'Unique runtime closure');
const resource={revision:'nt2-g3-c2-test-runtime-1',id,status:'development-test-runtime',sourceApproved:true,developmentOnly:true,testBuild:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,
 serviceId:'nt2-g3-native-c2-development',namespacePrefix:'g3_c2_',pipePrefix:client.selectors.pipePrefix,configurationDirectory:client.selectors.configurationDirectory,cargoFeature:client.selectors.cargoFeature,
 lobbyRevision:old.lobbyRevision,gameplayRevision:old.gameplayRevision,schemaSha256:old.schemaSha256,socialRevision:old.socialRevision,socialSchemaSha256:old.socialSchemaSha256,replayPolicy:old.replayPolicy,
 live:{enabled:true,endpoint:'/v3/live',subprotocol:'nt2.g3.live.v1',revision:'g3-live-draft-1',encoding:'protobuf-binary',schemaSha256:sha(fs.readFileSync('packages/contracts/g3-live/schema.json'))},
 compatibility:{...old.compatibility,modVersion:client.compatibilityInputs.modVersion,bridgeVersion:client.compatibilityInputs.bridgeVersion},buildRegistry,profile,
 requiredLocalFiles:mappings.map(({path,sha256})=>({path,sha256})),runtimeMappings:mappings,nativeArtifacts:binaryChecks,
 localProtocol:{brokerAbiVersion:1,controlVersion:4,reportVersion:5,replyBytes:48,startupWaitMaximumMs:250,lifecycleInspectionJobBudgetMs:250,heartbeatMovementSocialJobBudgetMs:80,leaseOrigin:'remaining original Core permission/preparation deadline; delivery re-fenced',recurringExports:'nonblocking; shared broker alone owns pipe I/O'},
 closure:{localDependencies:22,entry:'mods/noita_together2/init.lua',broker:'mods/noita_together2/files/nt2_g3_ipc.dll',primary:'mods/noita_together2/files/nt2_g3_transport.dll',social:'mods/noita_together2/files/nt2_g3_social.dll',allRequiredTogether:true},
 provenance:{clientDependency:input,originalBuildRegistry:registry,pins:[...pins.values()]},evidence:{nativeArtifacts:'normal production-05 compilation, hashed/export-inspected only, never loaded',privateChecks:client.checks,coordinatorAcceptedCheckpoint:'C2-C',combinedCoreNativeWss:false,ownerNoitaTwoPc:false}};
const runtimeBytes=Buffer.from(JSON.stringify(resource,null,2)+'\n');
const dependency={revision:'nt2-g3-c3-matching-source-dependency-1',activeGate:'G3A/A1',checkpoint:'C3-S',status:'matching-development-runtime',sourceApproved:true,developmentOnly:true,testBuild:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,usableCandidate:false,consumerRebuildRequired:true,finalRebuiltDesktopReviewRequired:false,
 snapshotRequest:input,originalBuildRegistry:registry,runtimeApprovals:[{path:runtimePath,sha256:sha(runtimeBytes),runtimeId:id,capabilities:resource.compatibility.capabilities}],pins:[...pins.values()],nativeArtifacts:binaryChecks,remaining:['Client combined Core/native/WSS injected-engine proof and separate kit','Owner actual Noita/two-PC/save acceptance']};
for(const [path,bytes]of [[runtimePath,runtimeBytes],[dependencyPath,Buffer.from(JSON.stringify(dependency,null,2)+'\n')]]){
 if(process.argv.includes('--check'))demand(fs.readFileSync(path).equals(bytes),'Generated matching metadata differs: '+path);else fs.writeFileSync(path,bytes,{flag:process.argv.includes('--refresh')?'w':'wx'});
}
process.stdout.write(JSON.stringify({runtime:{path:runtimePath,sha256:sha(runtimeBytes),id},sourceDependency:{path:dependencyPath,sha256:sha(Buffer.from(JSON.stringify(dependency,null,2)+'\n'))},runtimeFiles:22,sourceAndRuntimePins:pins.size,nativeArtifacts:binaryChecks.map(p=>({mode:p.mode,sha256:p.sha256,exports:p.exports.length}))})+'\n');
