import test from 'node:test';import assert from 'node:assert/strict';import {bankSettings,requiredRoomCapabilities,createHostController} from '../candidate/app.js';import {controllerDom} from './dom-fixture.mjs';
const caps=['core.seed','core.ghost','core.hud','core.poi','core.seed-policy.v1','core.randomize-loot.v1','core.emotes','core.cosmetics','core.gold','core.items'];
const info={compatibility:{capabilities:caps,modes:['core'],settingKeys:['NT_send_gold','NT_send_wands','NT_send_flasks','NT_send_items']},roomSemantics:{revision:'g3-room-semantics-1',kind:'core-items-gold-social-controls-seed-loot',requiredCapabilities:caps}};
test('Item categories and gold are explicit independent settings with exact combined capability semantics',()=>{
 assert.deepEqual(requiredRoomCapabilities(info),caps);
 assert.deepEqual(bankSettings(info,{sendGold:'on',sendFlasks:'on'},[{key:'future',value:'kept'}]),[{key:'future',value:'kept'},{key:'NT_send_gold',value:'true'},{key:'NT_send_wands',value:'false'},{key:'NT_send_flasks',value:'true'},{key:'NT_send_items',value:'false'}]);
});
test('Create defaults every category on and Configure preserves independent choices',async()=>{
 const dom=await controllerDom('../candidate/index.html'),calls=[],room={roomId:'room',name:'Room',hostAccountId:'host',state:'waiting',revision:1,configRevision:1,config:{seed:424242,seedPolicy:'shared-fixed',mode:'core',settings:[{key:'NT_send_gold',value:'true'},{key:'NT_send_wands',value:'false'},{key:'NT_send_flasks',value:'true'},{key:'NT_send_items',value:'false'}],requiredCapabilities:caps},members:[]};
 const client={credentials:{accountId:'host',sessionId:'browser',sessionEpoch:'epoch'},generation:1,schema:{revision:'g2-draft-5'},info,async initialize(){info.mockProvider=true;return info;},dispose(){},async rpc(payload){calls.push(payload);return payload.roomList?{rooms:[]}:{room:structuredClone(room)};}};
 const controller=createHostController({...dom,client,schedule:()=>{}});try{await controller.ready;for(const k of ['sendGold','sendWands','sendFlasks','sendItems'])assert.equal(dom.get('create').elements[k].checked,true);
  await dom.submit('create',{name:'Room',seed:424242,mode:'core',visibility:'public',seedPolicy:'shared-fixed'});assert.ok(calls.find(v=>v.roomCreate).roomCreate.config.settings.every(s=>s.value==='true'));
  await dom.submit('configure',{seed:424243,mode:'core',seedPolicy:'shared-fixed'});assert.deepEqual(calls.find(v=>v.configure).configure.settings,room.config.settings);
 }finally{controller.dispose();}
});
