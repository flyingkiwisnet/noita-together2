import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {LifecycleSocialControls} from '../scripts/lifecycle-lc1-social-controls.js';
import {encode,schema} from '../../../packages/contracts/src/g3-social.js';
const plan=()=>({target:'consumer',worldId:randomUUID(),operation:'publishPoi',eventId:randomUUID(),phase:'reply',effect:'drop'});
test('private social plans reject broad/invalid faults and bound arms/status copies',()=>{
 const c=new LifecycleSocialControls();for(const bad of [null,[],{...plan(),target:'all'},{...plan(),phase:'before'},{...plan(),effect:'refuse'},{...plan(),text:'not logged'},{...plan(),worldId:'other'},{...plan(),delayMs:1},{...plan(),eventId:undefined}])assert.throws(()=>c.arm(bad),/FIXTURE_SOCIAL_ARGUMENT/);
 const read={target:'peer',worldId:randomUUID(),operation:'poiSnapshot',phase:'reply',effect:'delay',delayMs:10000};for(const bad of [{...read,eventId:randomUUID()},{...read,delayMs:10001},{...read,delayMs:0},{...read,delayMs:1.5},{...read,operation:'poiOutcome'}])assert.throws(()=>c.arm(bad));
 c.arm(read);assert.throws(()=>c.arm(plan()),/FIXTURE_SOCIAL_BUSY/);c.release();for(let i=1;i<32;i++){c.arm(plan());c.release();}assert.equal(c.status().armsUsed,32);assert.equal(c.status().events.length,64);assert.throws(()=>c.arm(plan()),/FIXTURE_SOCIAL_BUSY/);const s=c.status();s.events[0].kind='tampered';assert.notEqual(c.status().events[0].kind,'tampered');c.close();assert.equal(c.status().timers,0);assert.equal(c.status().active,null);
});

test('emote reply-drop requires an exact original or explicit stop-only selector',()=>{
 const c=new LifecycleSocialControls(),stop={target:'consumer',worldId:randomUUID(),operation:'publishEmote',stopOnly:true,phase:'reply',effect:'drop'};
 for(const bad of [{...stop,stopOnly:undefined},{...stop,stopOnly:false},{...stop,stopOnly:'true'},{...stop,eventId:'bad'},{...stop,phase:'before'},{...stop,effect:'delay',delayMs:1},{...stop,delayMs:0},{...plan(),stopOnly:true}])assert.throws(()=>c.arm(bad),/FIXTURE_SOCIAL_ARGUMENT/);
 c.arm(stop);c.release();c.arm({...stop,eventId:randomUUID()});c.release();c.arm({...stop,stopOnly:undefined,eventId:randomUUID()});c.close();
});

