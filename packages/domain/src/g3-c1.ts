import {createRequire} from 'node:module';
import {validate} from '../../contracts/src/g3-c1.js';
import type {Rules,Payload} from '../../contracts/g3-c1/g3-c1.js';
const require=createRequire(import.meta.url);
export const C1_CATALOGUE=require('../../contracts/g3-c1/catalogue.json') as {revision:string;perks:{id:string;available:boolean}[];teamPerks:string[];bannedPerks:string[];orbs:{id:number}[];materials:string[]};
export const C1_DEFAULTS:Required<Rules>={NT_sync_hearts:true,NT_sync_orbs:true,NT_sync_shift:true,NT_team_perks:true,NT_sync_perks:false,NT_alt_heartache:false,NT_ban_perks:false,NT_sync_orbs_no_curse:false,NT_ondeath_kick:false,NT_sync_orb_count:0,NT_death_penalty:'end'};
export class C1PolicyError extends Error{constructor(readonly code:string){super(code);}}
const need=(value:unknown,code='INVALID_MESSAGE')=>{if(!value)throw new C1PolicyError(code);};
export function normalizeC1Rules(input:Rules){validate('Rules',input);const rules={...C1_DEFAULTS,...input};if(!rules.NT_sync_orbs)rules.NT_sync_orb_count=0;return rules;}
export function normalizeC1Payload(input:Payload):Payload{
 validate('Payload',input);
 if(input.heart)return {heart:{}};
 if(input.orb){const orbId=input.orb.orbId??0;need(C1_CATALOGUE.orbs.some(v=>v.id===orbId),'UNSUPPORTED_CONTENT');return {orb:{orbId}};}
 if(input.perk){need(C1_CATALOGUE.perks.some(v=>v.id===input.perk!.perkId&&v.available),'UNSUPPORTED_CONTENT');return {perk:{perkId:input.perk.perkId!}};}
 if(input.fungal){const p=input.fungal;need(p.fromMaterials&&p.fromMaterials.length>0&&new Set(p.fromMaterials).size===p.fromMaterials.length);need([...p.fromMaterials!,p.toMaterial!,...(p.displayMaterial?[p.displayMaterial]:[])].every(v=>C1_CATALOGUE.materials.includes(v)),'UNSUPPORTED_CONTENT');return {fungal:{fromMaterials:[...p.fromMaterials!],toMaterial:p.toMaterial!,...(p.displayMaterial?{displayMaterial:p.displayMaterial}:{})}};}
 need(input.death?.state&&input.death.cause);need(input.death!.progression===undefined||input.death!.state==='won');return {death:{state:input.death!.state!,cause:input.death!.cause!,...(input.death!.state==='won'&&input.death!.progression!==undefined?{progression:input.death!.progression}:{})}};
}
export function c1SharingSkip(payload:Payload,rules:Required<Rules>):string|undefined{
 if(payload.heart&&!rules.NT_sync_hearts||payload.orb&&!rules.NT_sync_orbs||payload.fungal&&!rules.NT_sync_shift)return 'sharing_disabled';
 if(payload.perk){if(rules.NT_ban_perks&&C1_CATALOGUE.bannedPerks.includes(payload.perk.perkId!))return 'perk_banned';if(!rules.NT_sync_perks&&!(rules.NT_team_perks&&C1_CATALOGUE.teamPerks.includes(payload.perk.perkId!)))return 'sharing_disabled';}
}
export const C1_SKIP_REASONS=['sharing_disabled','perk_banned','orb_owned','orb_cap','fungal_cooldown','fungal_limit','fungal_noop','gold_world','unsupported_content','recipient_terminal','source_terminal','run_ended','effect_not_applicable'] as const;
