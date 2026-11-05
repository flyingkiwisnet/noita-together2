// INTERNAL provisional audit rules. No network earning/offer grant or game RNG.
import {createHash} from 'node:crypto';
import {NemesisError,NEMESIS_CATALOGUE,nemesisContext,nemesisOfferPrice,validateNemesisActiveActor,type NemesisActor} from './g3-nemesis.js';

export const NEMESIS_PROVENANCE_REVISION='g3-nemesis-provenance-internal-1';
export const NEMESIS_KILL_CAPACITY=2048;
const uuid=(v:unknown):v is string=>typeof v==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(v);
function demand(v:unknown,code:NemesisError['code']='INVALID_MESSAGE'):asserts v {if(!v)throw new NemesisError(code);}
function keys(v:object,expected:string){demand(Object.keys(v).sort().join(',')===expected);}
export const nemesisProvenanceDigest=(v:unknown)=>createHash('sha256').update(JSON.stringify(v,(_k,x)=>x&&typeof x==='object'&&!Array.isArray(x)?Object.fromEntries(Object.keys(x).sort().map(k=>[k,x[k]])):x)).digest('hex');
function active(a:NemesisActor){validateNemesisActiveActor(a);demand(!/[\ud800-\udfff]/u.test(a.worldToken),'FORBIDDEN');}
export function nemesisF64(bits:unknown){demand(typeof bits==='string'&&/^[0-9a-f]{16}$/.test(bits));const value=Buffer.from(bits,'hex').readDoubleBE();demand(Number.isFinite(value)&&!Object.is(value,-0));return value;}
export function nemesisF64Bits(value:number){demand(Number.isFinite(value)&&!Object.is(value,-0));const bytes=Buffer.alloc(8);bytes.writeDoubleBE(value);return bytes.toString('hex');}
export interface NemesisKillEvidence {
 killId:string;victimId:string;playerEntityId:string;responsibleEntityId:string;responsibleParentId:string|null;
 maxHpBits:string|null;nemesisEnemy:boolean;
}
export interface NemesisKillOriginal {
 revision:typeof NEMESIS_PROVENANCE_REVISION;source:ReturnType<typeof nemesisContext>;evidence:NemesisKillEvidence;
 digest:string;points:number;attribution:'player'|'child'|'unattributed';enemyBroadcastEligible:boolean;provenance:'validated-client-report';
}
export function nemesisKillOriginal(sender:NemesisActor,input:NemesisKillEvidence):NemesisKillOriginal {
 active(sender);demand(input&&typeof input==='object'&&!Array.isArray(input));keys(input,'killId,maxHpBits,nemesisEnemy,playerEntityId,responsibleEntityId,responsibleParentId,victimId');
 demand([input.killId,input.victimId,input.playerEntityId,input.responsibleEntityId].every(uuid)&&(input.responsibleParentId===null||uuid(input.responsibleParentId))&&typeof input.nemesisEnemy==='boolean');
 const hp=input.maxHpBits===null?null:nemesisF64(input.maxHpBits);demand(hp===null||hp>=0);
 const attribution=input.responsibleEntityId===input.playerEntityId?'player':input.responsibleParentId===input.playerEntityId?'child':'unattributed';
 // IEEE754 binary64 multiplication THEN floor, not an exact rational product floor.
 // Conversion of actual Noita max_hp into these bits still needs the client adapter.
 const points=attribution==='unattributed'||hp===null?0:Math.floor(hp*10);demand(Number.isSafeInteger(points)&&points>=0&&points<=0xffffffff,'CAPACITY');
 const body={revision:NEMESIS_PROVENANCE_REVISION as typeof NEMESIS_PROVENANCE_REVISION,source:nemesisContext(sender),evidence:structuredClone(input)};
 return {...body,digest:nemesisProvenanceDigest(body),points,attribution,enemyBroadcastEligible:attribution!=='unattributed'&&!input.nemesisEnemy,provenance:'validated-client-report'};
}
export function appendNemesisKill(sender:NemesisActor,input:NemesisKillEvidence,originals:NemesisKillOriginal[]){
 const next=nemesisKillOriginal(sender,input),old=originals.find(x=>x.source.runId===sender.runId&&x.evidence.killId===input.killId);
 if(old){demand(old.digest===next.digest,'CONFLICT');return structuredClone(old);}
 demand(originals.length<NEMESIS_KILL_CAPACITY,'CAPACITY');
 // A new kill UUID cannot re-credit the same original victim in the same run.
 demand(!originals.some(x=>x.source.runId===sender.runId&&x.evidence.victimId===input.victimId),'CONFLICT');
 originals.push(next);return structuredClone(next);
}
export function nemesisEarnedPoints(originals:readonly NemesisKillOriginal[],runId:string,principalId:string,debits:readonly {cost:number}[]){
 const credit=originals.filter(x=>x.source.runId===runId&&x.source.principalId===principalId).reduce((n,x)=>n+x.points,0),spent=debits.reduce((n,x)=>{demand(Number.isInteger(x.cost)&&x.cost>=35&&x.cost<=1265,'FORBIDDEN');return n+x.cost;},0),points=credit-spent;
 demand(Number.isSafeInteger(credit)&&Number.isSafeInteger(points)&&points>=0&&points<=0xffffffff,'CAPACITY');return points;
}

