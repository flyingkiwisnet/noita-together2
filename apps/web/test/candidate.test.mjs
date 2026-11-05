import test from 'node:test';
import assert from 'node:assert/strict';
import {createHostController,requiredRoomCapabilities} from '../candidate/app.js';
import {controllerDom} from './dom-fixture.mjs';
test('declared canonical semantics work across artifact IDs and reject unsupported shapes',()=>{
 const caps=['core.seed','core.ghost','core.hud','core.poi'];
 const roomSemantics={revision:'g3-room-semantics-1',kind:'core-social',requiredCapabilities:caps};
 const info={profile:{id:'artifact-one'},compatibility:{capabilities:caps},roomSemantics};
 for(const id of ['artifact-one','different-valid-artifact','nt2-g3-native-social-development-999']){
  const v={...info,profile:{id}};assert.deepEqual(requiredRoomCapabilities(v),caps);assert.deepEqual(requiredRoomCapabilities(v,['core.seed']),caps);
 }
 for(const capabilities of [undefined,null,'core.ghost',caps.slice(0,2),[...caps,'core.gold'],[...caps.slice(0,3),'core.hud']])assert.throws(()=>requiredRoomCapabilities({...info,compatibility:{capabilities}}),/INCOMPATIBLE_GAME/);
 for(const declaration of [null,{...roomSemantics,kind:'unknown'},{...roomSemantics,revision:'future'},{...roomSemantics,requiredCapabilities:['core.seed']},{...roomSemantics,extra:true}])assert.throws(()=>requiredRoomCapabilities({...info,roomSemantics:declaration}),/INCOMPATIBLE_GAME/);
 const legacy={profile:{id:'legacy'},compatibility:{capabilities:caps}};assert.deepEqual(requiredRoomCapabilities(legacy),['core.seed']);assert.deepEqual(requiredRoomCapabilities(legacy,['core.seed','core.ghost']),['core.seed','core.ghost']);
});
test('current social website create and explicit Configure both request the reviewed capabilities',async()=>{
 const caps=['core.seed','core.ghost','core.hud','core.poi'];
 for(const revision of [3,4,5]){
  const dom=await controllerDom('../candidate/index.html'),calls=[];
  const room={roomId:'room',name:'Room',hostAccountId:'host',state:'waiting',revision:1,configRevision:1,config:{seed:424242,mode:'core',settings:[],requiredCapabilities:['core.seed']},members:[]};
  const client={credentials:{accountId:'host',sessionId:'browser',sessionEpoch:'epoch'},generation:1,schema:{revision:'g2-draft-5'},info:{serviceId:'service',serviceEpoch:'epoch',profile:{id:'nt2-g3-native-social-development-'+revision},compatibility:{capabilities:caps,modes:['core'],settingKeys:[]},roomSemantics:{revision:'g3-room-semantics-1',kind:'core-social',requiredCapabilities:caps}},async initialize(){this.info.mockProvider=true;return this.info;},dispose(){},async rpc(payload){calls.push(payload);if(payload.roomList)return {rooms:[]};return {room:structuredClone(room)};}};
  const controller=createHostController({...dom,client,schedule:()=>{}});
  try{await controller.ready;await dom.submit('create',{name:'Room',seed:424242,mode:'core',visibility:'public'});await dom.submit('configure',{seed:424243,mode:'core'});
   assert.deepEqual(calls.find(c=>c.roomCreate).roomCreate.config.requiredCapabilities,caps);
   assert.deepEqual(calls.find(c=>c.configure).configure.requiredCapabilities,caps);
  }finally{controller.dispose();}
 }
});
test('candidate website automatic preparation and default included roster require explicit inspected Start',async()=>{
 const dom=await controllerDom('../candidate/index.html'),scheduled=[],calls=[];
 const room={roomId:'room',name:'Room',hostAccountId:'host',state:'staging',runId:'run',runEpoch:'run-epoch',revision:1,configRevision:1,config:{seed:424242,mode:'core',settings:[]},members:[{desktopId:'desktop',accountId:'guest',principal:{displayName:'Alex',kind:'guest'},present:true,ready:true,readiness:'ready',included:true}]};
 const report={desktopId:'desktop',attachmentId:'attached',gameInstanceId:'game',worldToken:'world',observationRevision:1,contentDigest:'a'.repeat(64),actualSeed:'424242',seedStatus:'known',collectorFailure:'',modVersion:'test',bridgeVersion:'test',compatibilityProvenance:'game-thread-client-observation',activeMods:['noita_together2','different-mod'],inventoryComplete:true,protected:true,loaded:true,capturedAt:'2026-10-06T00:00:00Z',beta:'unknown'};
 const client={credentials:null,generation:1,schema:{revision:'g2-draft-5'},info:{serviceId:'service',serviceEpoch:'epoch',profile:{id:'candidate'},compatibility:{modes:['core'],settingKeys:[]}},async initialize(){this.info.mockProvider=true;return this.info;},async login(subject){calls.push({subject});this.credentials={accountId:'host',sessionId:'browser',sessionEpoch:'epoch',deviceId:'tab'};},dispose(){this.credentials=null;},async rpc(payload){calls.push(payload);if(payload.roomList)return {rooms:[]};if(payload.chatList)return {chatMessages:[]};if(payload.inspectionList)return {inspections:[report]};if(payload.prepare){room.state='staging';room.runId='run';room.runEpoch='run-epoch';}if(payload.finalStart)room.state='running';return {room:structuredClone(room)};}};
 const controller=createHostController({...dom,client,schedule:fn=>scheduled.push(fn)});await controller.ready;await dom.click('mock-twitch');assert.equal(calls[0].subject,'twitch-local-alex');await dom.submit('create',{name:'Room',seed:424242,mode:'core',visibility:'public'});const create=calls.find(c=>c.roomCreate);assert.equal(create.roomCreate.allowGuests,false);
 await scheduled.shift()();assert.equal(room.state,'staging');const row=dom.get('players').children[0];assert.match(row.textContent,/Alex/);assert.match(row.textContent,/Waiting for host/);assert.match(row.textContent,/Not checked/);assert.equal(calls.some(c=>c.finalStart||c.prepare||c.ready),false);assert.equal(row.children[0].children[0].checked,true);
 const check=row.children[0].children[0];check.checked=true;await check.fire('change');await dom.click('start');const start=calls.find(c=>c.finalStart);assert.deepEqual(start.finalStart.participants,[{desktopId:'desktop',attachmentId:'attached',worldToken:'world',observationRevision:1,contentDigest:'a'.repeat(64)}]);assert.equal(room.state,'running');controller.dispose();
});
test('automatic host roster keeps late/offline/loading joins included, preserves explicit exclusions and blocks Start',async()=>{
 const dom=await controllerDom('../candidate/index.html'),scheduled=[],calls=[];
 const room={roomId:'room',name:'Room',hostAccountId:'host',state:'staging',runId:'run',runEpoch:'epoch',revision:1,configRevision:1,config:{seed:424242,mode:'core'},members:[{desktopId:'a',accountId:'a',present:true,ready:true,readiness:'ready',included:true},{desktopId:'b',accountId:'b',present:true,ready:false,readiness:'loading_world',included:true}]};
 const report={desktopId:'a',attachmentId:'attachment',worldToken:'world',observationRevision:1,contentDigest:'a'.repeat(64),actualSeed:'424242',seedStatus:'known',activeMods:['noita_together2'],inventoryComplete:true,capturedAt:'now'};
 const client={credentials:{accountId:'host',sessionId:'host',sessionEpoch:'epoch'},generation:1,schema:{revision:'g2-draft-5'},info:{serviceId:'service',serviceEpoch:'epoch',profile:{id:'flow'},compatibility:{modes:['core'],settingKeys:[]}},async initialize(){this.info.mockProvider=true;return this.info;},dispose(){},async rpc(payload){calls.push(payload);if(payload.roomList)return {rooms:[]};if(payload.chatList)return {chatMessages:[]};if(payload.inspectionList)return {inspections:[report]};return {room:structuredClone(room)};}};
 const controller=createHostController({...dom,client,schedule:fn=>scheduled.push(fn)});
 try{
  await controller.ready;await dom.submit('create',{name:'Room',seed:424242,mode:'core',visibility:'public'});await scheduled.shift()();
  const [a,b]=dom.get('players').children;assert.equal(a.children[0].children[0].checked,true);assert.equal(b.children[0].children[0].checked,true);assert.equal(dom.get('start').disabled,true);assert.match(b.textContent,/protected game world is loading/);
  const exclude=b.children[0].children[0];exclude.checked=false;await exclude.fire('change');assert.match(b.textContent,/Excluded/);assert.equal(dom.get('start').disabled,false);
  room.members.push({desktopId:'c',accountId:'c',present:false,ready:false,readiness:'offline',included:true});room.revision++;await scheduled.shift()();assert.equal(exclude.checked,false);assert.equal(dom.get('players').children[2].children[0].children[0].checked,true);assert.equal(dom.get('start').disabled,true);assert.equal(calls.some(c=>c.prepare||c.ready||c.finalStart),false);
 }finally{controller.dispose();}
});

