import test from 'node:test';
import assert from 'node:assert/strict';
import {createHostController} from '../candidate/app.js';
import {ServiceError} from '../candidate/client.js';
import {controllerDom} from './dom-fixture.mjs';
const deferred=()=>{let resolve;const promise=new Promise(r=>resolve=r);return {promise,resolve};};
const refusal=code=>Object.assign(new ServiceError(code),{verifiedResponse:true,operation:'finalStart'});
async function setup(factory=createHostController){
 const dom=await controllerDom('../candidate/index.html'),scheduled=[],calls=[];
 const server={room:{roomId:'room',runId:'run',runEpoch:'run-epoch',name:'Room',hostAccountId:'host',state:'staging',revision:1,configRevision:1,config:{seed:424242,mode:'core',settings:[]},members:[{desktopId:'desktop',accountId:'guest',present:true,ready:true,readiness:'ready',included:true}]},reports:[{desktopId:'desktop',attachmentId:'attached',gameInstanceId:'game',worldToken:'world',observationRevision:1,contentDigest:'a'.repeat(64),actualSeed:'424242',seedStatus:'known',activeMods:['noita_together2'],inventoryComplete:true,protected:true,loaded:true,capturedAt:'2026-10-23T00:00:00Z'}],beforeRpc:null,submissions:0};
 const client={credentials:{accountId:'host',sessionId:'browser',sessionEpoch:'epoch'},generation:1,schema:{revision:'g2-draft-5'},info:{serviceId:'service',serviceEpoch:'epoch',profile:{id:'test'},compatibility:{modes:['core'],settingKeys:[]}},async initialize(){this.info.mockProvider=true;return this.info;},dispose(){this.credentials=null;++this.generation;},async rpc(p,target){
  calls.push({payload:structuredClone(p),room:structuredClone(target)});if(server.beforeRpc){const result=await server.beforeRpc(p,target);if(result)return result;}
  if(p.roomList)return {rooms:[]};if(p.chatList)return {chatMessages:[]};if(p.inspectionList)return {inspections:structuredClone(server.reports)};
  if(p.finalStart){server.submissions++;if(target.revision!==server.room.revision)throw refusal('STALE_REVISION');for(const old of p.finalStart.participants){if(server.reports.find(r=>r.desktopId===old.desktopId)?.observationRevision!==old.observationRevision)throw refusal('REPORT_CHANGED');}server.room.state='running';server.room.revision++;}
  return {room:structuredClone(server.room)};
 }};
 const controller=factory({...dom,client,schedule:f=>scheduled.push(f)});await controller.ready;
 await dom.submit('create',{name:'Room',seed:424242,mode:'core',visibility:'public'});
 const poll=()=>scheduled.shift()();await poll();
 return {dom,client,controller,server,calls,poll};
}
test('current click refreshes unchanged inspected content after cached revision changes',async()=>{
  const x=await setup(createHostController);try{
   x.server.room.revision=2;x.server.reports[0].observationRevision=2;
   await x.dom.click('start');
   assert.equal(x.server.room.state,'running');
   assert.match(x.dom.get('start-feedback').textContent,/Start confirmed/);const request=x.calls.find(c=>c.payload.finalStart);assert.equal(request.room.revision,2);assert.equal(request.payload.finalStart.participants[0].observationRevision,2);
  }finally{x.controller.dispose();}
});
test('pending background poll gets immediate nearby feedback and exactly one queued Start',async()=>{
 const x=await setup();try{
  const entered=deferred(),held=deferred();let blocked=false;
  x.server.beforeRpc=async p=>{if(p.roomGet&&!blocked){blocked=true;entered.resolve();await held.promise;}};
  const polling=x.poll();await entered.promise;
  const first=x.dom.click('start'),duplicate=x.dom.click('start');
  assert.match(x.dom.get('start-feedback').textContent,/Waiting for the current room refresh/);
  assert.equal(x.dom.get('start').disabled,true);assert.equal(x.dom.get('players').children[0].children[0].children[0].disabled,true);
  held.resolve();await Promise.all([polling,first,duplicate]);assert.equal(x.server.submissions,1);assert.equal(x.server.room.state,'running');
  await x.poll();assert.match(x.dom.get('start-feedback').textContent,/Start confirmed/,'poll does not erase result');
 }finally{x.controller.dispose();}
});
test('one confirmed stale refusal retries only unchanged intent, with a new fresh metadata revision',async()=>{
 const x=await setup();try{
  let racing=false;x.server.beforeRpc=async p=>{if(p.finalStart&&!racing){racing=true;x.server.room.revision++;x.server.reports[0].observationRevision++;throw refusal('STALE_REVISION');}};
  await x.dom.click('start');const requests=x.calls.filter(c=>c.payload.finalStart);assert.equal(requests.length,2);assert.equal(requests[1].room.revision,2);assert.equal(requests[1].payload.finalStart.participants[0].observationRevision,2);assert.match(x.dom.get('start-feedback').textContent,/confirmed/);
 }finally{x.controller.dispose();}
});
test('refresh never starts changed roster, world, configuration, content or not-ready games',async()=>{
 for(const [name,change,reason]of [
  ['new-player',s=>s.room.members.push({desktopId:'new',accountId:'new',ready:true,present:true}),/Players joined/],
  ['removed-player',s=>s.room.members=[],/Players joined/],
  ['new-world',s=>s.reports[0].worldToken='replacement',/world or game attachment changed/],
  ['new-attachment',s=>s.reports[0].attachmentId='replacement',/world or game attachment changed/],
  ['new-game',s=>s.reports[0].gameInstanceId='replacement',/world or game attachment changed/],
  ['content',s=>s.reports[0].contentDigest='b'.repeat(64),/content changed/],
  ['config',s=>{s.room.configRevision++;s.room.config.seed=99;},/configuration or host changed/],
  ['run',s=>s.room.runId='replacement',/configuration or host changed/],
  ['offline',s=>{s.room.members[0].ready=false;s.room.members[0].readinessReason='game_offline';},/not ready: game_offline/],
  ['not-loaded',s=>s.reports[0].loaded=false,/not a complete loaded protected world/],
  ['no-report',s=>s.reports=[],/Inspected world missing/],
 ]){
  const x=await setup();try{change(x.server);await x.dom.click('start');assert.equal(x.server.submissions,0,name);assert.match(x.dom.get('start-feedback').textContent,reason,name);}finally{x.controller.dispose();}
 }
});
test('same ready world after a synthetic task-switch pause is not refused for capture age',async()=>{
 const x=await setup();try{x.server.reports[0].capturedAt='2020-01-01T00:00:00Z';x.server.room.revision++;await x.dom.click('start');assert.equal(x.server.submissions,1);assert.match(x.dom.get('start-feedback').textContent,/confirmed/);}finally{x.controller.dispose();}
});
test('refresh preserves an explicitly excluded player and never adds them to Start',async()=>{
 const x=await setup();try{
  x.server.room.members.push({desktopId:'excluded',accountId:'excluded-account',present:false,ready:false,included:false});await x.poll();
  await x.dom.click('start');const request=x.calls.find(c=>c.payload.finalStart).payload.finalStart;
  assert.deepEqual(request.excludedDesktopIds,['excluded']);assert.deepEqual(request.participants.map(p=>p.desktopId),['desktop']);
 }finally{x.controller.dispose();}
});
test('a content change after confirmed stale rejection requires review instead of a second submission',async()=>{
 const x=await setup();try{x.server.beforeRpc=async p=>{if(p.finalStart){x.server.reports[0].contentDigest='b'.repeat(64);throw refusal('REPORT_CHANGED');}};await x.dom.click('start');assert.equal(x.calls.filter(c=>c.payload.finalStart).length,1);assert.match(x.dom.get('start-feedback').textContent,/content changed/);}finally{x.controller.dispose();}
});
test('a changed run during a queued Start cancels the old intent without submitting it',async()=>{
 const x=await setup();try{
  const entered=deferred(),held=deferred();let blocked=false;x.server.beforeRpc=async p=>{if(p.roomGet&&!blocked){blocked=true;entered.resolve();await held.promise;}};
  const polling=x.poll();await entered.promise;const pending=x.dom.click('start');x.server.room.runId='replacement';held.resolve();await Promise.all([polling,pending]);assert.equal(x.server.submissions,0);assert.equal(x.dom.get('start-feedback').textContent,'');
 }finally{x.controller.dispose();}
});
test('second confirmed stale refusal stops bounded refresh and gives durable nearby refusal',async()=>{
 const x=await setup();try{x.server.beforeRpc=async p=>{if(p.finalStart)throw refusal('REPORT_CHANGED');};await x.dom.click('start');assert.equal(x.calls.filter(c=>c.payload.finalStart).length,2);assert.match(x.dom.get('start-feedback').textContent,/selected observation changed/);await x.poll();assert.match(x.dom.get('start-feedback').textContent,/selected observation changed/);}finally{x.controller.dispose();}
});
test('uncertain or malformed mutation reply never creates another original; later read can confirm running',async()=>{
 for(const mode of ['lost','unverified-stale','malformed']){
  const x=await setup();try{
   x.server.beforeRpc=async p=>{if(p.finalStart){if(mode==='malformed')return {room:{...x.server.room,state:'staging'}};if(mode==='unverified-stale')throw new ServiceError('STALE_REVISION');throw Error('lost reply');}};
   await x.dom.click('start');await x.dom.click('start');assert.equal(x.calls.filter(c=>c.payload.finalStart).length,1);assert.equal(x.dom.get('start').disabled,true);assert.match(x.dom.get('start-feedback').textContent,/outcome is not confirmed/);
   await x.poll();assert.match(x.dom.get('start-feedback').textContent,/outcome is not confirmed/,'staging read cannot prove pending mutation never committed');
   x.server.room.state='running';await x.poll();assert.match(x.dom.get('start-feedback').textContent,/current state has been confirmed/);
  }finally{x.controller.dispose();}
 }
});
test('late Start reply after page close does not restore old room or feedback',async()=>{
 const x=await setup();try{const entered=deferred(),held=deferred();x.server.beforeRpc=async p=>{if(p.finalStart){entered.resolve();return held.promise;}};const pending=x.dom.click('start');await entered.promise;await x.dom.window.fire('pagehide');held.resolve({room:{...x.server.room,state:'running'}});await pending;assert.equal(x.dom.get('room-panel').hidden,true);assert.equal(x.dom.get('start-feedback').textContent,'');}finally{x.controller.dispose();}
});
