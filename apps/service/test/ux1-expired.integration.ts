import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {readFileSync} from 'node:fs';import {startAuth1Fixture} from '../scripts/auth1-fixture.js';
// Execute the shipped web client against the actual service/receipt store.
// @ts-ignore JavaScript browser consumer exercised by tsx.
import {HostClient} from '../../web/candidate/client.js';
test('Committed but lost create/settings/chat replies expire, require review, and permit only deliberate new operations',async()=>{
 const f=await startAuth1Fixture(),db=f.service.db,auth=f.service.authentication!;let lost=false,offset=0;const sent:any[]=[];
 try{
  const identity=await db.transaction(async tx=>{const accountId=await auth.accounts.provision(tx,'google',randomUUID(),'Expired receipt fixture');return {accountId,credentials:await f.service.auth.family(tx,accountId,randomUUID(),'browser'),account:await auth.accounts.current(tx,{accountId} as any)};});
  const client=new HostClient(async(_path:string,request:any,token:string)=>{sent.push(structuredClone(request));const reply=JSON.parse((await f.service.app.inject({method:'POST',url:'/v1/rpc',headers:{host:new URL(f.origin).host,...(token?{authorization:'Bearer '+token}:{})},payload:request})).body);if(lost)throw Error('committed response lost');return reply;},randomUUID,()=>Date.now()+offset);
  client.schema=JSON.parse(readFileSync('packages/contracts/g2-next/schema.json','utf8'));client.info={major:1,revision:'g2-draft-5',serviceId:db.serviceId,serviceEpoch:db.epoch};client.credentials=identity.credentials;client.device=identity.credentials.deviceId;
  const create={roomCreate:{name:identity.account.entitlements.generatedLobbyName,capacity:8,visibility:'public',config:{seed:123,mode:'core',seedPolicy:'shared-fixed',randomizeLoot:false,requiredCapabilities:f.profile.compatibility.capabilities,settings:[]}}};
  lost=true;await assert.rejects(client.mutateRoom(create));assert.equal((await db.pool.query('SELECT count(*) FROM rooms')).rows[0].count,'1');const original=sent[0];assert.deepEqual(sent[1],original);
  f.advance(31000);offset+=31000;lost=false;
  // Create uses the retained foundation receipt (no 30s expiry); it must recover
  // the original room even after 30s, without another create request identity.
  const recovered=await client.retryRoomMutation('roomCreate:new');assert.equal(recovered.room.roomId,(await db.pool.query('SELECT id FROM rooms')).rows[0].id);assert.deepEqual(sent.at(-1),original);
  const listing=await client.rpc({roomList:{limit:32}});assert.equal(listing.rooms.length,1);let pending:any;
  const room=(await client.rpc({roomGet:{}},recovered.room)).room;
  lost=true;await assert.rejects(client.mutateRoom({configure:{...room.config,seed:456}},room));f.advance(31000);offset+=31000;lost=false;await assert.rejects(client.retryRoomMutation('configure:'+room.roomId),(e:any)=>e.code==='EXPIRED');pending=client.pendingMutations.get('configure:'+room.roomId);
  const current=(await client.rpc({roomGet:{}},room)).room;assert.equal(current.config.seed,456);const count=sent.length;client.acknowledgeExpiredOutcome('configure:'+room.roomId,pending);assert.equal(sent.length,count);
  const updated=(await client.mutateRoom({configure:{...current.config,seed:789}},current)).room;assert.equal(updated.config.seed,789);assert.notEqual(sent.at(-1).requestId,pending.request.requestId);
  lost=true;await assert.rejects(client.sendRoomChat('committed old message',updated));assert.equal((await db.pool.query('SELECT count(*) FROM room_chat')).rows[0].count,'1');const chatRequest=client.pendingChat.request;f.advance(15*60*1000+1000);offset+=15*60*1000+1000;lost=false;await assert.rejects(client.sendRoomChat('committed old message',updated),(e:any)=>e.code==='EXPIRED');
  await assert.rejects(client.sendRoomChat('edited draft',updated),(e:any)=>e.code==='CHAT_SEND_UNCERTAIN');pending=client.pendingChat;const history=await client.rpc({chatList:{limit:32}},updated);assert.equal(history.chatMessages.length,0);const countBeforeAck=sent.length;client.acknowledgeExpiredOutcome('chat',pending);assert.equal(sent.length,countBeforeAck);
  await client.sendRoomChat('deliberate new message',updated);assert.notEqual(sent.at(-1).requestId,chatRequest.requestId);assert.equal((await client.rpc({chatList:{limit:32}},updated)).chatMessages.length,1);assert.equal((await db.pool.query('SELECT count(*) FROM rooms')).rows[0].count,'1');
  // A stale acknowledgment cannot release anything after authentication turnover.
  client.pendingChat=pending;client.dispose();client.credentials={...identity.credentials,sessionId:randomUUID()};assert.throws(()=>client.acknowledgeExpiredOutcome('chat',pending),(e:any)=>e.code==='STALE_EPOCH');assert.equal(client.pendingChat,null);assert.equal(client.pendingMutations.size,0);
  console.log(JSON.stringify({ux1ExpiredReceiptComposition:true,committedLostCreate:true,committedLostSettings:true,committedLostChat:true,explicitAcknowledgmentOnly:true,noAutomaticDuplicate:true,authTurnoverFenced:true,actualNoita:false}));
 }finally{await f.close();}
});