test('U1-S-SEED website drafts default shared random/loot and expose explicit fixed/individual policies without widening old configs',async()=>{
 const {seedDraft,seedDescription}=await import('../candidate/app.js');const info={compatibility:{capabilities:['core.seed','core.seed-policy.v1','core.randomize-loot.v1']}};
 assert.deepEqual(seedDraft(info,{seed:0,mode:'core',randomizeLoot:'on'}),{seed:0,mode:'core',seedPolicy:'shared-random',randomizeLoot:true,requiredCapabilities:['core.seed','core.seed-policy.v1','core.randomize-loot.v1']});
 const individual=seedDraft(info,{seedPolicy:'individual-random',seed:999,mode:'core',randomizeLoot:'on'});assert.equal(individual.seed,0);assert.equal(individual.randomizeLoot,false);assert.deepEqual(individual.requiredCapabilities,['core.seed','core.seed-policy.v1']);
 assert.equal(seedDraft(info,{seedPolicy:'shared-fixed',seed:123,mode:'core'}).randomizeLoot,false);assert.match(seedDescription({seedPolicy:'individual-random',seed:0}),/Separate random worlds/);assert.match(seedDescription({seed:0}),/Fixed shared seed 0.*loot off/);
 assert.throws(()=>seedDraft({compatibility:{capabilities:['core.seed-policy.v1']}},{seed:0,mode:'core',randomizeLoot:'on'}),/INCOMPATIBLE_GAME/);
});
test('U1-S-SEED website supported controls preserve draft-only reroll and show individual actual seed as accepted',async()=>{
 const dom=await controllerDom('../candidate/index.html'),calls=[],scheduled=[];const caps=['core.seed','core.seed-policy.v1','core.randomize-loot.v1'];
 const room={roomId:'room',runId:'run',runEpoch:'epoch',name:'Room',hostAccountId:'host',state:'staging',revision:1,configRevision:1,config:{seed:321,seedPolicy:'shared-random',randomizeLoot:true,mode:'core',requiredCapabilities:caps},members:[{desktopId:'a',accountId:'a',ready:true,included:true,seedApplied:true}]};
 const client={credentials:{accountId:'host',sessionId:'browser'},generation:1,schema:{revision:'g2-draft-5'},info:{serviceId:'service',serviceEpoch:'epoch',profile:{id:'fixture'},compatibility:{capabilities:caps,modes:['core'],settingKeys:[]}},async initialize(){this.info.mockProvider=true;return this.info;},dispose(){},async rpc(p){calls.push(p);if(p.roomList)return {rooms:[]};if(p.chatList)return {chatMessages:[]};if(p.inspectionList)return {inspections:[{desktopId:'a',actualSeed:'987',seedStatus:'known',inventoryComplete:true,activeMods:[],capturedAt:'now'}]};return {room:structuredClone(room)};}};
 const controller=createHostController({...dom,client,schedule:f=>scheduled.push(f)});try{await controller.ready;dom.get('create').elements.randomizeLoot.checked=true;await dom.submit('create',{name:'Room',seed:0,seedPolicy:'shared-random',mode:'core',visibility:'public'});assert.equal(calls.find(c=>c.roomCreate).roomCreate.config.randomizeLoot,true);assert.match(dom.get('scope').textContent,/Shared random seed 321.*loot on/);
 const count=calls.length;await dom.click('configure-random');assert.equal(calls.length,count);assert.ok(Number(dom.get('configure').elements.seed.value)>0&&Number(dom.get('configure').elements.seed.value)<=0xffffffff);assert.equal(dom.get('configure').elements.seedPolicy.value,'shared-fixed');
 room.config={seed:0,seedPolicy:'individual-random',randomizeLoot:false,mode:'core',requiredCapabilities:['core.seed','core.seed-policy.v1']};await scheduled.shift()();assert.match(dom.get('players').textContent,/Individual random worlds use independent seeds/);assert.doesNotMatch(dom.get('players').textContent,/MISMATCH/);
 }finally{controller.dispose();}
});
