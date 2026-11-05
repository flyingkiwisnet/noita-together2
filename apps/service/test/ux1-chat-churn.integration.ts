import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {startAuth1Fixture} from '../scripts/auth1-fixture.js';
test('Room chat tolerates roster/config/presence churn while enforcing scope, moderation, limits and receipt identity',async()=>{
 const f=await startAuth1Fixture(),db=f.service.db,auth=f.service.authentication!;
 const account=()=>db.transaction(async tx=>{const id=await auth.accounts.provision(tx,'google',randomUUID(),'Chat fixture');return{id,desktop:await f.service.auth.family(tx,id,randomUUID(),'desktop'),browser:await f.service.auth.family(tx,id,randomUUID(),'browser'),public:await auth.accounts.current(tx,{accountId:id} as any)};});
 const rpc=async(c:any,op:any,room?:any,id=randomUUID(),extra={})=>JSON.parse((await f.service.app.inject({method:'POST',url:'/v1/rpc',headers:{host:new URL(f.origin).host,authorization:'Bearer '+c.accessToken},payload:{major:1,revision:'g2-draft-5',serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:c.sessionEpoch,requestId:id,...(room?{roomId:room.roomId,expectedRevision:room.revision,configRevision:room.configRevision,runId:room.runId,runEpoch:room.runEpoch}:{}),...extra,...op}})).body);
 const ok=(value:any)=>{assert.ok(!value.error,JSON.stringify(value.error));return value;};
 const create=async(a:any)=>ok(await rpc(a.desktop,{roomCreate:{name:a.public.entitlements.generatedLobbyName,capacity:8,visibility:'public',config:{seed:123,seedPolicy:'shared-fixed',mode:'core',requiredCapabilities:f.profile.compatibility.capabilities,settings:[]}}})).room;
 try{
  const host=await account(),member=await account(),peer=await account(),outsider=await account();let room=await create(host);const initial=structuredClone(room),other=await create(outsider);
  room=ok(await rpc(host.desktop,{configure:{...room.config,seed:456}},room)).room;
  room=ok(await rpc(member.desktop,{roomJoin:{}},room)).room;const memberSnapshot=structuredClone(room);
  room=ok(await rpc(peer.desktop,{roomJoin:{}},room)).room;
  room=ok(await rpc(peer.desktop,{presence:{present:false}},room)).room;
  room=ok(await rpc(host.desktop,{configure:{...room.config,seed:789}},room)).room;
  assert.ok(room.revision>initial.revision);assert.ok(room.configRevision>memberSnapshot.configRevision);
  // All sends deliberately keep old snapshots, including old run/config IDs.
  for(let i=0;i<4;i++)assert.equal(ok(await rpc(member.desktop,{chatSend:{text:'stale desktop '+i}},memberSnapshot)).chatMessages[0].text,'stale desktop '+i);
  assert.equal(ok(await rpc(member.browser,{chatSend:{text:'stale browser'}},initial)).chatMessages[0].text,'stale browser');
  assert.equal(ok(await rpc(host.desktop,{chatSend:{text:'stale host'}},initial)).chatMessages[0].text,'stale host');
  assert.equal((await rpc(host.desktop,{configure:{...initial.config,seed:999}},initial)).error.code,'STALE_REVISION');
  assert.equal((await rpc(host.desktop,{roomPolicy:{capacity:7}},initial)).error.code,'STALE_REVISION');
  // A committed message's reply is discarded, then the exact immutable request
  // is replayed after more churn. Only one row and sequence may be committed.
  const id=randomUUID(),body={chatSend:{text:'lost reply'}},receipt=ok(await rpc(member.desktop,body,memberSnapshot,id));
  const originalSequence=receipt.chatMessages[0].sequence;
  room=ok(await rpc(host.desktop,{roomPolicy:{capacity:7}},room)).room;
  const replay=ok(await rpc(member.desktop,body,memberSnapshot,id));assert.deepEqual(replay.chatMessages,receipt.chatMessages);
  assert.equal((await db.pool.query('SELECT count(*) FROM room_chat WHERE room_id=$1 AND text=$2',[room.roomId,'lost reply'])).rows[0].count,'1');
  assert.equal((await rpc(member.desktop,{chatSend:{text:'different body'}},memberSnapshot,id)).error.code,'CONFLICT');
  assert.equal((await rpc(member.desktop,body,other,id)).error.code,'FORBIDDEN');
  assert.equal((await rpc(member.desktop,body,other)).error.code,'FORBIDDEN');
  assert.equal((await rpc(outsider.browser,{chatSend:{text:'visible observer'}},initial)).error.code,'FORBIDDEN');
  assert.equal((await rpc(member.desktop,{chatSend:{text:'bad epoch'}},memberSnapshot,randomUUID(),{sessionEpoch:randomUUID()})).error.code,'STALE_EPOCH');
  f.advance(1100);room=ok(await rpc(host.desktop,{roomModerate:{accountId:member.id,action:'mute'}},room)).room;
  assert.equal((await rpc(member.desktop,{chatSend:{text:'muted stale'}},memberSnapshot)).error.code,'CHAT_MUTED');
  assert.equal((await rpc(member.browser,{chatSend:{text:'muted browser'}},initial)).error.code,'CHAT_MUTED');
  assert.equal(ok(await rpc(member.desktop,body,memberSnapshot,id)).chatMessages[0].sequence,originalSequence);
  f.advance(1100);room=ok(await rpc(host.desktop,{roomModerate:{accountId:member.id,action:'kick'}},room)).room;
  assert.equal((await rpc(member.desktop,{chatSend:{text:'kicked stale'}},memberSnapshot)).error.code,'FORBIDDEN');
  assert.equal((await rpc(member.browser,{chatSend:{text:'kicked browser'}},initial)).error.code,'FORBIDDEN');
  f.advance(1100);room=ok(await rpc(host.desktop,{roomModerate:{accountId:peer.id,action:'ban'}},room)).room;
  assert.equal((await rpc(peer.desktop,{chatSend:{text:'banned stale'}},memberSnapshot)).error.code,'NOT_FOUND');
  assert.equal((await rpc(host.desktop,{chatSend:{text:'   '}},initial)).error.code,'INVALID_MESSAGE');
  assert.ok((await rpc(host.desktop,{chatSend:{text:'你'.repeat(342)}},initial)).error);
  // Existing five-per-ten-seconds chat budget is still enforced.
  f.advance(11000);for(let i=0;i<5;i++)ok(await rpc(host.desktop,{chatSend:{text:'bounded '+i}},initial));assert.equal((await rpc(host.desktop,{chatSend:{text:'sixth'}},initial)).error.code,'CAPACITY');
  console.log(JSON.stringify({ux1ChatChurn:true,staleRosterAndConfig:true,presenceChurn:true,repeatedDesktopAndBrowserChat:true,hostCASPreserved:true,kickMuteBanAndCrossRoomFences:true,chatRatePreserved:true,immutableLostReplyDedup:true,actualNoita:false}));
 }finally{await f.close();}
});
