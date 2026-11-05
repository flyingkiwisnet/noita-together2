import assert from 'node:assert/strict';
import {fixture} from './ux1-dom.mjs';
const member=(id,name)=>({desktopId:id,accountId:'account-'+id,principal:{kind:'registered',displayName:name},present:true,ready:true,readiness:'ready',included:true,consentedConfigRevision:1});
const a=member('a','Same name'),b=member('b','Same name'),c=member('c','<img src=x onerror=alert(1)> 中文 🌊');
const report=(m,mods,more={})=>({desktopId:m.desktopId,inventoryComplete:true,collectorFailure:'',activeMods:mods,...more});
const base={protocol_revision:'g2-draft-5',server_origin:'https://fixture.invalid',service_id:'fixture',service_epoch:'epoch',device_id:'device',signed_in:true,profile_selected:true,account_id:a.accountId,desktop_id:a.desktopId,session_epoch:'session',principal:a.principal,setup:{enabled:true,stage:'ready'},g3:{enabled:false},authentication:{discovered:true},ux1:{rules:{player:{revision:1,rules:{NT_ondeath_kick:true}}}},room:{roomId:'lobby',runId:'run',runEpoch:'run-epoch',name:'Lobby',hostAccountId:a.accountId,state:'staging',revision:1,configRevision:1,config:{seed:42,mode:'core'},members:[a,b,c]},inspections:[report(a,['shared','shared','Zulu','alpha','Alpha','__proto__']),report(b,['shared','Zulu']),report(c,['shared','<svg onload=x>'])]};
const rowNames=f=>f.el('mods-rows').children.map(r=>r.firstChild.firstChild.firstChild.textContent);
const button=(f,name)=>f.el('mods-rows').children.find(r=>r.firstChild.firstChild.firstChild.textContent===name).firstChild.firstChild;
{
 const f=await fixture(base);const tab=f.read("document.querySelectorAll('[data-lobby-tab=\"mods\"]')[0]");assert.ok(tab);await tab.onclick();assert.equal(f.el('tab-mods').hidden,false);
 assert.deepEqual(rowNames(f),['shared','Zulu','<svg onload=x>','__proto__','Alpha','alpha']);assert.match(f.el('mods-coverage').textContent,/3 of 3.*Complete coverage/);
 const shared=button(f,'shared');assert.equal(shared.type,'button');assert.equal(shared.lastChild.textContent,'3 players');assert.equal(shared.attributes['aria-expanded'],'false');shared.onclick();assert.equal(shared.attributes['aria-expanded'],'true');
 const users=shared.parentNode.lastChild;assert.equal(users.hidden,false);assert.deepEqual(users.children.map(n=>n.dataset.playerId),['a','b','c']);assert.deepEqual(users.children.map(n=>n.textContent),[a.principal.displayName,b.principal.displayName,c.principal.displayName]);assert.equal(f.el('mods-rows').querySelectorAll('img').length,0);assert.equal(f.el('mods-rows').querySelectorAll('svg').length,0);
 assert.ok(f.el('player-seed-details').children.length===3,'per-player details remain');
 const refreshed={...base,room:{...base.room,revision:2},inspections:[report(a,['shared','new']),report(b,['shared'])]};await f.render(refreshed);assert.equal(button(f,'shared').attributes['aria-expanded'],'true');assert.equal(button(f,'shared').lastChild.textContent,'2 players');assert.match(f.el('mods-coverage').textContent,/2 of 3.*Incomplete coverage/);assert.deepEqual(rowNames(f),['shared','new']);
 await f.render({...refreshed,inspections:[report(a,['shared'],{inventoryComplete:false}),report(b,['shared'],{collectorFailure:'collector failed'}),report({desktopId:'not-in-roster'},['foreign'])]});assert.equal(f.el('mods-rows').children.length,0);assert.match(f.el('mods-coverage').textContent,/0 of 3.*Incomplete/);assert.match(f.el('mods-empty').textContent,/Awaiting/);
 await f.render(base);assert.equal(button(f,'shared').attributes['aria-expanded'],'false','removed rows lose expansion');button(f,'shared').onclick();
 await f.render({...base,room:{...base.room,roomId:'other'},inspections:[]});assert.equal(f.el('mods-rows').children.length,0);await f.render(base);assert.equal(button(f,'shared').attributes['aria-expanded'],'false','lobby switch clears expansion');button(f,'shared').onclick();
 await f.render({...base,session_epoch:'replacement-session'});assert.equal(button(f,'shared').attributes['aria-expanded'],'false','session switch clears expansion');
 await f.render({...base,room:{...base.room,members:[]},inspections:[]});assert.equal(f.el('mods-rows').children.length,0);assert.match(f.el('mods-empty').textContent,/No active mods/);
 await f.render({...base,signed_in:false,room:null,inspections:[]});assert.equal(f.el('mods-rows').children.length,0);assert.equal(f.el('mods-coverage').textContent,'');
}
{
 const name='🌊中文'.repeat(160)+'<script>alert(1)</script>',player=member('long',name),view={...base,room:{...base.room,members:[player]},inspections:[report(player,[name])]};const f=await fixture(view);assert.deepEqual(rowNames(f),[name]);button(f,name).onclick();assert.equal(button(f,name).parentNode.lastChild.firstChild.textContent,name);assert.equal(f.el('mods-rows').querySelectorAll('script').length,0);
}
{
 const removed={...base,room:{...base.room,state:'running',members:[b,c]},rejoin:{phase:'Ended',terminal_reason:'kick_on_death'}};const f=await fixture(removed);assert.equal(f.el('lobby-screen').hidden,false);assert.equal(f.el('end').disabled,false);assert.equal(f.el('nav-current').hidden,false);assert.match(f.el('settings-permission').textContent,/You host this lobby/);assert.match(f.el('primary-instruction').textContent,/not participating.*controls remain/);assert.match(f.el('death-fields').textContent,/Kick on death/);assert.equal(f.calls.some(c=>['join','leave_lobby'].includes(c.input?.action)),false);
 await f.render({...removed,account_id:b.accountId,desktop_id:b.desktopId,principal:b.principal});assert.equal(f.el('end').disabled,true);assert.match(f.el('settings-permission').textContent,/read-only/);
}
console.log('PASS legacy-derived Mods sorting/distinct identities/dedup/coverage/text safety/refresh/expansion/scope cleanup and nonparticipating host controls/Kick on death label');