export interface NemesisShopEvidence {groupId:string;spawnKind:'eater'|'spitter';xBits:string;yBits:string;choices:{offerId:string;abilityId:string}[]}
export interface NemesisShopOriginal {
 revision:typeof NEMESIS_PROVENANCE_REVISION;source:ReturnType<typeof nemesisContext>;evidence:NemesisShopEvidence;digest:string;
 offers:{offerId:string;abilityId:string;x:number;y:number;level:number;tier:number;cost:number;weight:number}[];
 provenance:'validated-client-report';rngVerified:false;
}
const tiers:Readonly<Record<number,number>>={2:1,5:2,9:3,12:4,16:5,20:6,25:6};
export function appendNemesisShop(sender:NemesisActor,input:NemesisShopEvidence,originals:NemesisShopOriginal[]){
 active(sender);demand(input&&typeof input==='object'&&!Array.isArray(input));keys(input,'choices,groupId,spawnKind,xBits,yBits');
 demand(uuid(input.groupId)&&['eater','spitter'].includes(input.spawnKind)&&Array.isArray(input.choices)&&input.choices.length===3);
 const x=nemesisF64(input.xBits),y=nemesisF64(input.yBits);demand(Math.abs(x)<=1e7&&Math.abs(y)<=1e7);
 const body={revision:NEMESIS_PROVENANCE_REVISION as typeof NEMESIS_PROVENANCE_REVISION,source:nemesisContext(sender),evidence:structuredClone(input)},digest=nemesisProvenanceDigest(body),old=originals.find(s=>s.source.runId===sender.runId&&s.evidence.groupId===input.groupId);
 if(old){demand(old.digest===digest,'CONFLICT');return structuredClone(old);}
 demand(originals.length<256,'CAPACITY');const sourceKey=nemesisProvenanceDigest(nemesisContext(sender)),used=new Set(originals.filter(s=>nemesisProvenanceDigest(s.source)===sourceKey).flatMap(s=>s.offers.map(o=>o.tier+':'+o.abilityId))),ids=new Set<string>();
 const offers=input.choices.map((o,i)=>{
  demand(o&&typeof o==='object'&&!Array.isArray(o));keys(o,'abilityId,offerId');demand(uuid(o.offerId)&&!ids.has(o.offerId)&&!originals.some(s=>s.source.runId===sender.runId&&s.offers.some(p=>p.offerId===o.offerId)),'CONFLICT');ids.add(o.offerId);
  const ox=x+(input.spawnKind==='eater'?2:-2),oy=y-3-i*25,level=Math.floor(oy/512),tier=tiers[level];demand(tier!==undefined);
  const entry=NEMESIS_CATALOGUE.entries.find(e=>e.id===o.abilityId),weight=entry?.depthTierWeights[tier-1];demand(weight!==undefined&&weight>0);demand(!used.has(tier+':'+o.abilityId),'CONFLICT');used.add(tier+':'+o.abilityId);
  return {...o,x:ox,y:oy,level,tier,cost:nemesisOfferPrice(level),weight};
 });
 const record:NemesisShopOriginal={...body,digest,offers,provenance:'validated-client-report',rngVerified:false};originals.push(record);return structuredClone(record);
}
// Pure geometry planning only: no EntityKill/spawn and no replacement group issuance.
export function nemesisShopClosure(shop:NemesisShopOriginal,offerId:string,shops:readonly NemesisShopOriginal[],recipientGameSessionId:string){
 demand(uuid(recipientGameSessionId));const offer=shop.offers.find(o=>o.offerId===offerId);demand(offer,'NOT_FOUND');const x=Math.floor(offer.x),y=Math.floor(offer.y),contextKey=nemesisProvenanceDigest(shop.source);
 return {coordinateKey:'NEMESIS_USED_ABILITY_'+x+'_'+y,recipientGameSessionId,offerIds:shops.filter(s=>nemesisProvenanceDigest(s.source)===contextKey).flatMap(s=>s.offers).filter(o=>o.offerId===offer.offerId||Math.hypot(o.x-offer.x,o.y-(offer.y-25))<=1000).map(o=>o.offerId)};
}
