import {validate} from '../../contracts/src/g3-c2.js';
import type {Rules,Payload} from '../../contracts/g3-c2/g3-c2.js';
import {HOURGLASS_CATALOGUE} from './g3-hourglass.js';
export const C2_DEFAULTS={NT_sync_steve:true} as const;
export const C2_SKIP_REASONS=['sharing_disabled','recipient_terminal','already_angered','effect_not_applicable','unsupported_content','run_ended'] as const;
export function normalizeC2Rules(input:Rules){validate('Rules',input);return {...C2_DEFAULTS,...input};}
export function normalizeC2Payload(p:Payload){validate('Payload',p);if(p.hourglass){const v=p.hourglass,entry=HOURGLASS_CATALOGUE.entries.find(e=>e.id===v.effectId);if(!entry||entry.materialId!==v.materialId||v.catalogueRevision!==HOURGLASS_CATALOGUE.revision)throw Error('UNSUPPORTED_CONTENT');return {hourglass:{effectId:entry.id,materialId:entry.materialId,catalogueRevision:HOURGLASS_CATALOGUE.revision}};}
 if(p.steve?.progression===undefined&&p.steve||p.completed?.progression===undefined&&p.completed||p.pickup?.orbCount===undefined&&p.pickup)throw Error('INVALID_MESSAGE');
 if(p.pickup)return {pickup:{encounterId:p.pickup.encounterId,arenaToken:p.pickup.arenaToken,orbCount:p.pickup.orbCount??0}};
 if(p.steve)return {steve:{progression:p.steve.progression??0}};return {completed:{progression:p.completed?.progression??0}};}
export function c2Stage(stage:string,required:readonly {near:boolean;pickedUp:boolean}[]){if(!required.length)return stage;if(stage==='locked'&&required.every(p=>p.near))stage='unlocked';if(stage==='unlocked'&&required.every(p=>p.pickedUp))stage='battle';return stage;}
export function sampoNameIndex(orbs:number){if(!Number.isInteger(orbs))throw Error('INVALID_MESSAGE');const n=Math.max(0,Math.min(33,orbs));return n>=14&&n<=32?13:n;}
export function sampoNear(dx:number,dy:number){return Number.isFinite(dx)&&Number.isFinite(dy)&&dx*dx+dy*dy<200*200;}
