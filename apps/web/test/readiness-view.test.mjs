import test from 'node:test';import assert from 'node:assert/strict';
import {readinessPresentation,createReadinessCell} from '../candidate/readiness-view.js';
import {controllerDom} from './dom-fixture.mjs';
const now=Date.parse('2026-11-05T00:00:00Z'),info={serviceId:'service',serviceEpoch:'epoch',compatibility:{modVersion:'nt2-current',bridgeVersion:'runtime-current'}};
const member={desktopId:'desktop-private',present:true,attached:true,loaded:true,ready:true,readiness:'ready'},room={roomId:'room',runId:'run',runEpoch:'run-epoch',configRevision:1,state:'staging',config:{seed:123,seedPolicy:'shared-fixed'}};
const report={desktopId:member.desktopId,attachmentId:'attachment',worldToken:'world',gameInstanceId:'game',observationRevision:1,capturedAt:new Date(now-1000).toISOString(),presenceUntil:new Date(now+30000).toISOString(),validUntil:new Date(now+600000).toISOString(),compatibilityProvenance:'game-thread-client-observation',modVersion:'nt2-current',bridgeVersion:'runtime-current',seedStatus:'known',actualSeed:'123',activeMods:['noita_together2','optional_mod']};
const present=(patch={},previous)=>readinessPresentation({member,report,room,info,now,...patch},previous);
test('Current observation verifies versions and resolved shared seed; ready alone never does',()=>{
 const p=present();assert.equal(p.cells.app.kind,'ok');assert.equal(p.cells.noita.kind,'ok');assert.equal(p.cells.versions.kind,'ok');assert.equal(p.cells.seed.kind,'ok');assert.equal(p.cells.status.label,'Waiting for host');
 for(const r of [undefined,{...report,modVersion:undefined},{...report,compatibilityProvenance:'unknown'},{...report,desktopId:'other'},{...report,capturedAt:undefined},{...report,presenceUntil:undefined}])assert.equal(present({report:r}).cells.versions.kind,'unknown');
 assert.equal(present({member:{...member,attached:false}}).cells.versions.kind,'unknown');assert.equal(present({report:{...report,activeMods:['other_optional_mod']}}).cells.versions.kind,'ok');
 assert.equal(present({report:{...report,actualSeed:'124'}}).cells.seed.kind,'error');assert.equal(present({report:{...report,actualSeed:'124'},room:{...room,config:{...room.config,seedPolicy:'individual-random'}}}).cells.seed.label,'N/A');
});
test('Confirmed version failures survive missing/stale updates and clear only on matching validation or context turnover',()=>{
 const bad=present({report:{...report,modVersion:'different'}});assert.equal(bad.cells.versions.kind,'error');assert.match(bad.cells.versions.detail,/Running NT2 mod: different; required: nt2-current/);assert.match(bad.cells.status.detail,/Update the app and NT2 mod/);
 const missing=present({report:undefined,member:{...member,attached:false,readiness:'reconnecting'}},bad);assert.equal(missing.cells.versions.kind,'error');assert.equal(missing.cells.status.label,'Versions differ');
 assert.equal(present({report:{...report,presenceUntil:new Date(now-1).toISOString()}},bad).cells.versions.kind,'error');assert.equal(present({},bad).cells.versions.kind,'ok');
 for(const changed of [{room:{...room,runId:'next'}},{room:{...room,configRevision:2}},{info:{...info,serviceEpoch:'next'}},{report:{...report,worldToken:'next',modVersion:undefined}}])assert.equal(present({...changed,report:changed.report},bad).cells.versions.kind,'unknown');
 assert.equal(present({report:undefined,member:{...member,readinessReason:'observed_version_mismatch'}}).cells.versions.kind,'error');
});
test('Game loading, stale updates, explicit pause, reconnect and ended/excluded remain distinct',()=>{
 const missing=present({report:undefined,member:{...member,attached:false,loaded:false,ready:false,readiness:'waiting_noita'}}).cells.noita;assert.equal(missing.label,'Not observed');assert.match(missing.detail,/then start Noita and choose New Game/);assert.doesNotMatch(missing.detail,/New Game in the app/);
 assert.equal(present({member:{...member,loaded:false,ready:false,readiness:'loading_world'}}).cells.noita.label,'Loading');
 const stale=present({report:{...report,presenceUntil:new Date(now-1).toISOString()},member:{...member,readiness:'playing'}});assert.equal(stale.cells.noita.label,'Stale updates');assert.equal(stale.cells.status.label,'Stale updates');assert.equal(stale.cells.versions.kind,'unknown');
 const paused=present({member:{...member,readiness:'paused'}});assert.equal(paused.cells.app.label,'Connected');assert.equal(paused.cells.status.label,'Paused');
 assert.equal(present({member:{...member,present:false,attached:false,ready:false,readiness:'reconnecting'}}).cells.status.label,'Reconnecting');assert.equal(present({included:false}).cells.status.label,'Excluded');assert.equal(present({room:{...room,state:'ended'}}).cells.status.label,'Ended');
 assert.equal(present({room:{...room,state:'running'},member:{...member,readiness:'playing'},report:{...report,validUntil:new Date(now-1).toISOString()}}).cells.status.label,'Playing');
});
test('Status details support hover, keyboard focus/Escape and tap without unsafe markup or multiple overlays',async()=>{
 const d=await controllerDom('../candidate/index.html'),a=createReadinessCell(d.document,'Versions','detail-a'),b=createReadinessCell(d.document,'Seed','detail-b');a.update({kind:'error',label:'Mismatch',detail:'<script>unsafe</script>'});b.update({kind:'ok',label:'Matched',detail:'Shared seed matches'});
 assert.equal(a.button.getAttribute('aria-label'),'Versions: Mismatch. Details');assert.equal(a.button.getAttribute('aria-describedby'),'detail-a');assert.equal(a.help.hidden,true);await a.button.fire('mouseenter');assert.equal(a.help.hidden,false);assert.match(a.help.textContent,/<script>unsafe<\/script>/);assert.equal(a.help.children.length,0);await a.button.fire('mouseleave');assert.equal(a.help.hidden,true);
 await a.button.fire('focus');assert.equal(a.help.hidden,false);await a.button.fire('keydown',{key:'Escape'});assert.equal(a.help.hidden,true);await a.button.fire('click');assert.equal(a.button.getAttribute('aria-expanded'),'true');assert.equal(a.help.hidden,false);await b.button.fire('click');assert.equal(a.help.hidden,true);assert.equal(b.help.hidden,false);await b.button.fire('blur');assert.equal(b.help.hidden,true);
});
