import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {readFile} from 'node:fs/promises';
import {HourglassRegistry,HourglassLedger,planHourglass,HOURGLASS_CATALOGUE,type HourglassActor,type HourglassState} from '../../../packages/domain/src/g3-hourglass.js';
function fixture(){
 const shared={roomId:randomUUID(),runId:randomUUID(),runEpoch:randomUUID(),configRevision:2,mode:'core',live:true,capabilities:['core.hourglass'],catalogueRevision:HOURGLASS_CATALOGUE.revision};
 const actor=(name:string):HourglassActor=>({...shared,principalId:randomUUID(),gameSessionId:randomUUID(),attachmentId:randomUUID(),worldToken:'世界-'+randomUUID(),name});
 const alex=actor('Alex'),sam=actor('<img src=x onerror=alert(1)>'),bob=actor('Bob');return {alex,sam,bob,roster:[alex,sam,bob]};
}
const activation=(index=0)=>({activationId:randomUUID(),effectId:HOURGLASS_CATALOGUE.entries[index]!.id,materialId:HOURGLASS_CATALOGUE.entries[index]!.materialId});
test('all seven audited hourglass mappings retain source attribution, durations and self-filtered core recipient plans',async()=>{
 const audit=JSON.parse(await readFile(new URL('../../../docs/rewrite/legacy-player-experience.json',import.meta.url),'utf8'));
 const catalogue=Object.values(audit).find((v:any)=>Array.isArray(v?.hourglass_effects)) as any;
 assert.equal(HOURGLASS_CATALOGUE.entries.length,7);
 const {alex,sam,bob,roster}=fixture();
 for(const [i,entry] of HOURGLASS_CATALOGUE.entries.entries()){
  const source=catalogue.hourglass_effects.find((v:any)=>v.id===entry.id);assert.equal(entry.materialId,source.material_id);assert.equal(entry.durationFrames,source.requested_duration_frames);
  const p=activation(i),plan=planHourglass(alex,p,roster);assert.deepEqual(plan.recipients.map(r=>r.gameSessionId),[sam.gameSessionId,bob.gameSessionId]);assert.equal(plan.source.name,'Alex');assert.equal(plan.provenance,'validated-client-report');
  assert.throws(()=>planHourglass(alex,{...p,materialId:'water'},roster),/FORBIDDEN/);
 }
 // Display names remain literal data; they are never accepted from activation input.
 assert.equal(planHourglass(sam,activation(),roster).source.name,sam.name);
});
test('hourglass authorization rejects invalid IDs, injected targeting/code/settings, unsupported mode/caps and stale lifecycle/catalogue',()=>{
 const {alex,sam,bob,roster}=fixture(),p=activation();
 for(const invalid of [{...p,effectId:'unreviewed.third_party'},{...p,callback:'evil()'},{...p,entityPath:'mods/evil.xml'},{...p,targetPrincipalId:bob.principalId},{...p,settings:{hourglass:true}},{...p,activationId:p.activationId.toUpperCase()},{...p,activationId:'bad'}])assert.throws(()=>planHourglass(alex,invalid,roster));
 for(const sender of [{...alex,mode:'nemesis'},{...alex,live:false},{...alex,capabilities:[]},{...alex,catalogueRevision:'stale'},{...alex,name:'x'.repeat(81)},{...alex,name:'evil\nname'},{...alex,worldToken:'x'.repeat(129)}])assert.throws(()=>planHourglass(sender,p,roster),/FORBIDDEN/);
 for(const peer of [{...sam,roomId:randomUUID()},{...sam,runEpoch:randomUUID()},{...sam,configRevision:3},{...sam,runId:randomUUID()},{...sam,live:false},{...sam,mode:'nemesis'},{...sam,capabilities:[]},{...sam,catalogueRevision:'stale'}])assert.equal(planHourglass(alex,p,[alex,peer]).recipients.length,0);
 assert.throws(()=>planHourglass(alex,p,[sam,sam]),/CONFLICT/);assert.throws(()=>planHourglass(alex,p,Array(33).fill(bob)),/CAPACITY/);
 assert.equal(planHourglass(alex,p,[alex]).recipients.length,0);
});
test('lost activation/receipt responses retain originals, private frozen recipients and uncertainty across state serialization',()=>{
 const {alex,sam,bob,roster}=fixture(),state:HourglassState={records:[]},ledger=new HourglassLedger(state),p=activation();
 const original=ledger.activate(alex,p,roster);assert.equal(original.deliveries.length,2);
 const retry=ledger.activate(alex,p,[alex]);assert.deepEqual(retry,original);assert.equal(state.records.length,1);
 assert.throws(()=>ledger.activate(alex,{...p,...activation(1),activationId:p.activationId},roster),/CONFLICT/);
 assert.equal(ledger.pending(alex).length,0);const pending=ledger.pending(sam);assert.equal(pending.length,1);assert.equal(pending[0]!.plan.recipients.length,1);assert.equal(pending[0]!.plan.recipients[0]!.principalId,sam.principalId);
 pending[0]!.delivery.outcome='applied_reported';assert.equal(ledger.pending(sam)[0]!.delivery.outcome,'pending');
 for(const stale of [{...sam,worldToken:'replacement'},{...sam,attachmentId:randomUUID()},{...sam,gameSessionId:randomUUID()}])assert.equal(ledger.pending(stale).length,0);
 const delivery=original.deliveries[0]!,receipt=randomUUID();assert.throws(()=>ledger.report(bob,delivery.deliveryId,receipt,'applied_reported'),/NOT_FOUND/);
 const report=ledger.report(sam,delivery.deliveryId,receipt,'unknown');assert.equal(report.outcome,'unknown');assert.deepEqual(ledger.report(sam,delivery.deliveryId,receipt,'unknown'),report);
 assert.throws(()=>ledger.report(sam,delivery.deliveryId,randomUUID(),'applied_reported'),/CONFLICT/);assert.equal(ledger.pending(sam).length,0);
 const restored=new HourglassLedger(JSON.parse(JSON.stringify(state)));assert.deepEqual(restored.status({...sam,live:false},p.activationId),ledger.status(sam,p.activationId));assert.equal(restored.pending(sam).length,0);assert.equal(restored.pending(bob).length,1);
 assert.equal(restored.status(sam,p.activationId).deliveries.length,1);assert.throws(()=>restored.status({...sam,principalId:randomUUID()},p.activationId),/NOT_FOUND/);
 assert.throws(()=>restored.activate({...alex,live:false},p,roster),/FORBIDDEN/);assert.throws(()=>restored.pending({...bob,live:false}),/FORBIDDEN/);assert.throws(()=>restored.report({...bob,live:false},original.deliveries[1]!.deliveryId,randomUUID(),'applied_reported'),/FORBIDDEN/);
 assert.equal(restored.status({...alex,live:false},p.activationId).deliveries[0]!.outcome,'unknown');
});
test('trusted local extensions require version changes and explicit replace/disable; old deliveries cannot apply under a new catalogue',()=>{
 const registry=new HourglassRegistry(),entry={id:'trusted.boon',materialId:'water',localHandlerId:'trusted.boon',durationFrames:null,effect:'Local approved boon'};
 registry.registerTrusted({revision:'trusted-hourglass-2',entries:[entry],replace:false,disabledIds:[]});const {alex,sam,roster}=fixture();
 assert.throws(()=>planHourglass(alex,activation(),roster,registry),/FORBIDDEN/);
 const actors=roster.map(a=>({...a,catalogueRevision:registry.revision}));assert.equal(planHourglass(actors[0]!,{activationId:randomUUID(),effectId:entry.id,materialId:entry.materialId},actors,registry).recipients.length,2);
 assert.throws(()=>registry.registerTrusted({revision:'trusted-hourglass-3',entries:[entry],replace:false,disabledIds:[]}),/CONFLICT/);assert.equal(registry.revision,'trusted-hourglass-2');
 registry.registerTrusted({revision:'trusted-hourglass-3',entries:[],replace:true,disabledIds:[entry.id]});assert.equal(registry.get(entry.id),undefined);
 const state:HourglassState={records:[]},base=new HourglassLedger(state);const record=base.activate(alex,activation(),roster);const next=new HourglassLedger(state,registry),newSam={...sam,catalogueRevision:registry.revision};
 assert.equal(next.pending(newSam).length,0);assert.throws(()=>next.report(newSam,record.deliveries[0]!.deliveryId,randomUUID(),'applied_reported'),/NOT_FOUND/);
});
test('bounded activation state refuses new work without evicting pending/unknown originals',()=>{
 const {alex,roster}=fixture(),state:HourglassState={records:[]},ledger=new HourglassLedger(state);const p=activation();ledger.activate(alex,p,roster);
 for(let i=1;i<256;i++)ledger.activate(alex,activation(i%7),roster);assert.equal(state.records.length,256);
 assert.throws(()=>ledger.activate(alex,activation(),roster),/CAPACITY/);assert.equal(ledger.activate(alex,p,[]).plan.activationId,p.activationId);assert.equal(state.records.length,256);
});
test('private pending delivery pages are bounded and retain the captured high through duplicates/new activations',()=>{
 const {alex,sam,roster}=fixture(),state:HourglassState={records:[]},ledger=new HourglassLedger(state);
 for(let i=0;i<33;i++)ledger.activate(alex,activation(i%7),roster);
 const first=ledger.pendingPage(sam);assert.equal(first.deliveries.length,32);assert.equal(first.highSequence,33);assert.equal(first.nextSequence,32);
 ledger.activate(alex,activation(),roster);const next=ledger.pendingPage(sam,first.nextSequence,first.highSequence);assert.equal(next.deliveries.length,1);assert.equal(next.deliveries[0]!.sequence,33);assert.equal(next.nextSequence,33);
 assert.deepEqual(ledger.pendingPage(sam,32,33),next);assert.equal(ledger.pendingPage(sam,33).deliveries[0]!.sequence,34);
 assert.throws(()=>ledger.pendingPage(sam,0,35),/INVALID_MESSAGE/);assert.throws(()=>ledger.pendingPage(sam,34,33),/INVALID_MESSAGE/);
 const ids=[...first.deliveries,...next.deliveries].map(v=>v.delivery.deliveryId);assert.equal(new Set(ids).size,33);
});
test('R09 binds one per-run original activation to immutable sender principal/session/attachment/world/scope',()=>{
 const {alex,sam,roster}=fixture(),state:HourglassState={records:[]},ledger=new HourglassLedger(state),p=activation();const original=ledger.activate(alex,p,roster);
 for(const changed of [sam,{...alex,gameSessionId:randomUUID()},{...alex,attachmentId:randomUUID()},{...alex,worldToken:'replacement'},{...alex,runEpoch:randomUUID()},{...alex,configRevision:3},{...alex,roomId:randomUUID()}])assert.throws(()=>ledger.activate(changed,p,roster),/CONFLICT/);
 assert.deepEqual(ledger.activate(alex,p,[]),original);assert.equal(state.records.length,1);assert.equal(ledger.status(sam,p.activationId).plan.source.principalId,alex.principalId);
 assert.throws(()=>ledger.activate(alex,{...p,effectId:'berserk',materialId:'magic_liquid_berserk'},roster),/CONFLICT/);
});
