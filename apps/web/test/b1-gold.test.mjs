import test from 'node:test';
import assert from 'node:assert/strict';
import {goldSettings,createHostController,requiredRoomCapabilities} from '../candidate/app.js';
import {controllerDom} from './dom-fixture.mjs';
const caps=['core.seed','core.ghost','core.hud','core.poi','core.seed-policy.v1','core.randomize-loot.v1','core.emotes','core.cosmetics','core.gold'];
const info={compatibility:{capabilities:caps,modes:['core'],settingKeys:['NT_send_gold','NT_send_items']},roomSemantics:{revision:'g3-room-semantics-1',kind:'core-gold-social-controls-seed-loot',requiredCapabilities:caps}};
test('B1 gold has explicit independent setting and exact room semantics',()=>{
 assert.deepEqual(requiredRoomCapabilities(info),caps);
 const future=[{key:'NT_send_items',value:'false'},{key:'NT_send_gold',value:'false'}];
 assert.deepEqual(goldSettings(info,{sendGold:'on'},future),[{key:'NT_send_items',value:'false'},{key:'NT_send_gold',value:'true'}]);
 assert.deepEqual(goldSettings(info,{},future),future);
 assert.deepEqual(goldSettings({compatibility:{capabilities:[],settingKeys:[]}},{}),[]);
 assert.throws(()=>requiredRoomCapabilities({...info,compatibility:{...info.compatibility,capabilities:[...caps,'core.items']}}),/INCOMPATIBLE_GAME/);
});
test('B1 create defaults Send Gold on; explicit Configure can turn it off without changing item category',async()=>{
 const dom=await controllerDom('../candidate/index.html'),calls=[];
 const room={roomId:'room',name:'Room',hostAccountId:'host',state:'waiting',revision:1,configRevision:1,config:{seed:424242,seedPolicy:'shared-fixed',mode:'core',settings:[{key:'NT_send_gold',value:'true'},{key:'NT_send_items',value:'false'}],requiredCapabilities:caps},members:[]};
 const client={credentials:{accountId:'host',sessionId:'browser',sessionEpoch:'epoch'},generation:1,schema:{revision:'g2-draft-5'},info,async initialize(){info.mockProvider=true;return info;},dispose(){},async rpc(payload){calls.push(payload);if(payload.roomList)return {rooms:[]};return {room:structuredClone(room)};}};
 const controller=createHostController({...dom,client,schedule:()=>{}});
 try{await controller.ready;assert.equal(dom.get('create').elements.sendGold.checked,true);assert.equal(dom.get('create-send-gold').hidden,false);
  await dom.submit('create',{name:'Room',seed:424242,mode:'core',visibility:'public',seedPolicy:'shared-fixed'});
  assert.deepEqual(calls.find(c=>c.roomCreate).roomCreate.config.settings,[{key:'NT_send_gold',value:'true'}]);
  dom.get('configure').elements.sendGold.checked=false;await dom.submit('configure',{seed:424242,mode:'core',seedPolicy:'shared-fixed'});
  assert.deepEqual(calls.find(c=>c.configure).configure.settings,[{key:'NT_send_items',value:'false'},{key:'NT_send_gold',value:'false'}]);
 }finally{controller.dispose();}
});
