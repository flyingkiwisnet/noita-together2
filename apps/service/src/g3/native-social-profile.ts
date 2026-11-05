import {readFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
import {validateBuildRegistry} from '../g2-next/build-registry.js';import {fingerprint} from '../g1/security.js';
import {runtimeApprovalResource as primaryResource} from './native-profile.js';
const root=new URL('../../../../',import.meta.url);
export const nativeSocialRuntimeResource={path:'deploy/profiles/g3-native-social-runtime-approval.record1.json',sha256:'f1e4c251d0feb5caea64f645f8b60f74040744857831a89414202c8d17d1357e'} as const;
export const nativeSocialDesktopResource={path:'deploy/profiles/g3-native-desktop-review.record15.json',sha256:'d41ed5878840cd7be1c1b1d18f06594de06a657d3ffd8de0f912dd3bf920f58b'} as const;
const runtimeSourceRoot='spikes/noita-entry/.build/g3-candidates/nt2-g3-cp13-20261006-172633-5e8ca466';
export const NATIVE_SOCIAL_REQUIRED_FILES=['init.lua','files/nt2_g3_social.dll','files/g3/social_native.lua','files/g3/social_dispatch.lua','files/g3/hud_poi_local.lua','files/g3/social_game_api.lua','files/g3/validation.lua','files/inspection.lua','files/g3/native_owner.lua','files/g3/movement.lua','files/g3/ghosts.lua','files/g3/game_api.lua','files/g3/poi.xml','files/g3/ui/hpbar_full.png','files/g3/ui/hpbar_empty.png','files/g3/ui/sampo.png','files/logging.xml','files/g3/ghost.xml','files/g3/poi_pickup.lua','files/g3/ui/poi.png'] as const;
const caps=['core.seed','core.ghost','core.hud','core.poi'];
// Definition check is not approval. The only service loader pins both resource bytes.
export function assertNativeSocialDefinition(runtime:any,desktop:any,primary:any){
 const refuse=()=>{throw Error('G3 native social source definition changed');};
 if(runtime.id!=='nt2-g3-native-social-development-1'||runtime.sourceApproved!==true||runtime.developmentOnly!==true||runtime.installApproved!==false||runtime.packaged!==false||runtime.actualNoitaEvidence!==false||runtime.serviceId!=='nt2-g3-native-social-development'||runtime.namespacePrefix!=='g3_social_'||runtime.pipePrefix!=='nt2-g3-social-dev-v1-')refuse();
 if(fingerprint(runtime.compatibility.capabilities)!==fingerprint(caps)||fingerprint(runtime.compatibility.modes)!==fingerprint(['core'])||runtime.compatibility.modVersion!=='nt2-g3-social-dev-1'||runtime.compatibility.bridgeVersion!=='g3-social-transport-1'||runtime.compatibility.gameVersion!==primary.compatibility.gameVersion||runtime.schemaSha256!==primary.schemaSha256||runtime.socialSchemaSha256!=='6eb9aa03befb56ac4b334fb266ecbdb58629521249ac11413863cdf8eb04b399'||runtime.replayPolicy!=='sequence-bound-transient-v1')refuse();
 validateBuildRegistry(runtime.buildRegistry);
 if(runtime.buildRegistry.id!==runtime.id||fingerprint(runtime.buildRegistry.supportedBuilds)!==fingerprint(primary.profile.supportedBuilds)||fingerprint(runtime.profile)!==fingerprint({...runtime.buildRegistry,gameVersion:runtime.compatibility.gameVersion,modVersion:runtime.compatibility.modVersion,bridgeVersion:runtime.compatibility.bridgeVersion,capabilities:caps}))refuse();
 const files=runtime.requiredLocalFiles,mappings=runtime.runtimeMappings,expected=NATIVE_SOCIAL_REQUIRED_FILES.map(p=>'mods/noita_together2/'+p);
 if(!Array.isArray(files)||!Array.isArray(mappings)||files.length!==expected.length||mappings.length!==expected.length||new Set(files.map((p:any)=>p.path)).size!==expected.length||new Set(mappings.map((p:any)=>p.path)).size!==expected.length)refuse();
 for(const path of expected){const file=files.find((p:any)=>p.path===path),mapping=mappings.find((p:any)=>p.path===path);if(!file||!mapping||file.sha256!==mapping.sha256||!/^[0-9a-f]{64}$/.test(file.sha256)||!mapping.sourcePath.startsWith(runtimeSourceRoot+'/')||mapping.sourcePath.includes('/../')||!runtime.provenance.pins.some((p:any)=>p.path===mapping.sourcePath&&p.sha256===file.sha256)||!desktop.pins.some((p:any)=>p.path===desktop.frozenRoot+mapping.sourcePath.slice(runtimeSourceRoot.length)&&p.sha256===file.sha256))refuse();}
 if(runtime.buildRegistry.modSha256!==files.find((p:any)=>p.path==='mods/noita_together2/init.lua').sha256||runtime.buildRegistry.bridgeSha256!==files.find((p:any)=>p.path==='mods/noita_together2/files/nt2_g3_social.dll').sha256)refuse();
 if(desktop.installApproved!==false||desktop.packaged!==false||desktop.actualNoitaEvidence!==false||desktop.supplementalRuntimeApproval.path!==nativeSocialRuntimeResource.path||desktop.supplementalRuntimeApproval.sha256!==nativeSocialRuntimeResource.sha256||desktop.supplementalRuntimeApproval.runtimeId!==runtime.id||fingerprint(desktop.supplementalRuntimeApproval.capabilities)!==fingerprint(caps)||desktop.runtimeApproval.path!==primaryResource.path||desktop.runtimeApproval.sha256!==primaryResource.sha256||desktop.runtimeApproval.runtimeId!==primary.id||desktop.pins.length!==195||new Set(desktop.pins.map((p:any)=>p.path)).size!==195||!desktop.pins.some((p:any)=>p.path===desktop.selectedTestExecutable.path&&p.sha256===desktop.selectedTestExecutable.sha256)||!desktop.selectedTestExecutable.path.endsWith('/examples/g3_social_candidate.exe'))refuse();
}
export async function verifyNativeSocialDevelopmentArtifacts(reader:(url:URL)=>Promise<Buffer>=readFile){
 const checked=async(pin:{path:string;sha256:string})=>{const bytes=await reader(new URL(pin.path,root));if(createHash('sha256').update(bytes).digest('hex')!==pin.sha256)throw Error('G3 native social dependency changed: '+pin.path);return bytes;};
 // Every startup/restart checks resource, local closure, native source, contract,
 // manifest and frozen desktop separately. No supplied approval/profile object.
 const runtimeBytes=await checked(nativeSocialRuntimeResource),runtime=JSON.parse(runtimeBytes.toString()),desktop=JSON.parse((await checked(nativeSocialDesktopResource)).toString()),primary=JSON.parse((await checked(primaryResource)).toString());
 assertNativeSocialDefinition(runtime,desktop,primary);
 for(const pin of runtime.provenance.pins)await checked(pin);
 await checked(desktop.manifest);await checked(desktop.snapshotRequest);for(const pin of desktop.pins)await checked(pin);
 return {...runtime,desktopReview:desktop,runtimeApproval:nativeSocialRuntimeResource,desktopApproval:nativeSocialDesktopResource,runtimeResourceJson:runtimeBytes.toString('utf8')};
}
