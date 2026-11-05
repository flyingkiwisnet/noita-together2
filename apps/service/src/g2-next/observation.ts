import {seedMatches} from '../../../../packages/domain/src/seed-policy.js';
import type {Config} from '../../../../packages/contracts/g2-next/g2-next.js';
import {createHash} from 'node:crypto';
import type {GameObservation} from '../../../../packages/contracts/g2-next/g2-next.js';
import {validate} from '../../../../packages/contracts/src/g2-next.js';
import {demand} from '../g1/security.js';
const failures=['seed_unavailable','mods_unavailable','inventory_limit','invalid_inventory','collection_failed','protection_failed'];
export function validateObservation(input:GameObservation){
 validate('GameObservation',input);demand(Buffer.byteLength(JSON.stringify(input))<=32768,'CAPACITY');
 demand(input.attachmentId&&input.gameInstanceId&&input.worldToken&&input.observationRevision&&typeof input.inventoryComplete==='boolean'&&typeof input.protected==='boolean'&&typeof input.loaded==='boolean','INVALID_MESSAGE');
 demand(['known','unknown'].includes(input.seedStatus??'')&&['true','false','unknown'].includes(input.beta??''),'INVALID_MESSAGE');
 if(input.seedStatus==='known')demand(typeof input.actualSeed==='string'&&/^(0|[1-9][0-9]{0,19})$/.test(input.actualSeed)&&BigInt(input.actualSeed)<=18446744073709551615n,'INVALID_MESSAGE');
 else demand(input.actualSeed===undefined,'INVALID_MESSAGE');
 const failure=input.collectorFailure??'';demand(!failure||failures.includes(failure),'INVALID_MESSAGE');
 demand(!input.inventoryComplete||(input.seedStatus==='known'&&!failure),'INVALID_MESSAGE');
 demand(input.inventoryComplete||!!failure,'INVALID_MESSAGE');
 demand(input.modVersion&&input.bridgeVersion&&input.compatibilityProvenance==='game-thread-client-observation','INVALID_MESSAGE');
 const invalidText=(s:string)=>/\p{Cc}/u.test(s)||/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(s);
 demand((input.activeMods??[]).every(id=>id.length>0&&!invalidText(id))&&!invalidText(input.modVersion)&&!invalidText(input.bridgeVersion),'INVALID_MESSAGE');
 // Preserve ordered duplicates exactly. Mod differences/duplicates are advisory, never deduplicated.
 return input;
}
export function canonicalContent(input:GameObservation){
 return JSON.stringify([input.seedStatus==='known'?input.actualSeed:null,input.activeMods??[],input.beta==='unknown'?null:input.beta==='true',input.modVersion,input.bridgeVersion]);
}
export function contentDigest(input:GameObservation){return createHash('sha256').update(canonicalContent(input),'utf8').digest('hex');}
export function contentFailure(input:GameObservation,config:number|Config,profile:{modVersion:string;bridgeVersion:string}):string|undefined{
 if(input.seedStatus!=='known')return 'seed_unknown';if(!seedMatches(typeof config==='number'?{seed:config}:config,input.actualSeed))return 'seed_mismatch';
 if(!input.inventoryComplete)return 'inventory_incomplete';if(input.collectorFailure)return 'collector_failed';
 if(input.modVersion!==profile.modVersion||input.bridgeVersion!==profile.bridgeVersion)return 'observed_version_mismatch';
 if(!input.loaded)return 'not_loaded';return undefined;
}
