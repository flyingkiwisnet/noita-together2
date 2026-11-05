import {readFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
import {fingerprint} from '../g1/security.js';import {validateBuildRegistry} from '../g2-next/build-registry.js';
export const nativeSeedLootRuntimeResource={path:'deploy/profiles/g3-seed-loot-runtime.record1.json',sha256:'611801032008de35be6c70ba5e9512f13d91ed3489592c8976bd2f1263dcb48f'} as const;
export const nativeSeedLootSourceResource={path:'deploy/profiles/g3-seed-loot-source.record1.json',sha256:'84d4cd786ece2ed469d15ca3d59b676d05af7b0d457cc964afeac8caa0895461'} as const;
const root=new URL('../../../../',import.meta.url),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
// Metadata verification only: no normal DLL loading, installed/runtime selection or issuer change.
export async function verifyNativeSeedLootArtifacts(reader:(url:URL)=>Promise<Buffer>=url=>readFile(url)){
 const checked=async(p:{path:string;sha256:string})=>{if(!p.path||p.path.startsWith('/')||p.path.includes('..')||/[\\:]/.test(p.path))throw Error('Unsafe seed/loot dependency');const b=await reader(new URL(p.path,root));if(sha(b)!==p.sha256)throw Error('Seed/loot dependency changed: '+p.path);return b;};
 const bytes=await checked(nativeSeedLootRuntimeResource),runtime=JSON.parse(bytes.toString('utf8')),review=JSON.parse((await checked(nativeSeedLootSourceResource)).toString('utf8'));
 const demand=(v:unknown)=>{if(!v)throw Error('Seed/loot matching definition changed');};
 for(const r of [runtime,review])demand(r.sourceApproved===true&&r.developmentOnly===true&&r.testBuild===true&&r.installApproved===false&&r.productionApproved===false&&r.packaged===false&&r.actualNoitaEvidence===false);
 demand(review.usableCandidate===false&&review.consumerRebuildRequired===true&&review.finalRebuiltDesktopReviewRequired===true);
 const client=JSON.parse((await checked(review.snapshotRequest)).toString('utf8')),base=JSON.parse((await checked(review.baselineRuntimeApproval)).toString('utf8')),catalog=JSON.parse((await checked(review.originalBuildRegistry)).toString('utf8'));
 demand(runtime.id==='nt2-g3-seed-loot-1'&&runtime.serviceId===base.serviceId&&runtime.namespacePrefix===base.namespacePrefix&&runtime.pipePrefix===base.pipePrefix&&runtime.configurationDirectory===base.configurationDirectory&&runtime.cargoFeature===base.cargoFeature);
 const caps=['core.seed','core.ghost','core.hud','core.poi','core.seed-policy.v1','core.randomize-loot.v1'];
 demand(fingerprint(runtime.compatibility)===fingerprint({...base.compatibility,capabilities:caps})&&fingerprint(runtime.profile.capabilities)===fingerprint(caps));
 validateBuildRegistry(runtime.buildRegistry);demand(fingerprint(runtime.profile)===fingerprint({...runtime.buildRegistry,gameVersion:runtime.compatibility.gameVersion,modVersion:runtime.compatibility.modVersion,bridgeVersion:runtime.compatibility.bridgeVersion,capabilities:caps}));
 for(const k of ['supportedBuilds','gameExeSha256','originalLuaSha256','gameFamilyId'])demand(fingerprint(runtime.profile[k])===fingerprint(catalog.profile[k]));
 demand(runtime.profile.modSha256===client.entrySha256&&runtime.profile.bridgeSha256===client.primarySha256);
 demand(runtime.localProtocol.controlVersion===4&&runtime.localProtocol.reportVersion===5&&runtime.localProtocol.policyControlOperation===10&&runtime.localProtocol.actualSeedControlVersion===6&&runtime.localProtocol.actualSeedControlOperation===11&&runtime.localProtocol.brokerAbiVersion===1&&runtime.localProtocol.startupWaitMaximumMs===250&&runtime.localProtocol.heartbeatMovementSocialJobBudgetMs===80);
 demand(runtime.runtimeMappings.length===34&&new Set(runtime.runtimeMappings.map((m:any)=>m.path)).size===34&&fingerprint(runtime.runtimeMappings)===fingerprint(client.runtimeMappings)&&fingerprint(runtime.requiredLocalFiles)===fingerprint(client.requiredLocalFiles));
 for(const m of runtime.runtimeMappings)demand(m.path.startsWith('mods/noita_together2/')&&!m.path.includes('..')&&!/[\\:]/.test(m.path)&&review.pins.some((p:any)=>p.path===m.sourcePath&&p.sha256===m.sha256));
 demand(runtime.nativeArtifacts.length===3);for(const a of runtime.nativeArtifacts)demand(a.testFlags===false&&a.loaded===false&&a.installed===false&&a.path.startsWith('spikes/noita-entry/.build/g3-seed-loot-native-inputs-20261011-01/')&&runtime.runtimeMappings.some((m:any)=>m.sourcePath===a.path&&m.sha256===a.sha256));
 demand(fingerprint(runtime.provenance.pins)===fingerprint(review.pins)&&fingerprint(runtime.provenance.canonicalPins)===fingerprint(review.canonicalPins));
 for(const p of [...review.pins,...review.canonicalPins])await checked(p);
 demand(review.runtimeApprovals.some((p:any)=>p.path===nativeSeedLootRuntimeResource.path&&p.sha256===nativeSeedLootRuntimeResource.sha256&&p.runtimeId===runtime.id));
 return {...runtime,runtimeApproval:nativeSeedLootRuntimeResource,desktopApproval:nativeSeedLootSourceResource,sourceReview:review,runtimeResourceJson:bytes.toString('utf8')};
}
