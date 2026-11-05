import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {validateBuildRegistry} from '../g2-next/build-registry.js';
import {fingerprint} from '../g1/security.js';
const root=new URL('../../../../',import.meta.url);
export const nativeA1RuntimeResources={
 primary:{path:'deploy/profiles/g3-native-runtime-approval.record3-a1.json',sha256:'959fddf3a0d5948709e685fdfda325de96004d98b798e377b70f65b5ad8502dd'},
 social:{path:'deploy/profiles/g3-native-social-runtime-approval.record2-a1.json',sha256:'b0ce032108612ff1d2540008ae9a96f746748243857d5d6efaa8537b7be934c0'}
} as const;
export const nativeA1SourceReviewResource={path:'deploy/profiles/g3-native-a1-source-review.record17.json',sha256:'7e44483c30817c317830735f00e3ce04a5e16653493c81a923886afec51ad319'} as const;
// Explicit opt-in only. This source review is NOT final rebuilt desktop approval.
export async function verifyNativeA1DevelopmentArtifacts(social=false,reader:(path:URL)=>Promise<Buffer>=path=>readFile(path),selection:{resources:{primary:{path:string;sha256:string};social:{path:string;sha256:string}};sourceReview:{path:string;sha256:string};localTest?:true}={resources:nativeA1RuntimeResources,sourceReview:nativeA1SourceReviewResource}){
 const checked=async(pin:{path:string;sha256:string})=>{
  if(!pin.path||pin.path.startsWith('/')||pin.path.includes('..')||pin.path.includes('\\')||pin.path.includes(':'))throw Error('Unsafe A1 dependency path');
  const bytes=await reader(new URL(pin.path,root));
  if(createHash('sha256').update(bytes).digest('hex')!==pin.sha256)throw Error('G3 A1 dependency changed: '+pin.path);
  return bytes;
 };
 const resource=selection.resources[social?'social':'primary'],runtimeBytes=await checked(resource);
 const runtime=JSON.parse(runtimeBytes.toString('utf8')),review=JSON.parse((await checked(selection.sourceReview)).toString('utf8'));
 const refuse=()=>{throw Error('G3 A1 source definition changed');};
 const caps=social?['core.seed','core.ghost','core.hud','core.poi']:['core.seed','core.ghost'];
 if(runtime.sourceApproved!==true||runtime.developmentOnly!==true||runtime.installApproved!==false||runtime.packaged!==false||runtime.actualNoitaEvidence!==false||review.usableCandidate!==false||review.consumerRebuildRequired!==true)refuse();
 if(selection.localTest?runtime.testBuild!==true||review.testBuild!==true||review.finalRebuiltDesktopReviewRequired!==false:review.finalRebuiltDesktopReviewRequired!==true)refuse();
 if(fingerprint(runtime.compatibility.capabilities)!==fingerprint(caps)||!review.runtimeApprovals.some((p:any)=>p.path===resource.path&&p.sha256===resource.sha256&&p.runtimeId===runtime.id))refuse();
 validateBuildRegistry(runtime.buildRegistry);
 if(runtime.profile.id!==runtime.id||runtime.buildRegistry.id!==runtime.id||runtime.localProtocol.replyVersion!==2||runtime.localProtocol.replyBytes!==864||runtime.localProtocol.maxAuthorityLeaseMs!==200)refuse();
 if(runtime.requiredLocalFiles.length!==(social?20:14)||runtime.runtimeMappings.length!==runtime.requiredLocalFiles.length||new Set(runtime.runtimeMappings.map((m:any)=>m.path)).size!==runtime.runtimeMappings.length)refuse();
 for(const m of runtime.runtimeMappings){
  if(!m.path.startsWith('mods/noita_together2/')||m.path.includes('..')||m.path.includes('\\')||!m.sourcePath.startsWith(review.frozenRoot+'/')||!runtime.requiredLocalFiles.some((p:any)=>p.path===m.path&&p.sha256===m.sha256)||!review.pins.some((p:any)=>p.path===m.sourcePath&&p.sha256===m.sha256))refuse();
 }
 // Hash the selected metadata's dependencies; local tests list only required runtime bytes.
 await checked(review.snapshotRequest);
 for(const p of review.pins)await checked(p);
 for(const p of runtime.provenance.pins)await checked(p);
 return {...runtime,runtimeApproval:resource,desktopApproval:selection.sourceReview,sourceReview:review,runtimeResourceJson:runtimeBytes.toString('utf8')};
}
