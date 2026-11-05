import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {validateBuildRegistry} from '../g2-next/build-registry.js';
const root=new URL('../../../../',import.meta.url);
export const nativeDevelopmentProfile=JSON.parse(await readFile(new URL('deploy/profiles/g3-native-development-candidate.json',root),'utf8'));
export const runtimeApprovalResource={path:'deploy/profiles/g3-native-runtime-approval.record2-complete.json',sha256:'64095b1f054f793088572dc5751ae578d91e65ec0f3e5f4e51e4eaa853faea72'} as const;
export const desktopReviewResource={path:'deploy/profiles/g3-native-desktop-review.record9.json',sha256:'649b1f4974cea49514ac1e7bcb29b832ba0d5bcf04ab740eef90e9edcd9a7d8b'} as const;
export async function verifyNativeDevelopmentArtifacts(reader:(path:URL)=>Promise<Buffer>=path=>readFile(path)){
 const sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
 const checked=async(pin:{path:string;sha256:string})=>{const bytes=await reader(new URL(pin.path,root));if(sha(bytes)!==pin.sha256)throw Error('G3 native dependency changed: '+pin.path);return bytes;};
 // Read and hash resources on EVERY startup/restart; no cached JSON bypass.
 const runtime=JSON.parse((await checked(runtimeApprovalResource)).toString('utf8'));
 const desktop=JSON.parse((await checked(desktopReviewResource)).toString('utf8'));
 validateBuildRegistry(runtime.profile);
 if(desktop.runtimeApproval.path!==runtimeApprovalResource.path||desktop.runtimeApproval.sha256!==runtimeApprovalResource.sha256||desktop.runtimeApproval.runtimeId!==runtime.id)throw Error('G3 runtime review linkage changed');
 await checked(runtime.provenance.originalReview);
 for(const pin of runtime.provenance.pins)await checked(pin);
 await checked(desktop.manifest);await checked(desktop.snapshotRequest);
 for(const pin of desktop.pins)await checked(pin);
 return {...runtime,provenance:desktop,runtimeApproval:runtimeApprovalResource,desktopReview:desktopReviewResource};
}
