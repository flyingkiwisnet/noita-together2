import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHostController,seedDraft} from '../candidate/app.js';
import {declareRoomSemantics,validateRoomSemantics} from '../../../packages/contracts/src/room-semantics.js';
import {controllerDom} from './dom-fixture.mjs';
const runtime=JSON.parse(await readFile(new URL('../../../packages/test-kit/fixtures/a3-web-profile.json',import.meta.url)));
test('A3 exact8 profile gets explicit room semantics without extra capability grants',()=>{
 const compatibility=runtime.compatibility,caps=compatibility.capabilities;
 const declaration=declareRoomSemantics(compatibility);
 assert.equal(declaration.kind,'core-social-controls-seed-loot');
 assert.deepEqual(validateRoomSemantics(declaration,compatibility),caps);
 for(const capabilities of [[...caps,'core.gold'],caps.slice(0,-1),[...caps,caps[0]],caps.map(c=>c==='core.emotes'?'core.unknown':c)])assert.throws(()=>validateRoomSemantics(declaration,{capabilities}),/INCOMPATIBLE_GAME/);
 const info={compatibility,roomSemantics:declaration};
 for(const randomizeLoot of [false,true])for(const seedPolicy of ['shared-fixed','shared-random','individual-random']){
  const required=seedDraft(info,{seed:123,mode:'core',seedPolicy,randomizeLoot}).requiredCapabilities;
  for(const c of ['core.seed','core.ghost','core.hud','core.poi','core.emotes','core.cosmetics'])assert.ok(required.includes(c),c);
  assert.equal(required.includes('core.randomize-loot.v1'),randomizeLoot&&seedPolicy!=='individual-random');
 }
});
const deferred=()=>{let resolve;const promise=new Promise(r=>resolve=r);return {promise,resolve};};
async function setup(){
 const dom=await controllerDom('../candidate/index.html'),scheduled=[],calls=[];
 const room={roomId:'old-room',runId:'old-run',runEpoch:'run-epoch',name:'Room',hostAccountId:'host',state:'running',revision:1,configRevision:1,config:{seed:123,mode:'core'},members:[]};
 const participant={association:{roomId:room.roomId,runId:room.runId,accountId:'old-account',participantId:'old-player',worldToken:'old-world',configRevision:1},state:'reconnecting',generation:1,reviewRequired:true,pendingObservationRevision:2,pendingContentDigest:'a'.repeat(64)};
 const client={credentials:{accountId:'host',sessionId:'browser',sessionEpoch:'epoch'},generation:1,schema:{revision:'g2-draft-5'},info:{serviceId:'service',serviceEpoch:'epoch',profile:{id:'test'},compatibility:{modes:['core'],settingKeys:[]}},async initialize(){this.info.mockProvider=true;return this.info;},dispose(){this.credentials=null;++this.generation;},async logout(){this.dispose();},async rpc(p){calls.push(p);if(p.roomList)return {rooms:[]};if(p.chatList)return {chatMessages:[]};if(p.inspectionList)return {inspections:[]};if(p.endRun){room.state='ended';room.revision++;}return {room:structuredClone(room)};},async rejoin(p){calls.push(p);return {participants:[structuredClone(participant)]};}};
 const controller=createHostController({...dom,client,schedule:f=>scheduled.push(f)});
 await controller.ready;await dom.submit('create',{name:'Room',seed:123,mode:'core',visibility:'public'});
 return {dom,room,participant,client,controller,calls,poll:()=>scheduled.shift()()};
}
test('recovery rows clear on new lobby, staging, End and signout; old review button is inert',async()=>{
 for(const action of ['switch','staging','end','logout']){
  const x=await setup();try{
   await x.poll();const list=x.dom.get('recovery-players');assert.equal(list.children.length,1);
   const oldButton=list.children[0].children[0];
   if(action==='switch'){x.room.roomId='new-room';x.room.runId='';x.room.state='waiting';await x.dom.submit('create',{name:'New',seed:123,mode:'core',visibility:'public'});}
   if(action==='staging'){x.room.runId='new-run';x.room.state='staging';await x.poll();}
   if(action==='end')await x.dom.click('end');
   if(action==='logout')await x.dom.click('logout');
   assert.equal(list.children.length,0,action);const before=x.calls.length;await oldButton.fire('click');assert.equal(x.calls.length,before,'stale review cannot submit');
  }finally{x.controller.dispose();}
 }
});
test('late recovery and room replies cannot restore rows after page close or identity change',async()=>{
 for(const pendingKind of ['rejoin','roomGet'])for(const invalidation of ['pagehide','session','epoch']){
  const x=await setup();try{
   await x.poll();assert.equal(x.dom.get('recovery-players').children.length,1);
   const entered=deferred(),held=deferred();
   if(pendingKind==='rejoin')x.client.rejoin=async()=>{entered.resolve();return held.promise;};
   else{const rpc=x.client.rpc.bind(x.client);x.client.rpc=async p=>{if(p.roomGet){entered.resolve();return held.promise;}return rpc(p);};}
   const pending=x.poll();await entered.promise;
   if(invalidation==='pagehide')await x.dom.window.fire('pagehide');
   if(invalidation==='session'){x.client.credentials={...x.client.credentials,sessionId:'replacement'};++x.client.generation;}
   if(invalidation==='epoch')x.client.info.serviceEpoch='replacement';
   held.resolve(pendingKind==='rejoin'?{participants:[x.participant]}:{room:structuredClone(x.room)});await pending;
   assert.equal(x.dom.get('recovery-players').children.length,0,pendingKind+'/'+invalidation);
  }finally{x.controller.dispose();}
 }
});
