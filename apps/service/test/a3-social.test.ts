import test from 'node:test';import assert from 'node:assert/strict';
import {EMOTES,emotePresentation,checkSocialBudget,type SocialIntent} from '../../../packages/domain/src/g3-social.js';
import {encode,decode,validate} from '../../../packages/contracts/src/g3-social.js';
import {SocialAuthority} from '../src/g3/social.js';
test('all legacy finite boundaries and unbounded loops use original relative age',()=>{
 for(const e of EMOTES){
  const start=1000,p={emoteId:e.id,active:true};assert.equal(emotePresentation(p,start,start).active,true);
  if(e.durationFrames){const boundary=Math.ceil((e.durationFrames+2)*1000/60);assert.equal(emotePresentation(p,start,start+boundary-1).active,true);assert.equal(emotePresentation(p,start,start+boundary).active,false);}
  else assert.equal(emotePresentation(p,start,start+3600000).active,true);
  assert.equal(emotePresentation({...p,active:false},start,start).active,false);
 }
 assert.equal(emotePresentation({emoteId:'wave',active:true},1000,999).active,false);
 assert.equal(emotePresentation({emoteId:'wave',active:true},0,2**40).ageMs,0xffffffff);
 assert.throws(()=>emotePresentation({emoteId:'evil',active:true},0,0));
});
test('normal four-emote/two-cosmetic policy windows remain exact and independent of fixture setup',()=>{
 for(const [intent,limit]of [[{catalogId:'player.emote',emoteId:'wave',active:true},4],[{catalogId:'player.cosmetic',robeId:'purple',hat:false,amulet:false,gem:false},2]] as [SocialIntent,number][]){let state={windowAt:1000,windowCount:0};for(let i=0;i<limit;i++)state=checkSocialBudget(intent,1000,state);assert.throws(()=>checkSocialBudget(intent,1999,state),/CAPACITY/);assert.equal(checkSocialBudget(intent,2000,state).windowCount,1);}
});
test('social request maintenance keeps eligibility cleanup while historical retention remains periodic',async()=>{
 const a=new SocialAuthority(()=>'',[],()=>31*86400000),queries:string[]=[];const tx={query:async(sql:string)=>{queries.push(sql);return {rows:[]};}} as any;
 await a.maintain(tx,false);assert.equal(queries.length,4);assert.ok(queries.some(q=>q.includes('g3_social_hud_state')));assert.ok(queries.some(q=>q.includes('g3_social_presentation')));assert.ok(queries.every(q=>!q.includes('coalesce(n.ended_at,n.created_at)')));
 queries.length=0;await a.maintain(tx);assert.equal(queries.length,9);assert.equal(queries.filter(q=>q.includes('coalesce(n.ended_at,n.created_at)')).length,5);
});
test('presentation codec requires revisions, bounded IDs and forbids renewed TTL metadata',()=>{
 const p={presentationRevision:1,emote:{emoteId:'wave',active:true,durationFrames:0,ageMs:31000}};
 assert.equal((decode('Projection',encode('Projection',p)).emote as any).ageMs,31000);
 for(const bad of [{...p,presentationRevision:0},{emote:p.emote},{...p,expiresAt:'2099-01-01'},{...p,hudStateRevision:1},{...p,emote:{...p.emote,emoteId:'evil'}},{...p,emote:{...p.emote,ageMs:-1}}])assert.throws(()=>validate('Projection',bad));
 assert.throws(()=>validate('Cosmetic',{robeId:'cyan'}));
 assert.throws(()=>validate('Projection',{hud:{},hudStateRevision:1,presentationRevision:1}));
});
