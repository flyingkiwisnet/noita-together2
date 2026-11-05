import type {Config} from '../../contracts/g1/g1.js';
import {G1_LIMITS} from '../../contracts/g1/g1.js';
export interface CapabilityRegistry {capabilities:string[];modes:string[];settingKeys:string[]}
// The registry is injected; fixture entries are not the final gameplay inventory.
export function compatibleConfiguration(config:Config,registry:CapabilityRegistry):boolean {
 const settings=config.settings??[],caps=config.requiredCapabilities??[];
 return new TextEncoder().encode(JSON.stringify(config)).length<=G1_LIMITS.configBytes&&config.seed!==undefined&&!!config.mode&&registry.modes.includes(config.mode)
  &&new Set(settings.map(s=>s.key)).size===settings.length
  &&settings.every(s=>!!s.key&&s.value!==undefined&&registry.settingKeys.includes(s.key))
  &&new Set(caps).size===caps.length&&caps.every(c=>registry.capabilities.includes(c));
}
export interface RosterMember {desktopId:string;present:boolean;readyConfigRevision:number|null}
export function rosterFailure(members:RosterMember[],selected:string[],excluded:string[],configRevision:number):'ROSTER_MISMATCH'|'NOT_READY'|undefined {
 const all=[...selected,...excluded];
 if(selected.length===0||all.length!==members.length||new Set(all).size!==all.length||!members.every(m=>all.includes(m.desktopId)))return 'ROSTER_MISMATCH';
 if(!members.filter(m=>selected.includes(m.desktopId)).every(m=>m.present&&m.readyConfigRevision===configRevision))return 'NOT_READY';
 return undefined;
}
export function admissionFailure(input:{kind:string;entitled:boolean;existingMember:boolean;inOtherRoom:boolean;state:string;memberCount:number;capacity?:number}):'FORBIDDEN'|'NOT_FOUND'|'INVALID_STATE'|'CONFLICT'|'CAPACITY'|undefined {
 if(input.kind!=='desktop')return 'FORBIDDEN';
 if(!input.existingMember&&!input.entitled)return 'NOT_FOUND';
 if(!input.existingMember&&!['waiting','ended'].includes(input.state))return 'INVALID_STATE';
 if(input.inOtherRoom)return 'CONFLICT';
 if(!input.existingMember&&input.memberCount>=(input.capacity??32))return 'CAPACITY';
 return undefined;
}
// A generated/prepared or suspended world must be deliberately cancelled/ended.
export function roomDeletionFailure(v:{kind:string;registered:boolean;host:boolean;state:string;prepared:boolean;attached:boolean}):'FORBIDDEN'|'INVALID_STATE'|undefined{
 if(v.kind!=='browser'||!v.registered||!v.host)return 'FORBIDDEN';
 if(!['waiting','ended'].includes(v.state)||v.prepared||v.attached)return 'INVALID_STATE';
 return undefined;
}
