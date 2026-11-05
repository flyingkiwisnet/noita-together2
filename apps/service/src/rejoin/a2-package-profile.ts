// Matching normal A2 package. Historical recovery20 resources stay immutable.
import {readFile,writeFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
import {inspectNativeExports,safeArtifactPath,type ArtifactPin} from './composition-profile.js';
import {validateBuildRegistry} from '../g2-next/build-registry.js';
export const rejoinPackageInput={path:'apps/desktop/a2-package-inputs-20261022.json',sha256:'e1817b1a3381e90ab944907dd448b33083e901b02c0b13e83d967842f28920c3'};
export const rejoinRuntimePath='deploy/profiles/g3-a2-runtime.20261022.json',rejoinSourcePath='deploy/profiles/g3-a2-source.20261022.json';
const root=new URL('../../../../',import.meta.url),sha=(b:Buffer|string)=>createHash('sha256').update(b).digest('hex');
const demand=(v:unknown,m:string)=>{if(!v)throw Error('A2 package: '+m);};
const additions=['a2_text.lua','a2_capture.lua','a2_ui_model.lua','a2_biomes.lua','a2_input_lock.lua','a2_renderer.lua','a2_native_input.lua','poi_proximity.lua','ui/keyboard.png','ui/player_ghosts.png','ui/hide_player_ghosts.png','ui/chat.png','ui/hide_chat.png','ui/player_list.png','ui/radar.png','ui/poi_emitter.png'];
export async function rejoinPackageDefinition(){
 const checked=async(p:ArtifactPin)=>{safeArtifactPath(p.path);demand(/^[a-f0-9]{64}$/.test(p.sha256),'hash');const b=await readFile(new URL(p.path,root));demand(sha(b)===p.sha256,'changed '+p.path);return b;};
 const c=JSON.parse((await checked(rejoinPackageInput)).toString());
 const acceptedFlow={path:'spikes/noita-entry/.build/a2-client-20261021-20/client-inputs.json',sha256:'e1f75f52bd103c88f549cb305c201d12b9525c744b2da7e017c74869e7896f6c'},accepted=JSON.parse((await checked(acceptedFlow)).toString());
 const acceptedLog={path:'spikes/noita-entry/.build/a2-client-20261021-20/integration-a2_ordinary_scheduler_social_composition-20261021-081658.txt',sha256:'2deee1c97a055e095738e52459344facf5d7f4d6f4ec248e3dfd927b506843df'};demand((await checked(acceptedLog)).toString().includes('test result: ok. 1 passed; 0 failed;'),'accepted A2 composition');
 const reviewed=JSON.parse((await checked(accepted.runtime)).toString());demand(reviewed.runtimeMappings.length===66,'reviewed A2 closure');
 demand(c.revision==='g3-lifecycle-lc1-package-client-inputs-1'&&c.canonicalWaitingRevision==='g3-waiting-entry-candidate-3'&&c.acceptedFlow?.path===acceptedFlow.path&&c.acceptedFlow?.sha256===acceptedFlow.sha256,'accepted A2 normal package');
 demand(c.nativeAbi===2&&c.brokerApi===3&&c.startupWireRevision===9&&c.fixtureOnly===false&&c.normalDllsBuilt===true&&c.published===false&&c.actualNoitaEvidence===false,'normal A2 input');
 demand(c.canonicalLifecycle?.path==='packages/contracts/g3-lifecycle/schema.json'&&c.canonicalRejoinSchema?.path==='packages/contracts/g3-rejoin/schema.json','canonical lifecycle/rejoin');await checked(c.canonicalLifecycle);await checked(c.canonicalRejoinSchema);
 const basePin={path:'deploy/profiles/g3-lc1-recovery-runtime.20261020.json',sha256:'9cca0acfcbde924ab4ea124570abe7bf82cf1d614b9268e2877b7b00616c42e7'},base=JSON.parse((await checked(basePin)).toString());
 demand(c.runtimeFileCount===66&&c.runtimeMappings?.length===66&&c.sourcePins?.length>=8&&c.sourcePins.length<=256,'exact66-file closure');
 const sources=new Map<string,string>();for(const p of c.sourcePins){demand(!sources.has(p.path),'duplicate source');sources.set(p.path,p.sha256);await checked(p);}
 for(const p of ['native/g3-ipc/broker.cpp','native/g3-ipc/wrapper.cpp','native/g3-ipc/abi.h','native/g3-ipc/build.ps1'])demand(sources.has(p),'native provenance '+p);
 const mapped=new Map<string,any>();for(const m of c.runtimeMappings){safeArtifactPath(m.path);safeArtifactPath(m.sourcePath);demand(m.path.startsWith('mods/noita_together2/')&&!mapped.has(m.path),'mapping');mapped.set(m.path,m);await checked({path:m.sourcePath,sha256:m.sha256});}
 const allowed=new Set<string>(base.runtimeMappings.map((m:any)=>m.path));for(const p of allowed)demand(mapped.has(p),'retained runtime '+p);
 for(const p of additions){const sourcePath='mod/noita_together2/files/g3/'+p,dest='mods/noita_together2/files/g3/'+p;demand(mapped.get(dest)?.sourcePath===sourcePath,'fixed A2 mapping '+p);allowed.add(dest);}
 demand([...mapped.keys()].every(p=>allowed.has(p)),'unknown A2 dependency');
 for(const m of c.runtimeMappings)if(!m.path.endsWith('.dll'))demand(reviewed.runtimeMappings.some((r:any)=>r.path===m.path&&r.sha256===m.sha256),'reviewed A2 mod bytes '+m.path);
 const entry=mapped.get('mods/noita_together2/init.lua');demand(entry.sourcePath==='mod/noita_together2/init-g3-seed-loot.lua'&&sources.get(entry.sourcePath)===entry.sha256,'entry provenance');
 demand(c.normalArtifacts?.length===3&&new Set(c.normalArtifacts.map((a:any)=>a.mode)).size===3,'three normal images');
 await checked(c.normalNativeBuildLog);
 const normal=[];for(const a of c.normalArtifacts){
  const old=base.nativeArtifacts.find((x:any)=>x.mode===a.mode);demand(old&&a.private===false&&a.testFlags===false&&a.path.startsWith('spikes/noita-entry/.build/'),'normal image');
  const command=(await checked(a.buildCommand)).toString();demand(command.includes('/Brepro')&&command.includes('-arch=x86')&&command.includes('broker.cpp')&&command.includes('wrapper.cpp')&&!/\/DNT2_(IPC_PRIVATE|TEST_|PIPE_PREFIX)/.test(command),'normal reproducible build');
  const exports=inspectNativeExports(await checked(a)),expected=[...old.exports,...(a.mode==='broker'?[]:['nt2_g3_social_publish'])].sort();demand(JSON.stringify(exports)===JSON.stringify(expected)&&JSON.stringify([...a.exports].sort())===JSON.stringify(exports),'exact A2 exports '+a.mode);
  demand(c.runtimeMappings.some((m:any)=>m.sourcePath===a.path&&m.sha256===a.sha256),'native mapping');normal.push({...a,exports,loaded:false,installed:false});
 }
 for(const m of c.runtimeMappings)if(m.path.endsWith('.dll'))demand(normal.some(a=>a.path===m.sourcePath&&a.sha256===m.sha256),'foreign image');
 const id='nt2-g3-a2-dev-20261022',registry={...base.buildRegistry,id,modSha256:entry.sha256,bridgeSha256:normal.find(a=>a.mode==='primary')!.sha256};validateBuildRegistry(registry);
 const socialPath='packages/contracts/g3-social/schema.json',socialBytes=await readFile(new URL(socialPath,root));demand(sources.get(socialPath)===sha(socialBytes),'canonical social provenance');
 const runtime={...base,id,revision:'nt2-g3-a2-runtime-20261022',status:'reviewed-A2-development-engine-evidence-pending',socialSchemaSha256:sha(socialBytes),buildRegistry:registry,profile:{...base.profile,...registry},compositionInput:rejoinPackageInput,lifecycle:{...base.lifecycle,acceptedFlow,schema:c.canonicalLifecycle},requiredLocalFiles:c.runtimeMappings.map(({path,sha256}:any)=>({path,sha256})),runtimeMappings:c.runtimeMappings,nativeArtifacts:normal,provenance:{clientDependency:rejoinPackageInput,canonicalLifecycle:c.canonicalLifecycle,canonicalRejoinSchema:c.canonicalRejoinSchema,canonicalSocial:{path:socialPath,sha256:sha(socialBytes)},acceptedFlow,acceptedLog,pins:c.sourcePins},closure:{...base.closure,localDependencies:66},evidence:{...base.evidence,matchedCompositionPassed:true,actualNoitaA2:false},remainingEvidence:'Owner actual Noita two-PC A2 HUD/chat/POI/dismissal/Continue/outage/End smoke pending'};
 const bytes=JSON.stringify(runtime,null,2)+'\n',source={revision:'nt2-g3-a2-source-20261022',activeGate:'G3A/A2 player information and world messages',checkpoint:'matching-A2-package',sourceApproved:true,developmentOnly:true,testBuild:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,input:rejoinPackageInput,baseline:basePin,acceptedFlow,acceptedLog,runtimeApproval:{path:rejoinRuntimePath,sha256:sha(bytes)},pins:c.sourcePins,nativeArtifacts:normal,remaining:['Root final publication','Owner actual Noita A2 evidence']};
 return {runtime,bytes,source,sourceBytes:JSON.stringify(source,null,2)+'\n'};
}
export async function generateRejoinPackage(check=false){const d=await rejoinPackageDefinition();for(const [p,b]of [[rejoinRuntimePath,d.bytes],[rejoinSourcePath,d.sourceBytes]]){const u=new URL(p!,root);if(check)demand(await readFile(u,'utf8')===b,'resource drift');else try{await writeFile(u,b!,{flag:'wx'});}catch(e){if((e as NodeJS.ErrnoException).code!=='EEXIST'||await readFile(u,'utf8')!==b)throw e;}}return d;}
export async function verifyRejoinPackage(reader:(u:URL)=>Promise<Buffer>=u=>readFile(u)){const d=await rejoinPackageDefinition();for(const [p,b]of [[rejoinRuntimePath,d.bytes],[rejoinSourcePath,d.sourceBytes]])demand((await reader(new URL(p!,root))).equals(Buffer.from(b!)),'resource drift');return {...d.runtime,runtimeResourceJson:d.bytes,runtimeApproval:{path:rejoinRuntimePath,sha256:sha(d.bytes)},desktopApproval:{path:rejoinSourcePath,sha256:sha(d.sourceBytes)}};}
