import test from 'node:test';
import assert from 'node:assert/strict';
import {inspectionMetadata,validateCandidateMetadata} from '../src/g2-next/profile.js';
import {validateBuildRegistry,matchGameBuild} from '../src/g2-next/build-registry.js';
import {encode,decode,validate} from '../../../packages/contracts/src/g2-next.js';
test('registry accepts exact Steam/GOG pairs and preserves strict native/collector pins',()=>{
 const staged=inspectionMetadata(true);validateCandidateMetadata(staged);validateBuildRegistry(staged.profile);
 for(const build of staged.profile.supportedBuilds){assert.equal(matchGameBuild(staged.profile,build.gameExeSha256,build.originalLuaSha256)?.id,build.id);}
 assert.equal(staged.profile.bridgeSha256,inspectionMetadata().profile.bridgeSha256);assert.equal(staged.profile.modSha256,inspectionMetadata().profile.modSha256);
 assert.deepEqual(decode('CompatibilityProfile',encode('CompatibilityProfile',staged.profile)),staged.profile);
 assert.equal(matchGameBuild(staged.profile,'0'.repeat(64),staged.profile.originalLuaSha256),undefined);
 assert.equal(matchGameBuild(staged.profile,staged.profile.supportedBuilds[1].gameExeSha256,'0'.repeat(64)),undefined);
});
test('registry does not permit Cartesian product pairs or fallback from invalid/present registry',()=>{
 const original=inspectionMetadata(true).profile,steam=original.supportedBuilds[0],gog=original.supportedBuilds[1];
 const alternate={...original,supportedBuilds:[steam,{...gog,originalLuaSha256:'a'.repeat(64)}]};validateBuildRegistry(alternate);
 assert.equal(matchGameBuild(alternate,steam.gameExeSha256,'a'.repeat(64)),undefined);assert.equal(matchGameBuild(alternate,gog.gameExeSha256,steam.originalLuaSha256),undefined);
 for(const profile of [{...original,supportedBuilds:[]},{...original,gameFamilyId:undefined},{...original,supportedBuilds:undefined},{...original,supportedBuilds:[steam,steam]},{...original,supportedBuilds:[steam,{...gog,id:steam.id}]},{...original,supportedBuilds:[gog]},{...original,supportedBuilds:[steam,{...gog,gameExeSha256:'unknown-beta'}]},{...original,supportedBuilds:Array(9).fill(steam)}])assert.throws(()=>validateBuildRegistry(profile));
 const legacy=structuredClone(inspectionMetadata().profile);delete legacy.supportedBuilds;delete legacy.gameFamilyId;assert.equal(matchGameBuild(legacy,steam.gameExeSha256,steam.originalLuaSha256)?.id,'legacy-reference');assert.equal(matchGameBuild(legacy,gog.gameExeSha256,gog.originalLuaSha256),undefined);
 assert.throws(()=>validate('CompatibilityProfile',{...original,supportedBuilds:Array(9).fill(steam)}));
 assert.throws(()=>validateBuildRegistry({...original,evidenceStatus:'anything'}));
 for(const evidenceStatus of ['approved','candidate','verified',''])assert.throws(()=>validateBuildRegistry({...original,supportedBuilds:[steam,{...gog,evidenceStatus}]}));
});
