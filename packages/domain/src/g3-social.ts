// Player information and social policy, separate from gameplay authority.
// IDs map to client-owned trusted assets. No network-supplied filenames/scripts.
export const SOCIAL_REVISION='g3-social-draft-3';
export interface RetainedHud {hpMilli:number;maxHpMilli:number;sampo:boolean;hpKnown:boolean;sampoKnown:boolean;biome?:string}
// Missing optional observations never erase a previously accepted known fact.
export function mergeRetainedHud(previous:RetainedHud|undefined,next:RetainedHud):RetainedHud{
 const result={...(previous??{hpMilli:0,maxHpMilli:0,sampo:false,hpKnown:false,sampoKnown:false}),...(next.hpKnown?{hpMilli:next.hpMilli,maxHpMilli:next.maxHpMilli,hpKnown:true}:{}),...(next.sampoKnown?{sampo:next.sampo,sampoKnown:true}:{})};
 if(next.biome!==undefined)result.biome=next.biome;return result;
}
export const EMOTES=Object.freeze([
 ['wave',0],['heart',101],['thumbs_up',240],['disapprove',140],['come_on',0],['t_pose',0],['mario',0],['luigi',0],['clap',0],['macarena',0],['the_wave',0],['aggressive_point',0],['default_dance',0],['praise_the_sun',375],['dab',0],['worm',216],['meditate',0],['repose',0],['egyptian',0],['salute',0]
].map(([id,durationFrames])=>Object.freeze({id:id as string,durationFrames:durationFrames as number})));
export const ROBES=Object.freeze(['red','orange','yellow','green','blue','purple']);
export const SOCIAL_CONTROL_CAPABILITIES=Object.freeze(['core.emotes','core.cosmetics']);
// Server-relative age, never a client wall-clock deadline or renewable TTL.
export function emotePresentation(emote:{emoteId:string;active?:boolean},startedAt:number,now:number){
 const definition=EMOTES.find(e=>e.id===emote.emoteId);if(!definition)throw new SocialPolicyError('INVALID_MESSAGE');
 const ageMs=Math.min(0xffffffff,Math.max(0,Math.floor(now-startedAt)));
 return {emoteId:emote.emoteId,active:emote.active===true&&now>=startedAt&&(definition.durationFrames===0||Math.floor(ageMs*60/1000)<definition.durationFrames+2),durationFrames:definition.durationFrames,ageMs};
}
export const SOCIAL_CATALOG=Object.freeze([
 {id:'player.hud',capability:'core.hud',maxPerSecond:2},
 {id:'player.emote',capability:'core.emotes',maxPerSecond:4},
 {id:'player.cosmetic',capability:'core.cosmetics',maxPerSecond:2},
 {id:'world.poi',capability:'core.poi',maxPerSecond:1}
].map(entry=>Object.freeze({...entry,modes:Object.freeze(['core','nemesis','race']),target:'room'})));
export type SocialIntent=
 | {catalogId:'player.hud';hpMilli:number;maxHpMilli:number;sampo:boolean;hpKnown?:boolean;sampoKnown?:boolean;biome?:string}
 | {catalogId:'player.emote';emoteId:string;active:boolean}
 | {catalogId:'player.cosmetic';robeId:string;hat:boolean;amulet:boolean;gem:boolean}
 | {catalogId:'world.poi';text:string;xMilli:number;yMilli:number};
