import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {encode,decode,schema} from '../../../packages/contracts/src/g3-social.js';
import type {Request,Response,Publish,Scope} from '../../../packages/contracts/g3-social/g3-social.js';
import {SocialAuthority} from '../src/g3/social.js';
import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';
async function fixture(mode='core',tls=false,requiredCapabilities?:string[]){
 let now=Date.now();const f=await startG3Fixture({social:true,tls,now:()=>now});
 try{
 const state=await provisionG3Fixture(f,{mode,...(requiredCapabilities?{requiredCapabilities}:{})});
 const request=(i:number,payload:Partial<Request>,scope:Scope=state.scopes[i]!,credentials=state.games[i]!)=>({major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:credentials.sessionEpoch!,requestId:randomUUID(),scope,...payload});
 const call=async(i:number,payload:Partial<Request>,options:{scope?:Scope;desktop?:boolean;binary?:boolean;step?:number}={})=>{
  now+=options.step??100;const credentials=options.desktop?state.desktops[i]!:state.games[i]!,input=request(i,payload,options.scope??state.scopes[i]!,credentials);
  if(options.binary){const reply=await f.service.app.inject({method:'POST',url:'/v3/social/rpc',headers:{authorization:'Bearer '+credentials.accessToken,'content-type':'application/x-protobuf'},payload:Buffer.from(encode('Request',input))});assert.equal(reply.statusCode,200,reply.body);return decode('Response',reply.rawPayload) as Response;}
  const reply=await f.call('/v3/social/rpc',input,credentials.accessToken);assert.equal(reply.status,200,reply.body);return JSON.parse(reply.body) as Response;
 };
 const publish=async(i:number,p:Publish,options:Parameters<typeof call>[2]={})=>{
  const kind=(['hud','emote','cosmetic'] as const).find(k=>p[k]!==undefined);
  // Materialize a fixture's original identity once. Retries/alterations retain it.
  if(kind&&p.eventId![14]==='4')p.eventId=transientEventId(state.scopes[i]!,state.games[i]!.sessionId!,kind,p.sequence!);
  return call(i,{publish:p},options);
 };
 const ok=async(p:Promise<Response>)=>{const reply=await p;assert.equal(reply.error,undefined,reply.error?.code);return reply;};
 return {f,state,call,publish,ok,advance:(ms:number)=>{now+=ms;},clock:()=>now,request};
 }catch(e){await f.close();throw e;}
}
const hud=(sequence=1,hpMilli=100000):Publish=>({eventId:randomUUID(),sequence,hud:{hpMilli,maxHpMilli:100000,sampo:false,hpKnown:true,sampoKnown:true}});
const poi=(sequence=1,x=0,text='Hello 雪'):Publish=>({eventId:randomUUID(),sequence,poi:{text,xMilli:String(x),yMilli:'-10000'}});

