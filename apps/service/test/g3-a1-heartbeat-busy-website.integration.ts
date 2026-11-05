import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
const {requiredRoomCapabilities}=await import(new URL('../../web/candidate/app.js',import.meta.url).href);

test('heartbeat-busy website defaults allow first peer movement; missing capability and authority still refuse',async()=>{
 for(const seedOnly of [false,true]){
  let now=Date.now();
  const f=await startG3Fixture({tls:true,a1Runtime:true,a1HeartbeatBusy:true,nativeSocial:true,now:()=>now});
  try{
   const info=JSON.parse((await f.call('/v1/info')).body);
   assert.equal(info.profile.id,'nt2-g3-native-social-development-5');
   const caps=requiredRoomCapabilities(info);
   const s=await provisionG3Fixture(f,{playerCount:3,requiredCapabilities:seedOnly?['core.seed']:caps,beforeRpc:()=>{now+=100;}});
   const charged:string[]=[];const budget=f.service.traffic,originalCharge=budget.authorizedRoom.bind(budget);
   budget.authorizedRoom=(room,tx)=>{originalCharge(room,tx);charged.push(room);};
   const call=async(i:number,payload:object,scope=s.scopes[i]!)=>{
    now+=100;const c=s.games[i]!;
    const r=await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope,...payload},c.accessToken);
    assert.equal(r.status,200);return JSON.parse(r.body);
   };
   const movement=(sequence:number)=>({movement:{frames:[{sequence,xMilli:'1000',yMilli:'2000',vxMilli:'0',vyMilli:'0',facingLeft:false}]}});
   assert.equal((await call(0,movement(1))).error.code,'NOT_FOUND');assert.equal(charged.length,0);
   assert.equal((await call(0,movement(1),{...s.scopes[0]!,roomId:randomUUID()})).error.code,'NOT_FOUND');assert.equal(charged.length,0,'untrusted room scope cannot charge any room');
   await s.rpc(s.host,{finalStart:{participants:s.reports.filter(r=>r.desktopId!==s.desktops[2]!.sessionId).map(r=>({desktopId:r.desktopId!,attachmentId:r.attachmentId!,worldToken:r.worldToken!,observationRevision:r.observationRevision!,contentDigest:r.contentDigest!})),excludedDesktopIds:[s.desktops[2]!.sessionId!]}});
   assert.equal((await call(0,movement(1))).error.code,'STALE_EPOCH','protected selected actor is still held');
   for(let i=0;i<2;i++)await s.rpc(s.games[i]!,{gameObservation:{...s.observations[i]!,observationRevision:2,protected:false}});
   assert.equal((await call(2,movement(1))).error.code,'STALE_EPOCH','excluded actor has no playing context');
   if(seedOnly){
    assert.equal((await call(0,movement(1))).error.code,'FORBIDDEN');
    assert.equal((await call(1,{ghostList:{}})).error.code,'FORBIDDEN');
   }else{
    for(let i=0;i<2;i++)assert.equal((await call(i,movement(1))).error,undefined,'first website-default publication must succeed');
    assert.ok(charged.includes(s.scopes[0]!.roomId!),'canonical run access charges its room');
    assert.ok(charged.every(id=>id===s.scopes[0]!.roomId),'only the canonical room is charged');
    assert.deepEqual(s.room.config!.requiredCapabilities,['core.seed','core.ghost','core.hud','core.poi']);
    for(let i=0;i<2;i++)assert.equal((await call(i,{ghostList:{}})).ghosts.length,1);
    assert.equal((await call(0,movement(2),{...s.scopes[0]!,worldToken:'wrong-world'})).error.code,'STALE_EPOCH');
    await s.rpc(s.games[0]!,{gameObservation:{...s.observations[0]!,observationRevision:3,protected:true}});
    assert.equal((await call(0,movement(2))).error.code,'STALE_EPOCH');
    assert.equal((await call(1,{ghostList:{}})).ghosts.length,0);
    now+=31000;assert.equal((await call(1,movement(2))).error.code,'FORBIDDEN','expired selected authority suspends the run');
   }
  }finally{await f.close();}
 }
});
