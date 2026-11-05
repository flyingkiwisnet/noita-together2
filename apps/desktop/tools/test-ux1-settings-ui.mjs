import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {fixture} from './ux1-dom.mjs';
import {LOBBY_SETTING_HINTS as hints} from '../ui/lobby-setting-hints.js';
assert.deepEqual(readFileSync(new URL('../ui/lobby-setting-hints.js',import.meta.url)),readFileSync(new URL('../../../packages/contracts/src/lobby-setting-hints.js',import.meta.url)));
const source=readFileSync(new URL('./test-ux1-ui.mjs',import.meta.url),'utf8').replaceAll('\r\n','\n');const {base,signed,joined}=vm.runInNewContext(source.slice(source.indexOf('const base='),source.indexOf('\n{\n'))+';({base,signed,joined})');
const rules={NT_sync_hearts:true,NT_sync_orbs:true,NT_sync_shift:true,NT_team_perks:true,NT_sync_perks:false,NT_alt_heartache:false,NT_ban_perks:false,NT_sync_orbs_no_curse:false,NT_ondeath_kick:false,NT_sync_orb_count:0,NT_death_penalty:'end'};
const transfers=['NT_send_gold','NT_send_wands','NT_send_flasks','NT_send_items'];
const view=structuredClone(joined);view.ux1={...view.ux1,compatibility:{...view.ux1.compatibility,settingKeys:[...transfers,'NT_testing_mode'],capabilities:[...view.ux1.compatibility.capabilities,'core.world-rules']},access:{visibility:'public',admittedAccountIds:[]},rules:{player:{revision:4,rules},world:{rules:{revision:5,rules:{NT_sync_steve:true}}}}};
view.room.config.settings=[{key:'NT_send_gold',value:'false'},{key:'unknown-kept',value:'unchanged'}];
const event=form=>({preventDefault(){},currentTarget:form});
const mutations=new Set(['set_player_rules','set_world_rules','set_lobby_policy','set_lobby_access','configure_lobby']);
const sent=f=>f.calls.filter(c=>mutations.has(c.input?.action));
function model(initial=view,hook){let state=structuredClone(initial);return {get state(){return state;},handler:async(command,input)=>{if(hook){const result=await hook(command,input,state);if(result!==undefined)return result;}const kind=input?.action;if(mutations.has(kind)){if(['configure_lobby','set_lobby_policy','set_lobby_access'].includes(kind)){assert.equal(input.revision,state.room.revision);state.room.revision++;}if(kind==='configure_lobby'){state.room.config=structuredClone(input.config);state.room.configRevision++;}if(kind==='set_lobby_policy'){Object.assign(state.room,input.policy);if('password' in input.policy)state.room.passwordRequired=Boolean(input.policy.password);}if(kind==='set_lobby_access')state.ux1.access=structuredClone(input.access);if(kind==='set_player_rules'){assert.equal(input.player_rules.expectedRevision,state.ux1.rules.player.revision);state.ux1.rules.player={revision:state.ux1.rules.player.revision+1,rules:structuredClone(input.player_rules.rules)};}if(kind==='set_world_rules'){assert.equal(input.world_rules.expectedRevision,state.ux1.rules.world.rules.revision);state.ux1.rules.world.rules={revision:state.ux1.rules.world.rules.revision+1,rules:structuredClone(input.world_rules.rules)};}}return structuredClone(state);}};}
function change(f,id,name,value){const field=f.el(id).elements[name];if(field.type==='checkbox')field.checked=value;else field.value=String(value);field.dispatchEvent({type:'change'});return field;}
{
 const m=model(),f=await fixture(view,m.handler);for(const group of ['world','sharing','transfers','death'])assert.ok(f.el('settings-'+group));
 assert.equal(f.el('create-open').parentElement,f.el('refresh-lobbies').parentElement);
 assert.equal(f.context.document.querySelectorAll('[data-reload]').length,0);
 for(const id of ['lobby-config','lobby-policy','lobby-access','live-player-rules','live-world-rules'])assert.equal(f.el(id).querySelectorAll('button[type=submit]').length,0);
 for(const key of transfers){const input=f.el('lobby-config').elements['setting-'+key],row=input.parentElement.parentElement;assert.equal(input.type,'checkbox');assert.equal(input.getAttribute('form'),'lobby-config');assert.equal(row.querySelector('.setting-hint').textContent,hints[key].help);assert.equal(row.querySelector('.setting-help').title,undefined);assert.equal(input.getAttribute('aria-describedby'),null);assert.equal(row.querySelector('.setting-help').getAttribute('aria-describedby'),row.querySelector('.setting-hint').id);}
 await f.el('save-settings').onclick();assert.equal(sent(f).length,0);assert.match(f.el('settings-status').textContent,/No settings changed/);
 change(f,'lobby-config','setting-NT_send_gold',true);change(f,'lobby-policy','capacity',7);change(f,'lobby-access','visibility','private');change(f,'live-player-rules','NT_ondeath_kick',true);change(f,'live-player-rules','NT_sync_orb_count',3);change(f,'live-world-rules','NT_sync_steve',false);
 await f.el('save-settings').onclick();assert.deepEqual(sent(f).map(c=>c.input.action),[...mutations]);
 const config=sent(f)[4].input.config;assert.equal(config.settings.find(s=>s.key==='NT_send_gold').value,'true');assert.equal(config.settings.some(s=>s.key==='NT_send_wands'),false);assert.equal(config.settings.find(s=>s.key==='unknown-kept').value,'unchanged');assert.equal(sent(f)[0].input.player_rules.rules.NT_sync_orb_count,3);assert.match(f.el('settings-status').textContent,/Settings saved/);
 await f.el('save-settings').onclick();assert.equal(sent(f).length,5);
 const button=f.el('config-fields').querySelectorAll('button').find(n=>n.parentElement.dataset.settingKey==='generateSharedSeed'&&n.className==='setting-caption'),before=f.calls.length;button.onclick();const form=f.el('lobby-config');assert.equal(form.elements.seedPolicy.value,'shared-fixed');assert.ok(Number(form.elements.seed.value)>0&&Number(form.elements.seed.value)<=4294967295);assert.equal(form.elements.seed.parentElement.parentElement.hidden,false);assert.equal(f.calls.length,before);await f.el('save-settings').onclick();assert.equal(sent(f).at(-1).input.config.seed,Number(form.elements.seed.value));
}
{
 const m=model(),f=await fixture(view,m.handler);change(f,'lobby-config','seed',555);change(f,'lobby-policy','capacity',999);await f.el('save-settings').onclick();assert.equal(sent(f).length,0);assert.match(f.el('settings-status').textContent,/capacity/);change(f,'lobby-policy','capacity',8);change(f,'live-player-rules','NT_sync_orb_count','-1');await f.el('save-settings').onclick();assert.equal(sent(f).length,0);assert.match(f.el('settings-status').textContent,/whole-number/);
}
{
 let refuse=true;const m=model(view,async(command,input)=>{if(input?.action==='set_lobby_policy'&&refuse)throw 'CAPACITY';}),f=await fixture(view,m.handler);change(f,'lobby-config','seed',556);change(f,'lobby-policy','capacity',7);change(f,'live-player-rules','NT_sync_hearts',false);await f.el('save-settings').onclick();assert.deepEqual(sent(f).map(c=>c.input.action),['set_player_rules','set_lobby_policy']);assert.equal(f.el('live-player-rules').dataset.dirty,'');assert.equal(f.el('lobby-policy').elements.capacity.value,'7');assert.equal(f.el('live-player-rules').elements.NT_sync_hearts.checked,false);assert.match(f.el('settings-status').textContent,/Saved Sharing and death.*Remaining groups/);refuse=false;await f.el('save-settings').onclick();assert.deepEqual(sent(f).map(c=>c.input.action),['set_player_rules','set_lobby_policy','set_lobby_policy','configure_lobby']);
}
{
 const m=model(view,async(command,input,state)=>{if(input?.action==='set_world_rules'){state.ux1.pending=true;state.ux1.outcome={kind:'WorldRules',id:'original-settings-identity'};throw 'Connection lost';}}),f=await fixture(view,m.handler);change(f,'lobby-config','seed',557);change(f,'live-player-rules','NT_sync_hearts',false);change(f,'live-world-rules','NT_sync_steve',false);await f.el('save-settings').onclick();assert.deepEqual(sent(f).map(c=>c.input.action),['set_player_rules','set_world_rules']);assert.match(f.el('settings-status').textContent,/Saved Sharing and death.*Outcome unknown/);assert.equal(f.el('save-settings').disabled,true);assert.equal(f.el('live-world-rules').elements.NT_sync_steve.checked,false);await f.el('save-settings').onclick();assert.equal(sent(f).length,2);assert.equal(m.state.ux1.outcome.id,'original-settings-identity');await f.el('reload-settings').onclick();assert.equal(m.state.ux1.outcome.id,'original-settings-identity');assert.equal(f.el('live-world-rules').elements.NT_sync_steve.checked,true);assert.equal(f.el('save-settings').disabled,true);
}
{
 const m=model(),f=await fixture(view,m.handler);change(f,'lobby-config','seed',558);m.state.room.config.seed=999;m.state.room.revision++;await f.el('save-settings').onclick();assert.equal(sent(f).length,0);assert.equal(f.el('lobby-config').elements.seed.value,'558');assert.match(f.el('settings-status').textContent,/changed elsewhere/);await f.el('reload-settings').onclick();assert.equal(f.el('lobby-config').elements.seed.value,'999');assert.equal(f.el('lobby-config').dataset.dirty,'');
}
{
 let release;const gate=new Promise(r=>release=r);const m=model(view,async(command,input)=>{if(input?.action==='configure_lobby')await gate;}),f=await fixture(view,m.handler);change(f,'lobby-config','seed',559);const first=f.el('save-settings').onclick();await f.settle();assert.equal(f.el('save-settings').disabled,true);assert.equal(f.el('reload-settings').disabled,true);await f.el('save-settings').onclick();assert.equal(sent(f).length,1);release();await first;
}
{
 const running=structuredClone(view);running.room.state='running';const m=model(running),f=await fixture(running,m.handler);change(f,'lobby-config','seed',560);change(f,'live-player-rules','NT_sync_hearts',false);await f.el('save-settings').onclick();assert.equal(sent(f).length,0);assert.match(f.el('settings-status').textContent,/inactive lobby/);await f.el('reload-settings').onclick();change(f,'live-player-rules','NT_sync_hearts',false);await f.el('save-settings').onclick();assert.deepEqual(sent(f).map(c=>c.input.action),['set_player_rules']);
 m.state.account_id='peer';await f.render(structuredClone(m.state));assert.equal(f.el('save-settings').disabled,true);const count=sent(f).length;await f.el('save-settings').onclick();assert.equal(sent(f).length,count);
}
{
 const m=model(view,async(command,input,state)=>{if(input?.action==='refresh_lobby')state.ux1.rules.player.revision++;}),f=await fixture(view,m.handler);change(f,'live-player-rules','NT_sync_hearts',false);await f.el('save-settings').onclick();assert.equal(sent(f).length,0);assert.match(f.el('settings-status').textContent,/changed elsewhere/);
}
{
 const m=model(view,async(command,input,state)=>{if(input?.action==='refresh_lobby')state.room.hostAccountId='different-host';}),f=await fixture(view,m.handler);change(f,'lobby-config','seed',561);change(f,'live-player-rules','NT_sync_hearts',false);await f.el('save-settings').onclick();assert.equal(sent(f).length,0);assert.match(f.el('settings-status').textContent,/Host permission/);assert.equal(f.el('lobby-config').elements.seed.value,'561');
}
{
 const m=model(view,async(command,input,state)=>{if(input?.action==='set_player_rules'){state.room.config.seed=999;state.room.revision++;}}),f=await fixture(view,m.handler);change(f,'lobby-config','seed',562);change(f,'live-player-rules','NT_sync_hearts',false);await f.el('save-settings').onclick();assert.deepEqual(sent(f).map(c=>c.input.action),['set_player_rules']);assert.equal(f.el('lobby-config').elements.seed.value,'562');assert.match(f.el('settings-status').textContent,/Saved Sharing and death.*changed elsewhere/);
}
{
 const f=await fixture(base);assert.equal(f.el('create-open').disabled,true);f.el('create-open').onclick();assert.ok(!f.el('create-dialog').open);assert.equal(f.calls.some(c=>c.input?.action==='create_lobby'),false);
}
{
 const f=await fixture(signed,async(command,input)=>{if(input?.action==='create_lobby')throw 'Room name rejected';});f.el('create-open').focus();f.el('create-open').onclick();const form=f.el('create-form');assert.equal(f.el('create-dialog').open,true);assert.equal(form.elements.name.disabled,true);assert.equal(f.context.document.activeElement,form.elements.capacity);const seedButton=f.el('create-fields').querySelectorAll('button').find(n=>n.parentElement.dataset.settingKey==='generateSharedSeed'&&n.className==='setting-caption');const beforeSeed=f.calls.length;seedButton.onclick();assert.equal(form.elements.seedPolicy.value,'shared-fixed');assert.ok(Number(form.elements.seed.value)>0);assert.equal(form.elements.seed.parentElement.parentElement.hidden,false);assert.equal(f.calls.length,beforeSeed);form.elements.password.value='draft password';form.elements.capacity.value='99';await form.onsubmit(event(form));assert.match(f.el('create-error').textContent,/capacity/);assert.equal(f.calls.some(c=>c.input?.action==='create_lobby'),false);form.elements.capacity.value='8';await form.onsubmit(event(form));assert.match(f.el('create-error').textContent,/rejected/);assert.equal(form.elements.password.value,'draft password');f.el('create-dialog').dispatchEvent({type:'cancel',preventDefault(){}});assert.equal(f.el('create-dialog').open,false);assert.equal(f.context.document.activeElement,f.el('create-open'));f.el('create-open').onclick();assert.equal(form.elements.password.value,'draft password');
 const pending={...signed,ux1:{...signed.ux1,pending:true,outcome:{kind:'Create',id:'original-create'}}};await f.render(pending);f.el('create-close').onclick();assert.equal(f.el('create-dialog').open,false);assert.equal(f.el('create-open').disabled,true);const before=f.calls.filter(c=>c.input?.action==='create_lobby').length;await form.onsubmit(event(form));assert.equal(f.calls.filter(c=>c.input?.action==='create_lobby').length,before);assert.equal(form.elements.password.value,'draft password');
 await f.render({...signed,server_origin:'https://different.example',session_epoch:'new'});assert.equal(form.elements.password,undefined);assert.equal(f.el('create-dialog').open,false);
}
{
 let release;const pending=new Promise(resolve=>release=resolve);const f=await fixture(signed,async(command,input)=>{if(input?.action==='create_lobby'){await pending;return {...view,room:{...view.room,roomId:'created'}};}});f.el('create-open').onclick();const form=f.el('create-form');const first=form.onsubmit(event(form));await f.settle();await form.onsubmit(event(form));assert.equal(f.calls.filter(c=>c.input?.action==='create_lobby').length,1);assert.equal(f.el('create-open').disabled,true);release();await first;assert.equal(f.el('create-dialog').open,false);assert.equal(f.el('lobby-screen').hidden,false);
}
{
 const live=structuredClone(view);live.room.state='running';live.room.publicStats=false;
 const m=model(live),f=await fixture(live,m.handler),config=structuredClone(m.state.room.config);
 const field=f.el('lobby-statistics').elements.publicStats;assert.equal(field.checked,false);assert.equal(field.parentElement.parentElement.querySelector('.setting-hint').textContent,hints.publicStats.help);
 change(f,'lobby-statistics','publicStats',true);await f.el('save-settings').onclick();
 assert.deepEqual(JSON.parse(JSON.stringify(sent(f).map(c=>c.input.policy))),[{publicStats:true}]);assert.deepEqual(m.state.room.config,config);assert.equal(m.state.room.state,'running');assert.equal(m.state.room.publicStats,true);
 change(f,'lobby-statistics','publicStats',false);await f.el('reload-settings').onclick();assert.equal(f.el('lobby-statistics').elements.publicStats.checked,true);
 const guest=structuredClone(live);guest.room.hostAccountId='another';const g=await fixture(guest);assert.equal(g.el('lobby-statistics').elements.publicStats.disabled,true);
}
{
 const typed=structuredClone(view);typed.ux1.compatibility.modes=['core','race'];const m=model(typed),f=await fixture(typed,m.handler);
 const mode=f.el('lobby-config').elements.mode;assert.equal(mode.tagName.toLowerCase(),'input');assert.equal(mode.readOnly,true);
 mode.value='race';change(f,'lobby-config','seed',123456);await f.el('save-settings').onclick();
 assert.equal(sent(f).find(c=>c.input.action==='configure_lobby').input.config.mode,'core','Save preserves canonical lobby type despite a modified field');
 await f.el('reload-settings').onclick();assert.equal(f.el('lobby-config').elements.mode.value,'core');assert.equal(f.el('lobby-config').elements.mode.readOnly,true);
 f.el('create-open').onclick();const choice=f.el('create-form').elements.mode;assert.equal(choice.tagName.toLowerCase(),'select');assert.equal([...choice.children].some(o=>o.value==='race'),true);
}
console.log('PASS grouped single Save/Reload and Public stats during playing without configuration changes; exact hints, permissions, partial/unknown/stale handling and existing settings regressions');
