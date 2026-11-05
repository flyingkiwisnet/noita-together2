import {createRequire} from 'node:module';
import {validate} from '../../contracts/src/g3.js';
import type {ItemDescriptor,ItemWand,ItemSpell,ItemCard,Scope} from '../../contracts/g3/g3.js';
const require=createRequire(import.meta.url);
export const ITEM_CATALOGUE=require('../../contracts/g3-items/catalogue.json') as {revision:string;actions:any[];materials:any[];sprites:any[];objects:any[];wandSort:{key:string;ascending:boolean;enabled:boolean}[];containerLabels:Record<string,string[]>};
export const ITEM_LIMITS={descriptorBytes:131072,bank:2048,pending:256,operations:4096,page:40,noticePage:8,pendingPage:8,noticeMs:10000,inventoryBytes:131072,inventorySlots:8,deck:1024,permanent:256,materials:1024} as const;
export const ITEM_CAPABILITY='core.items';
export const ITEM_SETTINGS={wand:'NT_send_wands',container:'NT_send_flasks',object:'NT_send_items'} as const;
const actions=new Map(ITEM_CATALOGUE.actions.map(v=>[v.id,v])),materials=new Map(ITEM_CATALOGUE.materials.map(v=>[v.id,v])),sprites=new Map(ITEM_CATALOGUE.sprites.map(v=>[v.id,v])),objects=new Map(ITEM_CATALOGUE.objects.map(v=>[v.id,v]));
export class ItemPolicyError extends Error{constructor(readonly code:string){super(code);}}
export function itemDemand(ok:unknown,code='INVALID_MESSAGE'):asserts ok{if(!ok)throw new ItemPolicyError(code);}
const utf8=(s:string)=>new TextDecoder('utf-8',{fatal:true}).decode(new TextEncoder().encode(s))===s;
export function decimal(v:unknown,min=-3.4028234663852886e38,max=3.4028234663852886e38){itemDemand(typeof v==='string'&&v.length<=64&&/^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]{1,3})?$/.test(v));const n=Number(v);itemDemand(Number.isFinite(n)&&n>=min&&n<=max);return v;}
export function integer(v:unknown,min=-2147483648,max=2147483647){itemDemand(typeof v==='string'&&/^(?:0|-?[1-9][0-9]*)$/.test(v)&&Number.isSafeInteger(Number(v))&&Number(v)>=min&&Number(v)<=max);return v;}
export function itemKind(item:ItemDescriptor){return item.spell?'spell':item.wand?'wand':item.container?'container':'object';}
function spell(s:ItemSpell):ItemSpell{itemDemand(s&&actions.has(s.actionId!),'UNSUPPORTED_CONTENT');return {actionId:s.actionId!,usesRemaining:integer(s.usesRemaining,-1)};}
function cards(values:ItemCard[]|undefined,max:number):ItemCard[]{const a=values??[];itemDemand(a.length<=max);let previous=-1;return a.map(input=>{const c={...input,slot:input.slot??0};itemDemand(Number.isInteger(c.slot)&&c.slot!>=0&&c.slot!<=2147483647&&c.slot!>previous&&c.spell);previous=c.slot!;return {slot:c.slot!,spell:spell(c.spell!)};});}
export function normalizeWand(w:ItemWand):ItemWand{
 validate('ItemWand',w);itemDemand(sprites.has(w.spriteId!),'UNSUPPORTED_CONTENT');itemDemand(typeof (w.named??false)==='boolean'&&typeof (w.shuffleDeckWhenEmpty??false)==='boolean');itemDemand(typeof (w.uiName??'')==='string'&&utf8(w.uiName??'')&&!(w.uiName??'').includes('\0'));
 return {named:w.named??false,uiName:w.uiName??'',spriteId:w.spriteId!,gripX:decimal(w.gripX),gripY:decimal(w.gripY),tipX:decimal(w.tipX),tipY:decimal(w.tipY),manaMax:decimal(w.manaMax,0),manaChargeSpeed:decimal(w.manaChargeSpeed),reloadTime:integer(w.reloadTime),actionsPerRound:integer(w.actionsPerRound),deckCapacity:integer(w.deckCapacity,0),shuffleDeckWhenEmpty:w.shuffleDeckWhenEmpty??false,spreadDegrees:decimal(w.spreadDegrees),speedMultiplier:decimal(w.speedMultiplier),fireRateWait:integer(w.fireRateWait),deck:cards(w.deck,ITEM_LIMITS.deck),permanent:cards(w.permanent,ITEM_LIMITS.permanent)};
}
export function normalizeItem(item:ItemDescriptor):ItemDescriptor{
 validate('ItemDescriptor',item);itemDemand(item.catalogueRevision===ITEM_CATALOGUE.revision,'INCOMPATIBLE_CATALOGUE');let result:ItemDescriptor={catalogueRevision:ITEM_CATALOGUE.revision};
 if(item.spell)result.spell=spell(item.spell);else if(item.wand)result.wand=normalizeWand(item.wand);else if(item.container){const c=item.container;itemDemand(['potion','powder'].includes(c.containerType!));const seen=new Set<string>();const contents=(c.contents??[]).map(m=>{itemDemand(materials.has(m.materialId!),'UNSUPPORTED_CONTENT');itemDemand(!seen.has(m.materialId!));seen.add(m.materialId!);const amount=decimal(m.amount,Number.MIN_VALUE,Number.MAX_VALUE);return {materialId:m.materialId!,amount};});result.container={containerType:c.containerType!,color:integer(c.color,0,4294967295),contents};}else{itemDemand(item.object&&objects.get(item.object.objectId!)?.available,'UNSUPPORTED_CONTENT');result.object={objectId:item.object.objectId!};}
 itemDemand(Buffer.byteLength(JSON.stringify(result))<=ITEM_LIMITS.descriptorBytes,'CAPACITY');return result;
}
export function itemPolicy(config:{mode?:string;settings?:{key?:string;value?:string}[]},kind:string){const key=ITEM_SETTINGS[kind as keyof typeof ITEM_SETTINGS],setting=key?config.settings?.find(s=>s.key===key):undefined;return config.mode==='core'&&(!key||!setting||setting.value==='true');}
export function itemPresentation(item:ItemDescriptor){
 const kind=itemKind(item),words=new Set<string>();let displayName='',nameKey='',iconId='',spellType=0,usesRemaining='';const add=(v:string[])=>v.forEach(s=>words.add(s));
 if(item.spell){const a=actions.get(item.spell.actionId!)!;displayName=a.labels[0]??a.id;nameKey=a.nameKey;iconId=a.id;spellType=a.type;usesRemaining=item.spell.usesRemaining!;add([a.id,...a.labels]);}
 if(item.wand){const w=item.wand;displayName=w.uiName||'Wand';nameKey=w.named?'':w.uiName??'';iconId=w.spriteId!;add([displayName]);for(const c of [...w.deck!,...w.permanent!]){const a=actions.get(c.spell!.actionId!)!;add([a.id,...a.labels]);}}
 if(item.container){const c=item.container;displayName=c.containerType==='powder'?'Powder pouch':'Potion';nameKey=c.containerType==='powder'?'$item_powder_stash_3':'$item_potion';iconId=c.containerType!;add([displayName,...ITEM_CATALOGUE.containerLabels[c.containerType!]!]);for(const m of c.contents!){const a=materials.get(m.materialId!)!;add([a.id,...a.labels]);}}
 if(item.object){const a=objects.get(item.object.objectId!)!;displayName=a.labels[0]??a.id;nameKey=a.nameKey;iconId=a.id;add([a.id,...a.labels]);}
 return {kind,displayName,nameKey,iconId,spellType,usesRemaining,searchText:[...words].join('\n').toLowerCase(),wandSort:ITEM_CATALOGUE.wandSort.map(({key})=>item.wand?key==='shuffleDeckWhenEmpty'?(item.wand.shuffleDeckWhenEmpty?1:0):Number((item.wand as any)[key]):0)};
}
export function itemOriginal(principalId:string,scope:Scope,direction:'deposit'|'claim',payload:unknown){return {v:1,principalId,scope:{roomId:scope.roomId,runId:scope.runId,runEpoch:scope.runEpoch,configRevision:scope.configRevision,attachmentId:scope.attachmentId,worldToken:scope.worldToken},kind:'item.'+direction,payload};}