test('opt-in 103-to-104 replay migration retires transient answers without resetting durable fences or POIs',async()=>{
 const t=await fixture();try{
  await t.state.start();await t.state.playing();const original=hud();await t.ok(t.publish(0,original));await t.ok(t.publish(0,poi()));
  // Reconstruct the old answer table in this fixture's owned namespace only.
  await t.f.service.db.transaction(async tx=>{
   await tx.query('INSERT INTO g3_social_ids VALUES($1,$2,$3,$4,$5)',[t.state.scopes[0]!.runId,t.state.games[0]!.sessionId,original.eventId,'a'.repeat(64),new Date(t.clock())]);
   await tx.query('DROP TABLE g3_social_replay');await tx.query('DELETE FROM schema_migrations WHERE version=104');
  });
  await t.f.service.db.migrate();
  assert.equal((await t.f.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS count FROM g3_social_ids'))).rows[0].count,1);
  assert.equal((await t.publish(0,original)).error?.code,'CONFLICT');await t.ok(t.publish(0,hud(2)));
  assert.equal((await t.ok(t.call(1,{poiSnapshot:{}}))).markers!.length,1);
 }finally{await t.f.close();}
});

test('opt-in social lifecycle derives frozen attribution; duplicates neither consume budget nor change retained state',async()=>{
 const t=await fixture();try{
  assert.ok((await t.publish(0,hud())).error);await t.state.start();assert.ok((await t.publish(0,hud())).error);await t.state.playing();
  const proximityRace=await Promise.all([t.publish(0,poi(1,0)),t.publish(1,poi(1,49999))]);assert.equal(proximityRace.filter(r=>!r.error).length,1);assert.equal(proximityRace.filter(r=>r.error?.code==='TOO_CLOSE').length,1);
  const original=hud();await t.ok(t.publish(0,original,{binary:true}));
  const before=await t.f.service.db.transaction(tx=>tx.query("SELECT * FROM g3_social_fences WHERE kind='hud'"));
  assert.equal((await t.ok(t.publish(0,original))).duplicate,true);
  const after=await t.f.service.db.transaction(tx=>tx.query("SELECT * FROM g3_social_fences WHERE kind='hud'"));assert.deepEqual(after.rows,before.rows);
  assert.equal((await t.publish(0,{...original,hud:{...original.hud,hpMilli:1}})).error?.code,'CONFLICT');
  const snapshot=await t.ok(t.call(1,{snapshot:{limit:1}},{binary:true}));assert.equal(snapshot.projections![0]!.principalId,t.state.desktops[0]!.accountId);assert.deepEqual(snapshot.projections![0]!.scope,t.state.scopes[0]);
  assert.equal(snapshot.projections![0]!.provenance,'validated-client-report');
  assert.equal((await t.call(1,{snapshot:{}},{scope:{...t.state.scopes[1],worldToken:'foreign'}})).error?.code,'STALE_EPOCH');
  assert.equal((await t.call(1,{snapshot:{}},{scope:{...t.state.scopes[1],attachmentId:t.state.scopes[0]!.attachmentId!}})).error?.code,'STALE_EPOCH');
  assert.equal((await t.call(1,{snapshot:{}},{desktop:true})).error?.code,'FORBIDDEN');
  t.advance(2500);assert.equal((await t.ok(t.call(1,{snapshot:{}}))).projections!.length,1);
  assert.equal((await t.ok(t.publish(0,original))).duplicate,true);assert.equal((await t.ok(t.call(1,{snapshot:{}}))).projections!.length,1);
  assert.equal((await t.ok(t.publish(0,hud(1)))).duplicate,true); // Same authority/kind/sequence derives the original ID.
  assert.equal((await t.publish(0,hud(1,1))).error?.code,'CONFLICT');
  await t.state.rpc(t.state.games[0]!,{gameObservation:{...t.state.observations[0]!,observationRevision:3,protected:true}});
  assert.ok((await t.publish(0,hud(2))).error);assert.equal((await t.ok(t.call(1,{snapshot:{}}))).projections!.length,1);
  await t.state.rpc(t.state.games[0]!,{gameObservation:{...t.state.observations[0]!,observationRevision:4,protected:false}});
  await t.ok(t.publish(0,hud(2)));await t.state.rpc(t.state.host,{endRun:{}});assert.ok((await t.call(1,{snapshot:{}})).error);
 }finally{await t.f.close();}
});

test('durable POIs paginate, retry once and keep recipient dismissal local; cooldown/proximity races serialize',async()=>{
 const t=await fixture('core',true);try{
  await t.state.start();await t.state.playing();const p=poi();await t.ok(t.publish(0,p));assert.equal((await t.ok(t.publish(0,p))).duplicate,true);
  assert.equal((await t.publish(0,poi(2,100000))).error?.code,'TOO_SOON');
  assert.equal((await t.publish(1,poi(1,49999))).error?.code,'TOO_CLOSE');await t.ok(t.publish(1,poi(1,200000)));
  const first=await t.ok(t.call(1,{poiSnapshot:{limit:1}})),id=first.markers![0]!.poiSequence!;
  assert.equal(first.markers![0]!.poi!.text,'Hello 雪');assert.ok(BigInt(first.highSequence!)>=BigInt(id));
  await t.ok(t.call(1,{dismissPoi:{poiSequence:id}}));assert.equal((await t.ok(t.call(1,{dismissPoi:{poiSequence:id}}))).duplicate,true);
  const own=await t.ok(t.call(0,{poiSnapshot:{}}));assert.ok(own.markers!.some(m=>m.poiSequence===id));
  for(const kind of ['poiSnapshot','poiOutbox'] as const)assert.ok(!(await t.ok(t.call(1,{[kind]:{afterSequence:'0',limit:32}}))).markers!.some(m=>m.poiSequence===id));
  const tail=(await t.ok(t.call(1,{poiSnapshot:{}}))).markers!.at(-1)!.poiSequence!;
  await t.ok(t.call(1,{dismissPoi:{poiSequence:tail}}));
  for(const kind of ['poiSnapshot','poiOutbox'] as const){const done=await t.ok(t.call(1,{[kind]:{afterSequence:'0',limit:1}}));assert.equal(done.markers!.length,0);assert.equal(done.nextSequence,done.highSequence);}
  assert.equal((await t.ok(t.publish(0,p))).duplicate,true);assert.ok(!(await t.ok(t.call(1,{poiSnapshot:{}}))).markers!.some(m=>m.poiSequence===id));
  await t.ok(t.call(0,{dismissPoi:{poiSequence:id}}));t.advance(14000);await t.state.pulse();t.advance(17000);await t.state.pulse();
  await t.ok(t.publish(0,poi(2,0))); // The sender's dismissed local marker no longer blocks spacing.
  const second=await t.ok(t.call(0,{poiSnapshot:{afterSequence:first.nextSequence!,limit:1}}));assert.equal(second.markers!.length,1);assert.ok(BigInt(second.nextSequence!)>BigInt(first.nextSequence!));
  const invalid=await t.f.call('/v3/social/rpc',{...t.request(0,{publish:poi(3)}),principalId:'forged'},t.state.games[0]!.accessToken);assert.equal(invalid.status,400);
 }finally{await t.f.close();}
});

test('social emote timings/robes, death cleanup, per-kind budgets, full authority expiry and restart',async()=>{
 const t=await fixture();try{
  await t.state.start();await t.state.playing();
  await t.ok(t.publish(0,{eventId:randomUUID(),sequence:1,emote:{emoteId:'heart',active:true}}));
  await t.ok(t.publish(0,{eventId:randomUUID(),sequence:1,cosmetic:{robeId:'purple',hat:false,amulet:true,gem:false}}));
  const page=await t.ok(t.call(1,{snapshot:{}}));assert.equal(page.projections!.find(p=>p.emote)?.emote!.durationFrames,101);
  await t.ok(t.publish(0,hud(1,0),{binary:true}));assert.equal((await t.publish(0,{eventId:randomUUID(),sequence:2,emote:{emoteId:'wave',active:true}})).error?.code,'FORBIDDEN');
  assert.deepEqual((await t.ok(t.call(1,{snapshot:{}}))).projections!.map(p=>p.hud?.hpMilli??0),[0]);
  await t.ok(t.publish(0,hud(2)));await t.ok(t.publish(0,{eventId:randomUUID(),sequence:2,emote:{emoteId:'wave',active:true}}));
  const eventId=randomUUID(),game=t.state.games[0]!,scope=t.state.scopes[0]!;
  const deathRequest={major:1,revision:'g3-draft-1',serviceId:t.f.service.db.serviceId,serviceEpoch:t.f.service.db.epoch,sessionEpoch:game.sessionEpoch,requestId:randomUUID(),scope,gameEvent:{eventId,catalogId:'player.death',count:1}};
  const death=await t.f.call('/v3/rpc',deathRequest,game.accessToken);assert.equal(JSON.parse(death.body).error,undefined);const deadState=(await t.ok(t.call(1,{snapshot:{}}))).projections!;assert.equal(deadState.length,1);assert.equal(deadState[0]!.hud!.hpMilli,0);
  t.advance(1000);await t.ok(t.publish(0,hud(3)));await t.f.call('/v3/rpc',{...deathRequest,requestId:randomUUID()},game.accessToken);const restored=(await t.ok(t.call(1,{snapshot:{}}))).projections!;assert.equal(restored.length,3);assert.equal(restored.find(p=>p.emote)!.emote!.active,false);assert.equal(restored.find(p=>p.cosmetic)!.cosmetic!.robeId,'purple');
  t.advance(1000);for(let seq=4;seq<6;seq++)await t.ok(t.publish(0,hud(seq),{step:0}));assert.equal((await t.publish(0,hud(6),{step:0})).error?.code,'CAPACITY');
  t.advance(31000);assert.ok((await t.publish(0,hud(9))).error);assert.ok((await t.call(1,{snapshot:{}})).error);
  await t.f.restart();assert.ok((await t.call(1,{poiSnapshot:{}})).error);
 }finally{await t.f.close();}
});

test('social-only simulated Nemesis and bounded marker/id/read storage; default fixture has no social grants',async()=>{
 const ordinary=await startG3Fixture();try{assert.equal((await ordinary.call('/v3/social/info')).status,404);assert.ok(!ordinary.profile.compatibility.capabilities.includes('core.poi'));}finally{await ordinary.close();}
 const t=await fixture('nemesis');try{
  await t.state.start();await t.state.playing();await t.ok(t.publish(0,hud()));await t.ok(t.publish(0,poi()));
  await t.f.service.db.transaction(async tx=>{
   for(let i=1;i<255;i++){const eventId=randomUUID();await tx.query('INSERT INTO g3_social_ids VALUES($1,$2,$3,$4,now())',[t.state.scopes[0]!.runId,t.state.games[0]!.sessionId,eventId,'a'.repeat(64)]);await tx.query('INSERT INTO g3_social_pois(run_id,game_session_id,event_id,body,recipient_sessions,created_at) VALUES($1,$2,$3,$4,$5,now())',[t.state.scopes[0]!.runId,t.state.games[0]!.sessionId,eventId,{poi:{text:'capacity fixture',xMilli:String(i*100000),yMilli:'0'}},[t.state.games[0]!.sessionId]]);}
  });
  t.advance(15000);await t.state.pulse();t.advance(17000);await t.state.pulse();
  const race=await Promise.all([t.publish(0,poi(2,300000000)),t.publish(1,poi(1,100000000))]);assert.equal(race.filter(r=>!r.error).length,1);assert.equal(race.filter(r=>r.error?.code==='CAPACITY').length,1);
  assert.equal((await t.publish(1,poi(2,400000000))).error?.code,'CAPACITY');
  const markers=await t.ok(t.call(0,{poiSnapshot:{limit:32}}));assert.equal(markers.markers!.length,16);
  t.advance(1000);for(let i=0;i<10;i++)await t.ok(t.call(1,{snapshot:{}},{step:0}));assert.equal((await t.call(1,{snapshot:{}},{step:0})).error?.code,'CAPACITY');
  await t.ok(t.publish(0,hud(2))); // Durable marker capacity cannot starve HUD.
  await t.state.rpc(t.state.host,{endRun:{}});t.advance(31*86400000);
  const authority=new SocialAuthority(()=>t.f.service.db.epoch,t.f.profile.compatibility.capabilities,t.clock);
  await t.f.service.db.transaction(tx=>authority.maintain(tx));
  assert.equal(+(await t.f.service.db.transaction(tx=>tx.query('SELECT count(*) FROM g3_social_ids'))).rows[0].count,0);
  assert.equal(+(await t.f.service.db.transaction(tx=>tx.query('SELECT count(*) FROM g3_social_pois'))).rows[0].count,0);
 }finally{await t.f.close();}
});

test('continuous social traffic exceeds 4096 with bounded independent retry windows and durable sequence fences',async()=>{
 const t=await fixture();try{
  await t.state.start();await t.state.playing();
  const marker=poi();await t.ok(t.publish(0,marker,{step:0}));
  const markerId=(await t.ok(t.call(1,{poiSnapshot:{}},{step:0}))).markers![0]!.poiSequence!;
  await t.ok(t.call(1,{dismissPoi:{poiSequence:markerId}},{step:0}));
  const authority=new SocialAuthority(()=>t.f.service.db.epoch,t.f.profile.compatibility.capabilities,t.clock);
  const retired=hud(1),recent:Publish[][]=[[],[]];
  for(let sequence=1;sequence<=720;sequence++){
   t.advance(1000);
   if(sequence%10===0)await t.state.pulse();
   for(let sender=0;sender<2;sender++){
    const publications=[sender===0&&sequence===1?retired:hud(sequence),
     {eventId:randomUUID(),sequence,emote:{emoteId:'wave',active:true}},
     {eventId:randomUUID(),sequence,cosmetic:{robeId:'purple',hat:false,amulet:false,gem:false}}] as Publish[];
    for(const p of publications)await t.ok(t.publish(sender,p,{step:0,binary:true}));
    recent[sender]=publications;
   }
   if(sequence%60===0){
    t.advance(3500);await t.state.pulse();
    await t.f.service.db.transaction(tx=>authority.maintain(tx));
    const counts=(await t.f.service.db.transaction(tx=>tx.query('SELECT game_session_id,kind,count(*)::int AS count FROM g3_social_replay GROUP BY game_session_id,kind'))).rows;
    assert.equal(counts.length,6);assert.ok(counts.every(r=>r.count<=64));
   }
  }
  assert.ok(t.clock()<Date.parse(t.state.games[0]!.expiresAt!),'all access tokens remain valid throughout simulated traffic');
  const inspect=()=>t.f.service.db.transaction(async tx=>({
   fences:(await tx.query('SELECT * FROM g3_social_fences ORDER BY game_session_id,kind')).rows,
   latest:(await tx.query('SELECT * FROM g3_social_presentation ORDER BY world_token,kind')).rows,
   replay:(await tx.query('SELECT * FROM g3_social_replay ORDER BY game_session_id,kind,sequence')).rows
  }));
  const before=await inspect();assert.equal(before.replay.length,384);
  for(const [sender,publications] of recent.entries())for(const p of publications){
   assert.equal((await t.ok(t.publish(sender,p,{step:0}))).duplicate,true);
   assert.equal((await t.publish(sender,{...p,sequence:721},{step:0})).error?.code,'CONFLICT');
  }
  assert.deepEqual(await inspect(),before,'lost-response retries cannot renew expiry, consume budgets or recreate expired HUD');
  assert.equal((await t.publish(0,retired,{step:0})).error?.code,'CONFLICT');
  assert.equal((await t.publish(0,{...retired,hud:{...retired.hud,hpMilli:1}},{step:0})).error?.code,'CONFLICT');
  assert.equal((await t.publish(0,{...retired,sequence:721},{step:0})).error?.code,'CONFLICT','a retired UUID cannot be retagged with a fresh sequence');
  assert.equal((await t.publish(0,{eventId:retired.eventId!,sequence:721,emote:{emoteId:'wave',active:true}},{step:0})).error?.code,'CONFLICT');
  assert.equal((await t.publish(0,{...poi(2,200000),eventId:retired.eventId!},{step:0})).error?.code,'CONFLICT');
  assert.equal((await t.publish(0,hud(1),{step:0})).error?.code,'CONFLICT');
  assert.deepEqual(await inspect(),before,'retired originals and altered stale sequences have no effect');
  assert.equal((await t.ok(t.publish(0,marker,{step:0}))).duplicate,true);
  const hidden=await t.ok(t.call(1,{poiOutbox:{}},{step:0}));assert.equal(hidden.markers!.length,0);assert.equal(hidden.nextSequence,hidden.highSequence);
  assert.equal((await t.ok(t.call(0,{poiSnapshot:{}},{step:0}))).markers!.length,1);
  t.advance(1000);for(let sender=0;sender<2;sender++)await t.ok(t.publish(sender,hud(721),{step:0}));
  assert.equal((await t.f.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS count FROM g3_social_ids'))).rows[0].count,1);
  assert.equal((await t.f.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS count FROM g3_social_dismissals'))).rows[0].count,1);
 }finally{await t.f.close();}
});

test('negotiated capabilities and every frozen field fence social publishing; current looping/cosmetic state outlasts old TTLs',async()=>{
 const limited=await fixture('core',false,['core.seed','core.hud']);try{
  await limited.state.start();await limited.state.playing();await limited.ok(limited.publish(0,hud()));
  assert.equal((await limited.publish(0,{eventId:randomUUID(),sequence:1,emote:{emoteId:'wave',active:true}})).error?.code,'FORBIDDEN');
  const original=limited.state.scopes[0]!;
  for(const scope of [{...original,roomId:randomUUID()},{...original,runId:randomUUID()},{...original,runEpoch:randomUUID()},{...original,configRevision:original.configRevision!+1},{...original,attachmentId:randomUUID()},{...original,worldToken:'wrong-world'}])assert.ok((await limited.publish(0,hud(2),{scope})).error);
  for(const changes of [{serviceEpoch:randomUUID()},{sessionEpoch:randomUUID()},{serviceId:'foreign-service'}]){
   const reply=await limited.f.call('/v3/social/rpc',{...limited.request(0,{publish:hud(2)}),...changes},limited.state.games[0]!.accessToken);assert.equal(JSON.parse(reply.body).error.code,'STALE_EPOCH');
  }
  await limited.state.rpc(limited.state.host,{roomModerate:{accountId:limited.state.desktops[1]!.accountId!,action:'kick'}});assert.ok((await limited.publish(1,hud())).error);
 }finally{await limited.f.close();}
 const t=await fixture();try{
  await t.state.start();await t.state.playing();await t.ok(t.publish(0,{eventId:randomUUID(),sequence:1,emote:{emoteId:'wave',active:true}}));await t.ok(t.publish(0,{eventId:randomUUID(),sequence:1,cosmetic:{robeId:'red',hat:true,amulet:false,gem:false}}));
  t.advance(10500);await t.state.pulse();const snapshot=await t.ok(t.call(1,{snapshot:{}}));assert.equal(snapshot.projections!.length,2);assert.equal(snapshot.projections!.find(p=>p.cosmetic)!.cosmetic!.robeId,'red');assert.equal(snapshot.projections!.find(p=>p.emote)!.emote!.active,true);
  t.advance(20000);await t.state.pulse();assert.equal((await t.ok(t.call(1,{snapshot:{}}))).projections!.length,2);
  await t.ok(t.publish(0,{eventId:randomUUID(),sequence:2,emote:{emoteId:'wave',active:false}},{binary:true}));const stopped=(await t.ok(t.call(1,{snapshot:{}},{binary:true}))).projections!.find(p=>p.emote)!.emote!;assert.equal(stopped.active??false,false);assert.equal(stopped.durationFrames??0,0);
 }finally{await t.f.close();}
});
