import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {EMOTES,ROBES,authorizeSocial,checkSocialBudget,type SocialSender,type SocialRecipient} from '../../../packages/domain/src/g3-social.js';
const sender:SocialSender={principalId:'server-principal',gameSessionId:'server-game',worldToken:'world-雪',roomId:'room',runId:'run',mode:'core',live:true,capabilities:['core.hud','core.emotes','core.cosmetics','core.poi']};
const recipient:SocialRecipient={...sender,gameSessionId:'recipient'};
test('trusted social catalogue retains every audited emote timing and robe',async()=>{
 const audit=JSON.parse(await readFile(new URL('../../../docs/rewrite/legacy-player-experience.json',import.meta.url),'utf8'));
 assert.deepEqual(EMOTES,audit.catalogues.emotes.map((e:{id:string;duration_frames:number})=>({id:e.id,durationFrames:e.duration_frames})));
 assert.deepEqual(ROBES,audit.catalogues.robe_colors);
 const provisional=JSON.parse(await readFile(new URL('../../../packages/contracts/g3/social-provisional.json',import.meta.url),'utf8'));
 assert.deepEqual(provisional.emotes,EMOTES);assert.deepEqual(provisional.robes,ROBES);
 for(const sample of provisional.samples)assert.doesNotThrow(()=>authorizeSocial(sender,sample,[recipient]));
 for(const emote of EMOTES)for(const active of [true,false])assert.equal(authorizeSocial(sender,{catalogId:'player.emote',emoteId:emote.id,active},[recipient]).gameSessionId,sender.gameSessionId);
 for(const robeId of ROBES)assert.equal(authorizeSocial(sender,{catalogId:'player.cosmetic',robeId,hat:true,amulet:false,gem:true},[recipient]).payload.catalogId,'player.cosmetic');
});
test('social authorization derives sender and bounds live room/mode/capability/targets',()=>{
 const p={catalogId:'player.hud',hpMilli:100000,maxHpMilli:100000,sampo:false,hpKnown:true,sampoKnown:true};
 assert.deepEqual(authorizeSocial(sender,p,[recipient,{...recipient,roomId:'foreign'},{...recipient,runId:'foreign'},{...recipient,live:false},{...recipient,mode:'nemesis'},{...recipient,capabilities:[]}]).recipientSessionIds,['recipient']);
 for(const s of [{...sender,live:false},{...sender,mode:'unknown'},{...sender,capabilities:[]}])assert.throws(()=>authorizeSocial(s,p,[recipient]));
 assert.doesNotThrow(()=>authorizeSocial({...sender,mode:'nemesis'},p,[{...recipient,mode:'nemesis'}]));
 for(const extra of [{principalId:'forged'},{targetGameSessionId:'foreign'},{path:'evil.lua'}])assert.throws(()=>authorizeSocial(sender,{...p,...extra},[recipient]));
 for(const bad of [{...p,hpMilli:NaN},{...p,maxHpMilli:0},{catalogId:'player.emote',emoteId:'../evil.xml',active:true},{catalogId:'player.cosmetic',robeId:'cyan',hat:true,amulet:false,gem:false}])assert.throws(()=>authorizeSocial(sender,bad,[recipient]));
 assert.throws(()=>authorizeSocial(sender,p,Array.from({length:33},(_,i)=>({...recipient,gameSessionId:String(i)}))));
});
test('POI preserves Unicode text, 99-character bound, cooldown and proximity reasons',()=>{
 const poi={catalogId:'world.poi' as const,text:'雪'.repeat(99),xMilli:0,yMilli:0};
 assert.doesNotThrow(()=>authorizeSocial(sender,poi,[recipient]));
 for(const text of ['雪'.repeat(100),'\u0000','\ud800',' '])assert.throws(()=>authorizeSocial(sender,{...poi,text},[recipient]));
 assert.doesNotThrow(()=>checkSocialBudget(poi,30000,{windowAt:0,windowCount:0,lastPoiAt:0,pois:[{xMilli:50000,yMilli:0}]}));
 assert.throws(()=>checkSocialBudget(poi,29999,{windowAt:0,windowCount:0,lastPoiAt:0}),/TOO_SOON/);
 assert.throws(()=>checkSocialBudget(poi,30000,{windowAt:0,windowCount:0,pois:[{xMilli:49999,yMilli:0}]}),/TOO_CLOSE/);
 assert.throws(()=>checkSocialBudget(poi,30000,{windowAt:30000,windowCount:1}),/CAPACITY/);
 assert.throws(()=>checkSocialBudget(poi,30000,{windowAt:30001,windowCount:0}));
 assert.throws(()=>checkSocialBudget(poi,30000,{windowAt:0,windowCount:0,pois:Array.from({length:256},()=>({xMilli:100000,yMilli:100000}))}),/CAPACITY/);
});
