import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';
import {RACE_DEFAULT,validateRaceDraft,applyRaceDraft,raceGoalLabel} from '../../../packages/contracts/src/race-goal-ui.js';
import {createRaceEditor} from '../../../packages/contracts/src/race-goal-editor.js';
import {createRaceTransport,createRaceStatisticsTransport,renderRaceStatistics} from '../candidate/race-view.js';
import {controllerDom} from './dom-fixture.mjs';
import {createHostController} from '../candidate/app.js';
const json=p=>JSON.parse(fs.readFileSync(new URL(p,import.meta.url)));
const catalogue={schema:json('../../../packages/contracts/g3-race-goals/schema.json'),content:json('../../../packages/contracts/g3-race-goals/catalogue.json')};
const schema=json('../../../packages/contracts/g3-race/schema.json'),locator='10000000-0000-4000-8000-000000000001';
test('Race common UI validates supported manual sets, finite bounds and Classic33 conflict',()=>{
 assert.deepEqual(validateRaceDraft(RACE_DEFAULT,catalogue),RACE_DEFAULT);
 const named={instanceId:'named',id:'defeat_named_bosses',bossIds:['pyramid_boss','high_alchemist']};
 assert.match(raceGoalLabel(named,catalogue),/Pyramid Boss, High Alchemist/);
 for(const patch of [{durationMs:-1},{goals:[]},{goals:[null]},{goals:[{...named,bossIds:['unknown']}]},{goals:[{...named,bossIds:['pyramid_boss','pyramid_boss']}]},{goals:[{instanceId:'hp',id:'reach_max_hp',target:499}]},{goals:[{instanceId:'end',id:'classic_ending'},{instanceId:'orbs',id:'collect_orbs',target:33}]}])assert.throws(()=>validateRaceDraft({...RACE_DEFAULT,...patch},catalogue),e=>e.code==='INVALID_MESSAGE');
 const transformed=applyRaceDraft({mode:'race',seed:1,seedPolicy:'shared-fixed',requiredCapabilities:['core.seed'],settings:[{key:'NT_send_gold',value:'true'},{key:'NT_sync_hearts',value:'true'},{key:'NT_testing_mode',value:'true'}]},JSON.stringify(RACE_DEFAULT),catalogue);
 assert.ok(transformed.requiredCapabilities.includes('race.goals.v1'));assert.ok(transformed.settings.every(s=>s.value==='false'));
 const core=applyRaceDraft({...transformed,mode:'core'},undefined,catalogue);assert.equal(core.race,undefined);assert.equal(core.requiredCapabilities.includes('race.goals.v1'),false);
});
test('Creation drafts preserve the numeric orb cap when selecting Race or Core',()=>{
 for(const value of ['0','9']){
  const original={mode:'core',seed:1,seedPolicy:'shared-fixed',requiredCapabilities:['core.seed'],settings:[{key:'NT_sync_orb_count',value},{key:'NT_sync_orbs',value:'true'}]};
  const race=applyRaceDraft({...original,mode:'race'},JSON.stringify(RACE_DEFAULT),catalogue);
  const core=applyRaceDraft({...race,mode:'core'},undefined,catalogue);
  for(const config of [race,core]){
   assert.equal(config.settings.find(s=>s.key==='NT_sync_orb_count').value,value);
   assert.equal(config.settings.find(s=>s.key==='NT_sync_orbs').value,'false');
  }
  assert.deepEqual(original.settings,[{key:'NT_sync_orb_count',value},{key:'NT_sync_orbs',value:'true'}]);
  assert.equal(core.race,undefined);assert.deepEqual(core.requiredCapabilities,['core.seed']);
 }
});
test('Fresh Race draft defaults only to Classic Ending without overwriting loaded or dirty goals',async()=>{
 const d=await controllerDom('../candidate/index.html'),form=d.get('create');form.elements.mode.value='race';const editor=createRaceEditor(d.document,form,catalogue,RACE_DEFAULT);
 assert.deepEqual(JSON.parse(form.elements.raceDraft.value),{registryRevision:'race-goals-1',durationMs:0,goals:[{instanceId:'classic',id:'classic_ending'}]});
 const saved={registryRevision:'race-goals-1',durationMs:120000,goals:[{instanceId:'kept_hp',id:'reach_max_hp',target:500}]},original=structuredClone(saved);editor.load(saved);assert.deepEqual(JSON.parse(form.elements.raceDraft.value),saved);
 const list=editor.fieldset.children.find(c=>c.className==='race-goal-list'),row=list.children[0],target=row.children[1].children[0].children[0];target.value='1000';await row.fire('input');const dirty=JSON.parse(form.elements.raceDraft.value);assert.equal(dirty.goals[0].target,1000);assert.equal(dirty.goals[0].instanceId,'kept_hp');
 form.elements.mode.value='core';await form.elements.mode.fire('change');form.elements.mode.value='race';await form.elements.mode.fire('change');editor.mode(false);editor.mode(true);assert.deepEqual(JSON.parse(form.elements.raceDraft.value),dirty);assert.deepEqual(saved,original);
});
test('Compact goal rows expose labelled X removal, preserve IDs and move keyboard focus predictably',async()=>{
 const d=await controllerDom('../candidate/index.html'),form=d.get('create');form.elements.mode.value='race';const editor=createRaceEditor(d.document,form,catalogue,RACE_DEFAULT),list=editor.fieldset.children.find(c=>c.className==='race-goal-list'),add=editor.fieldset.children.find(c=>c.className==='race-goal-add');
 const first=list.children[0];assert.equal(first.children[0].getAttribute('aria-label'),'Goal type');assert.equal(first.children[2].textContent,'×');assert.match(first.children[2].getAttribute('aria-label'),/^Remove classic ending goal$/i);assert.equal(first.children[1].hidden,true);
 add.focus();await add.fire('click');const second=list.children[1],id=JSON.parse(form.elements.raceDraft.value).goals[1].instanceId;assert.equal(d.document.activeElement,second.children[0]);assert.equal(JSON.parse(form.elements.raceDraft.value).goals[0].id,'classic_ending');
 second.children[0].value='visit_parallel_world';await second.children[0].fire('change');assert.equal(second.dataset.goal,'visit_parallel_world');assert.equal(second.children[1].children.length,2);assert.equal(JSON.parse(form.elements.raceDraft.value).goals[1].instanceId,id);
 second.children[2].focus();await second.children[2].fire('click');assert.equal(d.document.activeElement,first.children[0]);assert.equal(JSON.parse(form.elements.raceDraft.value).goals.length,1);first.children[2].focus();await first.children[2].fire('click');assert.equal(d.document.activeElement,add);assert.equal(JSON.parse(form.elements.raceDraft.value).goals.length,0);assert.throws(()=>validateRaceDraft(JSON.parse(form.elements.raceDraft.value),catalogue));
});
test('Race import-free editor roundtrips all eight goal kinds and keeps explicit duration',async()=>{
 const d=await controllerDom('../candidate/index.html'),form=d.get('configure');form.elements.mode.value='race';
 const editor=createRaceEditor(d.document,form,catalogue,RACE_DEFAULT),goals=[
  {instanceId:'end',id:'classic_ending'},{instanceId:'orbs',id:'collect_orbs',target:1},{instanceId:'bosses',id:'defeat_bosses',target:2},
  {instanceId:'named',id:'defeat_named_bosses',bossIds:['pyramid_boss','high_alchemist']},{instanceId:'biome',id:'visit_biome',biomeId:'ancient_laboratory'},
  {instanceId:'pw',id:'visit_parallel_world',direction:'east',index:1},{instanceId:'hp',id:'reach_max_hp',target:500},{instanceId:'gold',id:'accumulate_gold',target:10000}];
 editor.load({registryRevision:'race-goals-1',durationMs:60000,goals});const value=validateRaceDraft(JSON.parse(form.elements.raceDraft.value),catalogue);
 assert.equal(value.goals.length,8);assert.equal(value.durationMs,60000);assert.deepEqual(new Set(value.goals.find(g=>g.id==='defeat_named_bosses').bossIds),new Set(['pyramid_boss','high_alchemist']));
 const named=editor.fieldset.children.find(c=>c.className==='race-goal-list').children.find(r=>r.dataset.goal==='defeat_named_bosses');assert.equal(named.children[1].children[0].className,'race-goal-parameter is-multiple');assert.equal(named.children[1].children[0].children[0].multiple,true);assert.equal(named.children[1].children[0].children[0].size,6);
 editor.mode(false);assert.equal(editor.fieldset.disabled,true);form.elements.mode.value='core';editor.mode(true);assert.equal(editor.fieldset.hidden,true);
});
test('Race transport rejects stale service/correlation and pages one coherent snapshot',async()=>{
 const info={serviceId:'fixture',serviceEpoch:'epoch',compatibility:{capabilities:['race.goals.v1']}},calls=[];
 let stale=false;
 const client={info,generation:1,credentials:null,async transport(path,q){calls.push({path,q});if(path==='/v3/race/schema')return schema;if(path==='/v3/race/catalogue')return catalogue;return {major:1,revision:'g3-race-1',serviceId:'fixture',serviceEpoch:'epoch',requestId:stale?'wrong':q.requestId,snapshot:{locator,lobbyMode:'race',runKey:locator,revision:'1',status:'running',collectionState:'available',entrants:[{rowKey:q.read.afterRow?'b':'a'}],nextRow:q.read.afterRow?'':'a'}};}};
 let fallback=0;const request=createRaceStatisticsTransport(client,async()=>{fallback++;return {};});
 const value=await request({read:{locator}});assert.equal(value.race.entrants.length,2);assert.equal(fallback,0);assert.equal(calls.filter(c=>c.q?.read).length,2);
 stale=true;await assert.rejects(createRaceTransport(client)({read:{locator}}),e=>e.code==='INVALID_MESSAGE');
});
test('Race dashboard keeps duplicate names distinct and renders selected boss names safely',async()=>{
 const d=await controllerDom('../candidate/index.html'),table=d.document.createElement('table'),head=d.document.createElement('thead');table.append(head,d.get('statistics-rows'));
 const goal={instanceId:'named',id:'defeat_named_bosses',bossIds:['pyramid_boss']};
 const entry={displayName:'Same <script>alert(1)</script>',rank:1,status:'racing',completedGoals:0,finalCountAtMs:'',progress:[{instanceId:'named',value:'0',target:'1',complete:false,credits:[]}]};
 renderRaceStatistics(d.document,{locator,lobbyName:'Race',status:'running',collectionState:'available',startedAt:'2026-11-05T01:00:00Z',elapsedMs:'1000',config:{goals:[goal]},catalogue,entrants:[{...entry,rowKey:'a'},{...entry,rowKey:'b'}]});
 assert.equal(d.get('statistics-rows').children.length,4);assert.match(d.get('statistics-rows').textContent,/Pyramid Boss/);assert.match(d.get('statistics-rows').textContent,/<script>/);assert.equal(d.get('statistics-rows').children[0].children[1].children.length,0);
 renderRaceStatistics(d.document,{lobbyName:'Race',collectionState:'expired'});assert.equal(d.get('statistics-rows').children.length,0);assert.match(d.get('statistics-status').textContent,/expired/);
});
test('Race shipped controller creates and saves selected goals through existing host settings',async()=>{
 const d=await controllerDom('../candidate/index.html'),calls=[];
 const info={serviceId:'fixture',serviceEpoch:'epoch',revision:'g2-draft-5',mockProvider:true,profile:{id:'fixture',evidenceStatus:'synthetic'},compatibility:{modes:['core','race'],capabilities:['core.seed','core.seed-policy.v1','race.goals.v1'],settingKeys:[]}};
 let room;
 const client={credentials:{accountId:'host',sessionId:'desktop',sessionEpoch:'session'},generation:1,schema:{revision:'g2-draft-5'},info,async initialize(){return this.info;},dispose(){},async transport(path){assert.equal(path,'/v3/race/catalogue');return catalogue;},async rpc(p){calls.push(p);if(p.roomList)return {rooms:[]};if(p.chatList)return {};if(p.roomCreate)room={roomId:'room',name:p.roomCreate.name,hostAccountId:'host',state:'waiting',revision:1,configRevision:1,config:p.roomCreate.config,members:[]};if(p.configure){room.config=p.configure;room.revision++;room.configRevision++;}return {room:structuredClone(room)};}};
 const c=createHostController({...d,client,schedule(){}});try{
  await c.ready;await d.submit('create',{name:'Manual race',seed:1,seedPolicy:'shared-fixed',mode:'race',visibility:'public'});
  assert.deepEqual(room.config.race,RACE_DEFAULT);assert.ok(room.config.requiredCapabilities.includes('race.goals.v1'));
  const edit=d.get('configure'),draft={...RACE_DEFAULT,goals:[{instanceId:'hp1000',id:'reach_max_hp',target:1000}],durationMs:60000};assert.equal(d.get('create').elements.mode.tagName,'select');assert.equal(edit.elements.mode.tagName,'input');assert.equal(edit.elements.mode.readOnly,true);edit.elements.mode.value='core';edit.elements.raceDraft.value=JSON.stringify(draft);await edit.fire('input');
  await d.click('save-settings');assert.deepEqual(room.config.race,draft);assert.equal(room.config.mode,'race');assert.equal(calls.filter(p=>p.configure).length,1);assert.equal(calls.filter(p=>p.roomCreate).length,1);
 }finally{c.dispose();}
});
