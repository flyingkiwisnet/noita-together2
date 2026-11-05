// Real desktop helpers + canonical service code, with owned synthetic rows/SQL.
// No database/service/game is started and no live mutation is performed.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {randomUUID} from 'node:crypto';
import {fixture} from './ux1-dom.mjs';
import {CandidateRooms} from '../../service/src/g2-next/rooms.ts';
import {C1Authority} from '../../service/src/g3/c1.ts';
import {C2Authority} from '../../service/src/g3/c2.ts';
import {schema} from '../../../packages/contracts/src/g2-next.ts';
const source=readFileSync(new URL('./test-ux1-ui.mjs',import.meta.url),'utf8').replaceAll('\r\n','\n');
const {joined}=vm.runInNewContext(source.slice(source.indexOf('const base='),source.indexOf('\n{\n'))+';({joined})');
let checks=0;const eq=(a,b)=>{assert.equal(a,b);checks++;};
const auth={principal:async()=>({kind:'registered'}),now:()=>Date.now(),security:{seal:v=>v}};
const service=new CandidateRooms({serviceId:'fixture',epoch:'epoch'},auth,{});
const roomId=randomUUID(),accountId=randomUUID(),actor={kind:'desktop',accountId,sessionId:randomUUID(),sessionEpoch:'session'};
const row={id:roomId,host_account_id:accountId,state:'ended',revision:7,config_revision:2,current_run_id:randomUUID(),config:{mode:'race'}};
let attached=false;const queries=[];
const tx={query:async(sql)=>{queries.push(sql);if(sql.startsWith('SELECT * FROM rooms'))return {rows:[row],rowCount:1};if(sql.includes('SELECT 1 FROM attachments'))return {rows:[],rowCount:attached?1:0};return {rows:[],rowCount:0};}};
const request=extra=>({major:1,revision:schema.revision,serviceId:'fixture',serviceEpoch:'epoch',sessionEpoch:'session',requestId:randomUUID(),roomId,expectedRevision:7,configRevision:2,...extra});
// Production Core-only read guards reproduce the real Race FORBIDDEN before
// any membership/gameplay lookup. The desktop must not issue those requests.
for(const Authority of [C1Authority,C2Authority]){await assert.rejects(Authority.prototype.apply.call({},tx,actor,{roomId,rulesGet:{}}),e=>e.code==='FORBIDDEN');checks++;}
const v=structuredClone(joined);v.room={...v.room,roomId,hostAccountId:accountId,state:'ended',members:[],config:{...v.room.config,mode:'race'}};v.account_id=accountId;v.ux1.compatibility.modes=["core","race"];v.ux1.access={visibility:"public",admittedAccountIds:[]};
const f=await fixture(v,async(_command,input)=>{if(['get_player_rules','get_world_rules'].includes(input?.action))throw 'FORBIDDEN';if(input?.action==='delete_lobby'){const result=await service.execute(tx,request({roomDelete:{}}),actor);eq(result.deletedRoomId,roomId);return {...v,room:null};}});
await f.read('uxRules()');await f.settle();eq(f.calls.some(c=>['get_player_rules','get_world_rules','recovery_list'].includes(c.input?.action)),false);eq(f.el('delete-lobby').disabled,false);eq(f.el('lobby-config').elements.seed.disabled,false);
const deleting=f.el('delete-lobby').onclick();await f.settle();await f.el('confirm-yes').onclick();await deleting;eq(f.calls.filter(c=>c.input?.action==='delete_lobby').length,1);eq(f.el('lobbies-screen').hidden,false);
eq(queries.some(q=>q.startsWith('DELETE FROM memberships')),true);eq(queries.some(q=>q.startsWith('UPDATE attachments SET revoked_at')),true);
attached=true;await assert.rejects(service.execute(tx,request({roomDelete:{}}),actor),e=>e.code==='INVALID_STATE');checks++;attached=false;
await assert.rejects(service.execute(tx,request({roomDelete:{}}),{...actor,accountId:randomUUID()}),e=>e.code==='FORBIDDEN');checks++;
await assert.rejects(service.execute(tx,request({roomDelete:{},expectedRevision:6}),actor),e=>e.code==='STALE_REVISION');checks++;
row.state='running';await assert.rejects(service.execute(tx,request({configure:joined.room.config}),actor),e=>e.code==='INVALID_STATE');checks++;
await assert.rejects(service.lobbyCommand(tx,row,{roomPolicy:{capacity:8}},actor),e=>e.code==='INVALID_STATE');checks++;
const running=structuredClone(v);running.room.state='running';const g=await fixture(running);eq(g.el('lobby-config').elements.seed.disabled,true);eq(g.el('lobby-policy').elements.capacity.disabled,true);eq(g.el('lobby-statistics').elements.publicStats.disabled,false);eq(g.el('lobby-access').elements.visibility.disabled,false);eq(g.el('end').disabled,false);assert.match(g.el('settings-permission').textContent,/even if everyone has left.*End run/);checks++;
// Locked drafts survive a state transition; Save never silently ends/retries.
const draft=await fixture(v);draft.el('lobby-config').elements.seed.value='123';draft.el('lobby-config').dataset.dirty='true';await draft.render(running);eq(draft.el('lobby-config').elements.seed.value,'123');await draft.el('save-settings').onclick();assert.match(draft.el('settings-status').textContent,/End run/);checks++;eq(draft.calls.some(c=>['configure_lobby','end_lobby'].includes(c.input?.action)),false);
const denied=await fixture(v,async(_command,input)=>{if(input?.action==='set_lobby_policy')throw 'FORBIDDEN';});denied.el('lobby-statistics').elements.publicStats.checked=true;await denied.el('save-settings').onclick();assert.match(denied.el('settings-status').textContent,/service refused.*draft.*FORBIDDEN/i);checks++;eq(denied.el('lobby-statistics').elements.publicStats.checked,true);
const core=structuredClone(running);core.room.config.mode='core';core.ux1.compatibility.capabilities.push('core.world-rules');core.ux1.rules={player:{revision:1,rules:{NT_sync_hearts:false,NT_death_penalty:'end'}},world:{rules:{revision:1,rules:{NT_sync_steve:false}}}};const live=await fixture(core);eq(live.el('live-player-rules').elements.NT_sync_hearts.disabled,false);eq(live.el('live-world-rules').elements.NT_sync_steve.disabled,false);
const unknown=structuredClone(v);unknown.ux1.pending=true;unknown.ux1.error='Outcome unconfirmed';const u=await fixture(unknown);eq(u.el('delete-lobby').disabled,true);eq(u.el('save-settings').disabled,true);
const readDenied=await fixture({...core,room:{...core.room,state:'ended'}},async(_command,input)=>{if(input?.action==='get_player_rules')throw 'FORBIDDEN';});await readDenied.read('uxRules()');assert.match(readDenied.el('settings-status').textContent,/Could not load Core sharing\/death rules.*FORBIDDEN.*No settings or deletion/);checks++;eq(readDenied.el('delete-lobby').disabled,false);eq(readDenied.calls.some(c=>c.input?.action==='delete_lobby'),false);
const next=structuredClone(v);next.room.roomId=randomUUID();await readDenied.render(next);eq(readDenied.el('settings-status').textContent,'');
console.log(JSON.stringify({passed:true,assertions:checks,productionDesktop:true,canonicalServiceMethods:true,syntheticSqlAndAuth:true,noDatabase:true,noGame:true}));
