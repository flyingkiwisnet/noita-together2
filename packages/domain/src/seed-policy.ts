import type {Config} from '../../contracts/g2-next/g2-next.js';
export const SEED_POLICY_CAPABILITY='core.seed-policy.v1';
export const LOOT_CAPABILITY='core.randomize-loot.v1';
export function seedPolicy(config:Config){return config.seedPolicy??'shared-fixed';}
export function effectiveLoot(config:Config){return config.seedPolicy!==undefined&&seedPolicy(config)!=='individual-random'&&config.randomizeLoot===true;}
export function seedMatches(config:Config,actualSeed:string|undefined){return actualSeed!==undefined&&(seedPolicy(config)==='individual-random'||actualSeed===String(config.seed));}
export function policyCapabilities(config:Config){return config.seedPolicy===undefined?[]:[SEED_POLICY_CAPABILITY,...(effectiveLoot(config)?[LOOT_CAPABILITY]:[])];}
