import {readFile,mkdir,writeFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
import {nativeLoot01RuntimeResource} from '../g3/native-loot01-profile.js';
import {validateBuildRegistry} from '../g2-next/build-registry.js';
export interface ArtifactPin {path:string;sha256:string}
export const rejoinWrapperExports=['nt2_g3_resume','nt2_g3_association','nt2_g3_saved_lookup','nt2_g3_offline_choice','nt2_g3_resume_decision'] as const;
export const rejoinInputRevision='g3-client-rejoin-composition-inputs-1';
const root=new URL('../../../../',import.meta.url),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
const demand=(v:unknown,m:string)=>{if(!v)throw Error('Rejoin composition: '+m);};
export function safeArtifactPath(path:string){demand(typeof path==='string'&&path.length<=512&&!path.startsWith('/')&&!/[\\:]/.test(path)&&path.split('/').every(p=>p&&p!=='.'&&p!=='..'),'unsafe artifact path');return path;}
// Structural PE inspection only. No normal/private image is ever loaded here.
export function inspectNativeExports(b:Buffer){
 demand(b.length>=256&&b.length<=8*1024*1024,'native image bounds');const pe=b.readUInt32LE(60);demand(pe+248<b.length&&b.toString('ascii',0,2)==='MZ'&&b.toString('ascii',pe,pe+4)==='PE\0\0'&&b.readUInt16LE(pe+4)===0x14c&&(b.readUInt16LE(pe+22)&0x2000)&&b.readUInt16LE(pe+24)===0x10b,'PE32 I386 DLL required');
 const sections=pe+24+b.readUInt16LE(pe+20),count=b.readUInt16LE(pe+6);demand(count>0&&count<=32&&sections+count*40<=b.length,'PE section bounds');
 const offset=(rva:number)=>{for(let n=0;n<count;n++){const s=sections+n*40,va=b.readUInt32LE(s+12),size=Math.max(b.readUInt32LE(s+8),b.readUInt32LE(s+16)),at=b.readUInt32LE(s+20)+rva-va;if(rva>=va&&rva<va+size&&at>=0&&at<b.length)return at;}throw Error('Rejoin composition: export RVA bounds');};
 const exp=offset(b.readUInt32LE(pe+24+96));demand(exp+40<=b.length,'export directory bounds');const names=offset(b.readUInt32LE(exp+32)),total=b.readUInt32LE(exp+24);demand(total>0&&total<=64&&names+total*4<=b.length,'export names bounds');
 const exports=[];for(let n=0;n<total;n++){const at=offset(b.readUInt32LE(names+n*4)),end=b.indexOf(0,at);demand(end>=at&&end-at<=128,'export name bounds');exports.push(b.toString('ascii',at,end));}demand(new Set(exports).size===exports.length,'duplicate export');return exports.sort();
}
export async function verifyRejoinCompositionInput(inputPin:ArtifactPin,reader:(url:URL)=>Promise<Buffer>=url=>readFile(url)){
 const checked=async(p:ArtifactPin)=>{safeArtifactPath(p.path);demand(/^[0-9a-f]{64}$/.test(p.sha256),'invalid hash');const b=await reader(new URL(p.path,root));demand(sha(b)===p.sha256,'changed '+p.path);return b;};
 demand(inputPin.path.startsWith('apps/desktop/')&&inputPin.path.endsWith('.json'),'owned Client input JSON required');const inputBytes=await checked(inputPin);demand(inputBytes.length<=131072,'input bounds');const c=JSON.parse(inputBytes.toString());
 demand(c.revision===rejoinInputRevision&&c.nativeAbi===2&&c.normalDllsBuilt===true&&c.published===false,'current normal source snapshot required');
 const base=JSON.parse((await checked(nativeLoot01RuntimeResource)).toString());
 demand(c.canonicalRejoinSchema?.path==='packages/contracts/g3-rejoin/schema.json','canonical schema required');await checked(c.canonicalRejoinSchema);
 demand(Array.isArray(c.sourcePins)&&c.sourcePins.length>=8&&c.sourcePins.length<=128&&Array.isArray(c.runtimeMappings)&&c.runtimeMappings.length>=35&&c.runtimeMappings.length<=48,'bounded current source/mod closure');
 const source=new Map<string,string>();for(const p of c.sourcePins){demand(!source.has(p.path),'duplicate source pin');source.set(p.path,p.sha256);await checked(p);}
 for(const path of ['native/g3-ipc/abi.h','native/g3-ipc/broker.cpp','native/g3-ipc/wrapper.cpp','native/g3-ipc/build.ps1','mod/noita_together2/init-g3-seed-loot.lua','apps/desktop/src-tauri/src/pipe.rs','apps/desktop/src-tauri/src/next_rejoin.rs','apps/desktop/src-tauri/src/protocol_rejoin.rs'])demand(source.has(path),'missing source '+path);
 const abi=(await checked({path:'native/g3-ipc/abi.h',sha256:source.get('native/g3-ipc/abi.h')!})).toString();const broker=(await checked({path:'native/g3-ipc/broker.cpp',sha256:source.get('native/g3-ipc/broker.cpp')!})).toString();demand(/\*resume\)\(/.test(abi)&&broker.includes('version==2?&api'),'ABI2 resume source required');
 const mapped=new Set<string>();for(const m of c.runtimeMappings){safeArtifactPath(m.path);safeArtifactPath(m.sourcePath);demand(m.path.startsWith('mods/noita_together2/')&&!mapped.has(m.path),'duplicate/foreign mod mapping');mapped.add(m.path);await checked({path:m.sourcePath,sha256:m.sha256});}
 for(const m of base.runtimeMappings)demand(mapped.has(m.path),'missing retained runtime file '+m.path);
 demand(mapped.has('mods/noita_together2/files/rejoin/local_world.lua'),'current local-world entry helper required');
 const entry=c.runtimeMappings.find((m:any)=>m.path==='mods/noita_together2/init.lua');demand(entry?.sourcePath==='mod/noita_together2/init-g3-seed-loot.lua'&&source.get(entry.sourcePath)===entry.sha256,'entry/source mismatch');
 const expected=(mode:string,privateImage=false)=>{const a=base.nativeArtifacts.find((a:any)=>a.mode===mode);demand(a,'unknown native mode');return [...a.exports,...(mode==='broker'?[]:rejoinWrapperExports),...(privateImage&&mode!=='broker'?['nt2_g3_private_api']:[])].sort();};
 const artifacts=c.normalArtifacts;demand(Array.isArray(artifacts)&&artifacts.length===3&&new Set(artifacts.map((a:any)=>a.mode)).size===3,'exact three normal images');
 for(const m of c.runtimeMappings)if(m.path.endsWith('.dll'))demand(artifacts.some((a:any)=>a.path===m.sourcePath&&a.sha256===m.sha256),'unverified DLL in normal closure');
 const validateImage=async(a:any,privateImage:boolean)=>{
  demand(a.testFlags===undefined||a.testFlags===privateImage,'declared native flags mismatch');
  demand(a.path.startsWith('spikes/noita-entry/.build/')&&a.path.endsWith('/'+(a.mode==='broker'?'nt2_g3_ipc':a.mode==='primary'?'nt2_g3_transport':'nt2_g3_social')+'.dll'),'bounded native output path');
  const exports=inspectNativeExports(await checked(a));demand(JSON.stringify(exports)===JSON.stringify(expected(a.mode,privateImage)),'native exports/flags mismatch '+a.mode);
  demand(a.buildCommand?.path.startsWith('spikes/noita-entry/.build/')&&a.buildCommand.path.endsWith('/compile.cmd'),'build receipt required');const command=(await checked(a.buildCommand)).toString();
  demand(command.includes('/Brepro')&&command.includes('-arch=x86')&&command.includes('broker.cpp')&&command.includes('wrapper.cpp'),'source-built x86 receipt required');
  demand(privateImage?command.includes('/DNT2_IPC_PRIVATE')&&command.includes('/DNT2_TEST_NOSAVE'):!/\/DNT2_(IPC_PRIVATE|TEST_)/.test(command),'native build flags mismatch');
  if(privateImage)demand(['private-live','private-saved'].includes(a.variant)&&command.includes('/DNT2_TEST_SAVED')===(a.variant==='private-saved'),'private saved/live receipt mismatch');
  return {...a,exports,loaded:false,installed:false,testFlags:privateImage};
 };
 const normal=[];for(const a of artifacts){const verified=await validateImage(a,false);demand(c.runtimeMappings.some((m:any)=>m.sourcePath===a.path&&m.sha256===a.sha256),'normal image outside closure');normal.push(verified);}
 const privateArtifacts=[];demand(Array.isArray(c.privateArtifacts??[])&&(c.privateArtifacts??[]).length<=6,'private image bounds');const identities=new Set();for(const a of c.privateArtifacts??[]){const identity=a.variant+':'+a.mode;demand(!identities.has(identity),'duplicate private image');identities.add(identity);demand(!c.runtimeMappings.some((m:any)=>m.sourcePath===a.path),'private image in normal closure');privateArtifacts.push(await validateImage(a,true));}
 const id='nt2-g3-rejoin-rj2-'+inputPin.sha256.slice(0,12),primary=normal.find(a=>a.mode==='primary');
 const buildRegistry={...base.buildRegistry,id,modSha256:entry.sha256,bridgeSha256:primary.sha256};validateBuildRegistry(buildRegistry);
 const r={...base,id,namespacePrefix:'g3_rejoin_',buildRegistry,profile:{...base.profile,...buildRegistry},status:'verified-current-uninstalled-RJ2-composition-only',sourceApproved:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,requiredLocalFiles:c.runtimeMappings.map(({path,sha256}:any)=>({path,sha256})),runtimeMappings:c.runtimeMappings,nativeArtifacts:normal,localProtocol:{...base.localProtocol,brokerAbiVersion:2,resumeControlVersion:7,resumeControlOperation:12,resumeControlBytes:56},rejoinRevision:'g3-rejoin-1',compositionInput:inputPin,privateArtifacts,provenance:{clientDependency:inputPin,canonicalRejoinSchema:c.canonicalRejoinSchema,pins:c.sourcePins},closure:{...base.closure,localDependencies:c.runtimeMappings.length}};
 const folder='spikes/noita-entry/.build/server-rejoin-rj2-'+inputPin.sha256,runtimeResourceJson=JSON.stringify(r,null,2)+'\n';
 const runtimeApproval={path:folder+'/runtime-approval.json',sha256:sha(Buffer.from(runtimeResourceJson))};
 const review={activeGate:'G3A/reconnect-rejoin',checkpoint:'RJ2-S',sourceApproved:true,developmentOnly:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,compositionOnly:true,input:inputPin,runtimeApproval,pins:c.sourcePins,privateArtifacts,remaining:['Client actual compiled composition','Root RJ2 review','RJ3 final app/host/kit','Actual Noita saved-world proof']};
 const sourceResourceJson=JSON.stringify(review,null,2)+'\n',desktopApproval={path:folder+'/source-approval.json',sha256:sha(Buffer.from(sourceResourceJson))};
 return {...r,runtimeResourceJson,sourceResourceJson,runtimeApproval,desktopApproval,compositionOnly:true};
}
export async function materializeRejoinComposition(inputPin:ArtifactPin){
 const r=await verifyRejoinCompositionInput(inputPin);await mkdir(new URL(r.runtimeApproval.path.replace(/\/runtime-approval.json$/,'/'),root),{recursive:true});
 for(const [p,body]of [[r.runtimeApproval.path,r.runtimeResourceJson],[r.desktopApproval.path,r.sourceResourceJson]]){const url=new URL(p!,root);try{await writeFile(url,body!,{flag:'wx'});}catch(e){if((e as NodeJS.ErrnoException).code!=='EEXIST'||(await readFile(url,'utf8'))!==body)throw e;}}
 return r;
}
