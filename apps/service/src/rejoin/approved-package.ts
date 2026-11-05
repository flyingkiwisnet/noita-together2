import {readFile} from 'node:fs/promises';import {createHash} from 'node:crypto';import {demand,fingerprint} from '../g1/security.js';import {validateBuildRegistry} from '../g2-next/build-registry.js';
export interface NormalPackageApproval {runtime:{path:string;sha256:string};source:{path:string;sha256:string}}
// Explicit operator-selected, pinned normal package. This is server configuration,
// never an RPC field, and never discovers trust from mutable checkout contents.
export async function verifyApprovedNormalPackage(approval:NormalPackageApproval){const root=new URL('../../../../',import.meta.url);const checked=async(p:{path:string;sha256:string})=>{demand(typeof p.path==='string'&&/^deploy\/profiles\/[a-zA-Z0-9._-]+\.json$/.test(p.path)&&/^[a-f0-9]{64}$/.test(p.sha256),'INVALID_PACKAGE_APPROVAL');const b=await readFile(new URL(p.path,root));demand(createHash('sha256').update(b).digest('hex')===p.sha256,'INVALID_PACKAGE_APPROVAL');return b;};
 const bytes=await checked(approval.runtime),reviewBytes=await checked(approval.source),r=JSON.parse(bytes.toString()),review=JSON.parse(reviewBytes.toString());
 for(const p of [r,review])demand(p.sourceApproved===true&&p.developmentOnly===true&&p.installApproved===false&&p.productionApproved===false&&p.packaged===false&&p.actualNoitaEvidence===false,'INVALID_PACKAGE_APPROVAL');
 demand(review.runtimeApproval?.path===approval.runtime.path&&review.runtimeApproval?.sha256===approval.runtime.sha256&&r.fixtureOnly===false&&!['fixtureNative','c1Fixture','c2Fixture','b1Fixture','itemFixture','raceFixture'].some(k=>r[k]!==undefined),'INVALID_PACKAGE_APPROVAL');
 demand(Array.isArray(r.nativeArtifacts)&&r.nativeArtifacts.length===3&&r.nativeArtifacts.every((a:any)=>a.private===false&&a.testFlags===false),'INVALID_PACKAGE_APPROVAL');validateBuildRegistry(r.buildRegistry);
 demand(r.requiredLocalFiles?.length===r.runtimeMappings?.length&&r.runtimeMappings.length>=1&&r.runtimeMappings.length<=512,'INVALID_PACKAGE_APPROVAL');const seen=new Set();for(const m of r.runtimeMappings){demand(typeof m.path==='string'&&m.path.startsWith('mods/noita_together2/')&&!/[\\:]/.test(m.path)&&m.path.split('/').every((s:string)=>s&&s!=='.'&&s!=='..')&&!seen.has(m.path)&&/^[a-f0-9]{64}$/.test(m.sha256)&&r.requiredLocalFiles.some((p:any)=>p.path===m.path&&p.sha256===m.sha256),'INVALID_PACKAGE_APPROVAL');seen.add(m.path);}
 const raceCapability='race.goals.v1',racePrefix='mods/noita_together2/files/race/';
 const raceSelected=r.compatibility?.modes?.includes('race')||r.compatibility?.capabilities?.includes(raceCapability)||r.profile?.capabilities?.includes(raceCapability)||r.runtimeMappings.some((m:any)=>m.path.startsWith(racePrefix));
 if(raceSelected){
  demand(r.runtimeMappings.length===324&&Array.isArray(r.compatibility?.modes)&&Array.isArray(r.compatibility?.capabilities)&&Array.isArray(r.profile?.capabilities)&&r.compatibility.modes.includes('core')&&r.compatibility.modes.includes('race')&&r.compatibility.capabilities.includes(raceCapability)&&r.profile.capabilities.includes(raceCapability),'INVALID_PACKAGE_APPROVAL');
  for(const k of ['gameVersion','modVersion','bridgeVersion','capabilities'])demand(fingerprint(r.profile[k])===fingerprint(r.compatibility[k]),'INVALID_PACKAGE_APPROVAL');
  demand(!Object.keys(r).some(k=>k.endsWith('Fixture')),'INVALID_PACKAGE_APPROVAL');
  const files=['catalogue.lua','hook_api.lua','local.lua','panel.lua','player_death.lua','register.lua','sample.lua','sha256.lua','storage.lua','append/boss.lua','append/orb.lua'];
  const mapped=r.runtimeMappings.filter((m:any)=>m.path.startsWith(racePrefix));demand(mapped.length===files.length,'INVALID_PACKAGE_APPROVAL');
  for(const file of files){const m=mapped.find((m:any)=>m.path===racePrefix+file);demand(m&&m.sourcePath==='mod/noita_together2/files/race/'+file,'INVALID_PACKAGE_APPROVAL');}
 }
 return {...r,runtimeResourceJson:bytes.toString(),runtimeApproval:approval.runtime,desktopApproval:approval.source};
}