// Hook-level simulation only: verifies fault matching, not service commits or DB.
for(const binary of [false,true])test('private '+(binary?'protobuf':'JSON')+' Stop arm captures authenticated original, survives refusals and drops only its accepted reply',async()=>{
 const c=new LifecycleSocialControls(),hooks=new Map<string,any[]>(),worldId=randomUUID(),sessionId=randomUUID(),attachmentId=randomUUID(),runId=randomUUID(),runEpoch=randomUUID(),roomId=randomUUID();
 const actor={kind:'game',accountId:randomUUID(),deviceId:randomUUID(),sessionId,sessionEpoch:randomUUID()};let authenticated=0;
 const service={app:{addHook:(name:string,fn:any)=>hooks.set(name,[...(hooks.get(name)??[]),fn])},db:{serviceId:'fixture-service',epoch:randomUUID(),transaction:async(fn:any)=>fn({query:async(_sql:string,args:any[])=>({rowCount:Number(args[0]===worldId&&args[1]===actor.accountId&&args[2]===actor.deviceId&&args[3]===sessionId&&args[4]===attachmentId&&args[5]===runId&&args[6]===runEpoch&&args[7]===1)})})},auth:{authenticate:async(_tx:any,token:string)=>{authenticated++;if(token!=='private-token')throw Error('UNAUTHENTICATED');return actor;}}};
 c.install(service as any,(target,a,r)=>target==='consumer'&&a.accountId===actor.accountId&&r===roomId);
 const pre=hooks.get('preHandler')!.at(-1)!,send=hooks.get('onSend')!.at(-1)!,scope={roomId,runId,runEpoch,configRevision:1,attachmentId,worldToken:worldId};
 const stopPlan={target:'consumer',worldId,operation:'publishEmote',stopOnly:true,phase:'reply',effect:'drop'};c.arm(stopPlan);
 const request=(payload:any={},changes:any={},token='private-token')=>{const body=JSON.parse(JSON.stringify({major:1,revision:schema.revision,serviceId:service.db.serviceId,serviceEpoch:service.db.epoch,sessionEpoch:actor.sessionEpoch,requestId:randomUUID(),scope,publish:{eventId:randomUUID(),sequence:1,emote:{emoteId:'wave',active:false}},...payload,...changes}));return {url:'/v3/social/rpc',headers:{authorization:'Bearer '+token,'content-type':binary?'application/x-protobuf':'application/json'},original:body.publish?.eventId,body:binary?Buffer.from(encode('Request',body)):body};};
 const reply=()=>{const state={hijacked:0,destroyed:0};return {state,raw:{destroy:()=>state.destroyed++},hijack:()=>state.hijacked++};};
 const payload=(v:any)=>binary?Buffer.from(encode('Response',v)):JSON.stringify(v);
 const ignored=[request({publish:{eventId:randomUUID(),sequence:1,emote:{emoteId:'wave',active:true}}}),request({snapshot:{},publish:undefined}),request({}, {scope:{...scope,worldToken:randomUUID()}}),request({}, {scope:{...scope,attachmentId:randomUUID()}}),request({}, {sessionEpoch:randomUUID()}),request({}, {serviceEpoch:randomUUID()}),request({}, {serviceId:'other'}),request({}, {},'wrong-token')];
 for(const req of ignored){const r=reply(),wire=payload({acceptedEventId:randomUUID()});await pre(req,r);assert.equal(c.status().active?.matched,false);assert.equal(await send(req,r,wire),wire);assert.equal(r.state.destroyed,0);}
 assert.ok(authenticated>0);assert.equal(c.status().armsUsed,1);
 for(const result of [{error:{code:'CAPACITY'}},{acceptedEventId:randomUUID()},null]){
  const req=request(),r=reply(),wire=result?payload(result):binary?Buffer.from([255]):'not JSON';await pre(req,r);assert.equal(c.status().active?.matched,true);assert.equal(c.status().active?.matchedEventId,req.original);
  assert.equal(await send(req,r,wire),wire);assert.equal(c.status().active?.matched,false);assert.equal(c.status().active?.matchedEventId,undefined);assert.equal(r.state.destroyed,0);
 }
 const req=request(),r=reply();await pre(req,r);const accepted=payload({acceptedEventId:req.original,duplicate:false});assert.equal(await send(req,r,accepted),null);assert.deepEqual(r.state,{hijacked:1,destroyed:1});assert.equal(c.status().active,null);assert.equal(c.status().events.at(-1).kind,'committed-reply-drop');assert.equal(c.status().events.at(-1).eventId,req.original);
 assert.equal(await send(req,r,accepted),accepted);assert.equal(r.state.destroyed,1,'one-shot even if the hook is called again');
 const exact=randomUUID();c.arm({...stopPlan,eventId:exact});const wrong=request();await pre(wrong,reply());assert.equal(c.status().active?.matched,false);
 const exactStart=request({publish:{eventId:exact,sequence:2,emote:{emoteId:'heart',active:true}}});await pre(exactStart,reply());assert.equal(c.status().active?.matched,false);
 const exactStop=request({publish:{eventId:exact,sequence:3,emote:{emoteId:'heart'}}}),cancelled=reply();await pre(exactStop,cancelled);assert.equal(c.status().active?.matchedEventId,exact);c.release();const cancelledWire=payload({acceptedEventId:exact});assert.equal(await send(exactStop,cancelled,cancelledWire),cancelledWire);assert.equal(cancelled.state.destroyed,0);
 c.arm({...stopPlan,target:'peer'});await pre(request(),reply());assert.equal(c.status().active?.matched,false);c.release();
 // POI refusal still finishes its arm; existing explicit-original drop remains.
 for(const refused of [true,false]){const eventId=randomUUID();c.arm({...plan(),worldId,eventId});const poi=request({publish:{eventId,sequence:4,poi:{text:'fixture',xMilli:'0',yMilli:'0'}}}),pr=reply(),wire=payload(refused?{error:{code:'TOO_SOON'}}:{acceptedEventId:eventId,duplicate:true});await pre(poi,pr);assert.equal(c.status().active?.matched,true);const result=await send(poi,pr,wire);assert.equal(result,refused?wire:null);assert.equal(pr.state.destroyed,refused?0:1);assert.equal(c.status().active,null);}
 c.close();assert.equal(c.status().timers,0);assert.equal(JSON.stringify(c.status()).includes('private-token'),false);
});
