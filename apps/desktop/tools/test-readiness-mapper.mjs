// Production mapper; synthetic canonical facts. No browser, app or game launch.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../ui/ux1-players.js',import.meta.url),'utf8');
const context=vm.createContext({});vm.runInContext(source.slice(0,source.indexOf('function uxStatusCell')),context);
const now=Date.parse('2026-11-05T00:00:00Z');let checks=0;
const check=(actual,expected,message)=>{assert.equal(actual,expected,message);checks++;};
const base={server_origin:'https://fixture.invalid',service_id:'s',service_epoch:'e',room:{roomId:'r',runId:'run',runEpoch:'epoch',configRevision:1,state:'staging',config:{seedPolicy:'fixed',seed:'42'}},ux1:{compatibility:{modVersion:'mod',bridgeVersion:'bridge'}},inspections:[{desktopId:'d',attachmentId:'a',worldToken:'w',gameInstanceId:'g',observationRevision:1,capturedAt:'2026-11-04T23:59:59Z',presenceUntil:'2026-11-05T00:01:00Z',validUntil:'2026-11-05T00:01:00Z',modVersion:'mod',bridgeVersion:'bridge',compatibilityProvenance:'game-thread-client-observation',loaded:true,seedStatus:'known',actualSeed:'42'}]};
const member={desktopId:'d',present:true,attached:true,loaded:true,readiness:'ready'};
const run=(v=structuredClone(base),m=structuredClone(member))=>{context.v=v;context.m=m;context.now=now;return vm.runInContext('uxReadinessCells(v,m,now)',context);};
for(const i of [1,2,3])check(run()[i].state,'ok','matching complete observation');
for(const key of ['attachmentId','gameInstanceId','worldToken','capturedAt']){const v=structuredClone(base);delete v.inspections[0][key];for(const i of [1,2,3])check(run(v)[i].state==='ok',false,'missing '+key+' must not verify cell '+i);}
for(const revision of ['1',1.5,0]){const v=structuredClone(base);v.inspections[0].observationRevision=revision;check(run(v)[2].state,'unknown','integer positive revision required');}
for(const i of [1,2,3])check(run(base,{...member,attached:false})[i].state==='ok',false,'detached member must not verify');
const otherDesktop=structuredClone(base);otherDesktop.inspections[0].desktopId='other';check(run(otherDesktop)[2].state,'unknown','foreign observation');
const badProvenance=structuredClone(base);badProvenance.inspections[0].compatibilityProvenance='assumed';check(run(badProvenance)[2].state,'unknown','game-thread provenance required');
const mismatch=structuredClone(base);mismatch.inspections[0].modVersion='wrong';check(run(mismatch)[4].label,'Versions differ');
const missing=structuredClone(base);missing.inspections=[];check(run(missing)[2].state,'error','missing report preserves confirmed warning');
const partial=structuredClone(base);delete partial.inspections[0].attachmentId;check(run(partial)[2].state,'error','incomplete report cannot clear warning');
check(run(base,{...member,attached:false})[2].state,'error','detached matching report cannot clear warning');
check(run()[2].state,'ok','actual matching validation clears');
for(const key of ['attachmentId','worldToken','gameInstanceId']){run(mismatch);const changed=structuredClone(partial);changed.inspections[0][key]='new-'+key;check(run(changed)[2].state==='error',false,key+' turnover clears old warning');run();}
run(mismatch);const changedContext=structuredClone(missing);changedContext.room.configRevision++;check(run(changedContext)[2].state,'unknown','configuration turnover clears');run();
const stale=structuredClone(base);stale.inspections[0].presenceUntil='2026-11-04T23:59:00Z';check(run(stale)[2].state,'unknown');check(run(stale)[1].label,'Stale updates');
const running=structuredClone(base);running.room.state='running';running.inspections[0].validUntil='2026-11-04T23:59:00Z';check(run(running)[2].state,'ok','playing presence independent of staging review');
const wrongSeed=structuredClone(base);wrongSeed.inspections[0].actualSeed='99';check(run(wrongSeed)[4].label,'Seed differs');check(run(wrongSeed)[4].state,'error');
const individual=structuredClone(wrongSeed);individual.room.config.seedPolicy='individual-random';check(run(individual)[3].state,'unknown');
for(const [stage,label,state] of [['loading_world','Loading','warn'],['checking_report','Checking','warn'],['configuration_changed','Review changes','error'],['restart_required','Restart required','error'],['expired','Restart required','error'],['cancelled','Restart required','error'],['offline','Reconnecting','warn'],['ended','Ended','unknown'],['dnf','DNF','unknown'],['dead','Died','unknown'],['excluded','Excluded','unknown'],['paused','Paused / stale updates','warn'],['playing','Playing','ok']]){const cells=run(base,{...member,readiness:stage});check(cells[4].label,label,stage);check(cells[4].state,state,stage);}
check(run(base,{...member,readiness:'checking_report',readinessReason:'collector_failed'})[4].label,'Inspection failed');
check(run(base,{...member,readiness:'checking_report',readinessReason:'seed_mismatch'})[4].label,'Seed differs');
console.log(JSON.stringify({passed:true,assertions:checks,productionMapper:true,syntheticCanonicalFacts:true}));
