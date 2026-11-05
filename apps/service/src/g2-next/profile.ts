import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import type {Options} from '../g1/app.js';
import {validate} from '../../../../packages/contracts/src/g2-next.js';
import {fingerprint} from '../g1/security.js';
import {validateBuildRegistry} from './build-registry.js';
import type {CompatibilityProfile} from '../../../../packages/contracts/g2-next/g2-next.js';
const root=new URL('../../../../',import.meta.url);
export const inspectionProfile=JSON.parse(await readFile(new URL('deploy/profiles/g2-inspect-candidate.json',root),'utf8'));
export const stagedMultibuildProfile=JSON.parse(await readFile(new URL('deploy/profiles/g2-inspect-multibuild-candidate.json',root),'utf8'));
const same=(a:unknown,b:unknown)=>fingerprint(a)===fingerprint(b);
export function inspectionMetadata(staged=false){const p=staged?stagedMultibuildProfile:inspectionProfile;return {profile:structuredClone(p.profile),compatibility:{gameVersion:p.gameVersion,modVersion:p.modVersion,bridgeVersion:p.bridgeVersion,capabilities:[...p.capabilities],modes:[...p.modes],settingKeys:[...p.settingKeys]}};}
export function validateCandidateMetadata(options:{profile?:CompatibilityProfile;compatibility:Options['compatibility']}){
 // Simulated fixtures remain explicitly labelled; never advertise historical or mixed native metadata.
 if(!options.profile)return;
 const expected=inspectionMetadata(options.profile.supportedBuilds!==undefined);
 if(options.profile.id!==expected.profile.id||!same(options.profile,expected.profile)||!same(options.compatibility,expected.compatibility))throw Error('Successor native metadata must match the separate inspection candidate profile exactly');
 validateBuildRegistry(options.profile);validate('Compatibility',options.compatibility);
}
export async function verifyInspectionArtifacts(reader:(path:URL)=>Promise<Buffer>=path=>readFile(path)){
 const p=inspectionProfile,manifestBytes=await reader(new URL(p.provenance.manifest.path,root)),manifest=JSON.parse(manifestBytes.toString('utf8'));
 const sha=(bytes:Buffer)=>createHash('sha256').update(bytes).digest('hex');
 if(sha(manifestBytes)!==p.provenance.manifest.sha256||manifest.revision!==p.provenance.manifest.revision)throw Error('Client inspection manifest changed; review refreshed native profile before launch');
 for(const key of ['gameVersion','modVersion','bridgeVersion'])if(manifest[key]!==p[key])throw Error('Client inspection versions differ');
 for(const [key,value]of Object.entries(p.profile))if(!same(manifest[key],value))throw Error('Client inspection profile differs');
 for(const artifact of p.provenance.artifacts)if(manifest[artifact.manifestField]!==artifact.sha256||sha(await reader(new URL(artifact.path,root)))!==artifact.sha256)throw Error('Candidate inspection artifact changed; review refreshed manifest/profile before launch');
 for(const source of p.provenance.sources)if(sha(await reader(new URL(source.path,root)))!==source.sha256)throw Error('Candidate inspection source changed; review refreshed build provenance before launch');
 return {id:p.profile.id,evidenceStatus:p.profile.evidenceStatus,manifestSha256:p.provenance.manifest.sha256,artifacts:p.provenance.artifacts.map((a:{path:string;sha256:string})=>({...a})),sources:p.provenance.sources};
}
