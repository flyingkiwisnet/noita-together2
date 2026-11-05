import test from 'node:test';import assert from 'node:assert/strict';
import {createHostController} from '../candidate/app.js';import {controllerDom} from './dom-fixture.mjs';
test('host delete confirms once, retains selection on cancel/refusal, and refreshes after success',async()=>{
 const dom=await controllerDom('../candidate/index.html'),calls=[];let confirmed=false,confirmation='',refused=false;
 dom.window.confirm=message=>{confirmation=message;return confirmed;};
 const room={roomId:'room-a',name:'Old lobby',hostAccountId:'host',state:'waiting',revision:3,configRevision:1,config:{seed:1,mode:'core'},members:[]};
 const client={credentials:{accountId:'host',sessionId:'host',sessionEpoch:'epoch'},generation:1,schema:{revision: "g2-draft-5"},info:{serviceId:"service",serviceEpoch:"epoch",profile:{id:"test"},compatibility:{modes:['core'],settingKeys:[]}},async initialize(){this.info.mockProvider=true;return this.info;},dispose(){},async rpc(payload,context){calls.push({payload,context});if(payload.roomList)return {rooms:[]};if(payload.chatList)return {};if(payload.roomDelete){if(refused)throw {code:'INVALID_STATE'};return {deletedRoomId:room.roomId};}return {room:structuredClone(room)};}};
 const c=createHostController({...dom,client,schedule(){}});try{
  await c.ready;await dom.submit('create',{name:'Old lobby',seed:1,mode:'core',visibility:'public'});assert.equal(dom.get('delete-room').disabled,false);
  await dom.click('delete-room');assert.match(confirmation,/Old lobby.*statistics will no longer be available/);assert.equal(calls.some(c=>c.payload.roomDelete),false);assert.equal(dom.get('room-panel').hidden,false);
  confirmed=true;refused=true;await dom.click('delete-room');assert.equal(dom.get('room-panel').hidden,false);assert.match(dom.get('notice').textContent,/End the run or cancel/);
  refused=false;await dom.click('delete-room');assert.equal(dom.get('room-panel').hidden,true);assert.equal(calls.filter(c=>c.payload.roomDelete).at(-1).context.revision,3);
 }finally{c.dispose();}
});
