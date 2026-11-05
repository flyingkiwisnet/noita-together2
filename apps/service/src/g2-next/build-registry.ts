import type {CompatibilityProfile,GameBuildVariant} from '../../../../packages/contracts/g2-next/g2-next.js';
import {validate} from '../../../../packages/contracts/src/g2-next.js';
const hash=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{64}$/.test(v);
export function validateBuildRegistry(profile:CompatibilityProfile){
 validate('CompatibilityProfile',profile);
 if(!['candidate-engine-evidence-pending','verified'].includes(profile.evidenceStatus??''))throw Error('Invalid native profile evidence status');
 if(!hash(profile.gameExeSha256)||!hash(profile.originalLuaSha256))throw Error('Native profile needs an exact reference executable/Lua pair');
 const variants=profile.supportedBuilds,family=profile.gameFamilyId;
 if(variants===undefined&&family===undefined)return;
 if(typeof family!=='string'||!/^[a-z0-9][a-z0-9._-]{0,79}$/.test(family)||!variants||variants.length<1||variants.length>8)throw Error('Build registry needs a compatible family and one to eight explicit pairs');
 const ids=new Set<string>(),pairs=new Set<string>();
 for(const v of variants){
  if(!v.id||!/^[a-z0-9][a-z0-9._-]{0,79}$/.test(v.id)||!v.store||!v.branch||!v.buildIdentity||!['candidate-engine-evidence-pending','engine-compatibility-verified'].includes(v.evidenceStatus??'')||!hash(v.gameExeSha256)||!hash(v.originalLuaSha256))throw Error('Build variant identity, approved evidence status and exact executable/Lua pair are required');
  const pair=v.gameExeSha256+':'+v.originalLuaSha256;
  if(ids.has(v.id)||pairs.has(pair))throw Error('Duplicate build variant identity or pair');ids.add(v.id);pairs.add(pair);
 }
 if(!pairs.has(profile.gameExeSha256+':'+profile.originalLuaSha256))throw Error('Reference pair must belong to the explicit build registry');
}
export function matchGameBuild(profile:CompatibilityProfile,exe:string,lua:string):GameBuildVariant|undefined{
 validateBuildRegistry(profile);
 if(profile.supportedBuilds)return profile.supportedBuilds.find(v=>v.gameExeSha256===exe&&v.originalLuaSha256===lua);
 // Legacy metadata has exactly one reference pair. Never fall back from a present registry.
 if(profile.gameExeSha256===exe&&profile.originalLuaSha256===lua)return {id:'legacy-reference',store:'unclassified',branch:'unclassified',buildIdentity:'Legacy exact reference pair',gameExeSha256:exe,originalLuaSha256:lua,evidenceStatus:profile.evidenceStatus??'candidate-engine-evidence-pending'};
 return;
}