export interface SocialSender {principalId:string;gameSessionId:string;worldToken:string;roomId:string;runId:string;mode:string;live:boolean;capabilities:readonly string[]}
export interface SocialRecipient {principalId:string;gameSessionId:string;roomId:string;runId:string;mode:string;live:boolean;capabilities:readonly string[]}
export class SocialPolicyError extends Error {constructor(readonly code:'INVALID_MESSAGE'|'FORBIDDEN'|'CAPACITY'|'TOO_SOON'|'TOO_CLOSE'){super(code);}}
const require=(condition:unknown,code:SocialPolicyError['code']='INVALID_MESSAGE'):void=>{if(!condition)throw new SocialPolicyError(code);};
const integer=(value:unknown,max:number,min=0)=>Number.isInteger(value)&&Number(value)>=min&&Number(value)<=max;
export function authorizeSocial(sender:SocialSender,input:unknown,recipients:readonly SocialRecipient[]){
 require(input&&typeof input==='object'&&!Array.isArray(input));const p=input as Record<string,unknown>;
 const entry=SOCIAL_CATALOG.find(e=>e.id===p.catalogId);
 require(entry&&sender.live&&sender.principalId&&sender.gameSessionId&&sender.worldToken&&sender.roomId&&sender.runId&&entry.modes.includes(sender.mode)&&sender.capabilities.includes(entry.capability),'FORBIDDEN');
 const keys:Record<string,readonly string[]>={'player.hud':['catalogId','hpMilli','maxHpMilli','sampo','hpKnown','sampoKnown','biome'],'player.emote':['catalogId','emoteId','active'],'player.cosmetic':['catalogId','robeId','hat','amulet','gem'],'world.poi':['catalogId','text','xMilli','yMilli']};
 require((entry!.id==='player.hud'||Object.keys(p).length===keys[entry!.id]!.length)&&Object.keys(p).every(k=>keys[entry!.id]!.includes(k)));
 switch(entry!.id){
  case 'player.hud':require(integer(p.hpMilli,2147483647)&&integer(p.maxHpMilli,2147483647)&&typeof p.sampo==='boolean'&&typeof p.hpKnown==='boolean'&&typeof p.sampoKnown==='boolean');
   require(p.hpKnown===false?p.hpMilli===0&&p.maxHpMilli===0:Number(p.maxHpMilli)>=1);
   require(p.sampoKnown!==false||p.sampo===false);
   require(p.biome===undefined||typeof p.biome==='string'&&p.biome.length>0&&new TextEncoder().encode(p.biome).length<=128&&!/[\u0000-\u001f\u007f-\u009f\ud800-\udfff]/u.test(p.biome));break;
  case 'player.emote':require(EMOTES.some(e=>e.id===p.emoteId)&&typeof p.active==='boolean');break;
  case 'player.cosmetic':require(ROBES.includes(String(p.robeId))&&['hat','amulet','gem'].every(k=>typeof p[k]==='boolean'));break;
  case 'world.poi':require(typeof p.text==='string'&&p.text.trim().length>0&&[...p.text].length<=99&&new TextEncoder().encode(p.text).length<=396&&!/[\u0000-\u001f\u007f-\u009f\ud800-\udfff]/u.test(p.text)&&integer(p.xMilli,2147483647,-2147483647)&&integer(p.yMilli,2147483647,-2147483647));break;
 }
 const selected=recipients.filter(r=>r.live&&r.roomId===sender.roomId&&r.runId===sender.runId&&r.mode===sender.mode&&r.capabilities.includes(entry!.capability));
 require(selected.length<=32,'CAPACITY');
 return {revision:SOCIAL_REVISION,principalId:sender.principalId,gameSessionId:sender.gameSessionId,worldToken:sender.worldToken,roomId:sender.roomId,runId:sender.runId,provenance:'validated-client-report',payload:{...p} as SocialIntent,recipientSessionIds:[...new Set(selected.map(r=>r.gameSessionId))]};
}
// Pure guard for use with transactionally loaded server state, never app-provided fences.
// The opt-in social service loads these fences under its database transaction gate.
export function checkSocialBudget(intent:SocialIntent,now:number,state:{windowAt:number;windowCount:number;lastPoiAt?:number;pois?:readonly {xMilli:number;yMilli:number}[]}){
 require(Number.isSafeInteger(now)&&now>=0&&Number.isSafeInteger(state.windowAt)&&state.windowAt>=0&&state.windowAt<=now&&integer(state.windowCount,4096));
 const entry=SOCIAL_CATALOG.find(e=>e.id===intent.catalogId)!;
 const count=state.windowAt<=now&&now<state.windowAt+1000?state.windowCount+1:1;
 if(intent.catalogId==='world.poi'){
  require(state.lastPoiAt===undefined||Number.isSafeInteger(state.lastPoiAt)&&state.lastPoiAt>=0);
  require(state.lastPoiAt===undefined||now-state.lastPoiAt>=30000,'TOO_SOON');
  const pois=state.pois??[];require(pois.length<256,'CAPACITY');
  for(const poi of pois){require(integer(poi.xMilli,2147483647,-2147483647)&&integer(poi.yMilli,2147483647,-2147483647));require(Math.hypot(intent.xMilli-poi.xMilli,intent.yMilli-poi.yMilli)>=50000,'TOO_CLOSE');}
 }
 require(count<=entry.maxPerSecond,'CAPACITY');
 return {windowAt:count===1?now:state.windowAt,windowCount:count};
}
