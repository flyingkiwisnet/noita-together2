import test from 'node:test';import assert from 'node:assert/strict';
import {decideWorldAdmission} from '../../../packages/domain/src/world-admission.js';
import {LiveProjection} from '../src/g3/live-projection.js';
import {publishCommitEffects} from '../src/g1/commit-hooks.js';
import {RejoinRooms} from '../src/rejoin/rooms.js';

test('CR1 admission separates first-entry review, transport and report eligibility',()=>{
 const f={identityValid:true,ownerActive:true,reportValid:true,protected:true,entered:false,generation:1,now:1000,transportUntil:31000,reviewUntil:5000};
 assert.equal(decideWorldAdmission(f).remainingMs,4000);
 assert.equal(decideWorldAdmission({...f,now:5000}).reason,'initial_review_expired');
 assert.equal(decideWorldAdmission({...f,now:5000,generation:2}).authorized,true);
 assert.equal(decideWorldAdmission({...f,reportValid:false}).authorized,false);
 assert.equal(decideWorldAdmission({...f,ownerActive:false}).authorized,false);
 assert.equal(decideWorldAdmission({...f,transportUntil:1000}).authorized,false);
 assert.equal(decideWorldAdmission({...f,terminalReason:'explicit_offline'}).terminal,true);
 assert.equal(decideWorldAdmission({...f,protected:false,entered:false}).authorized,false);
 assert.equal(decideWorldAdmission({...f,protected:false,entered:true,now:6000}).playing,true);
});

test('CR1 stale first-arrival preparation cancel cannot retire generation2',async()=>{
 const rooms=new RejoinRooms({epoch:'e'} as any,{now:()=>1} as any,{} as any);let retired=false;
 rooms.recovery.retire=async()=>{retired=true;};
 await assert.rejects(rooms.inspection.cancelPreparation({query:async()=>({rows:[{id:'p',generation:2,preparation_id:null}]})} as any,{kind:'desktop',sessionId:'d',sessionEpoch:'s'} as any,{serviceEpoch:'e',sessionEpoch:'s',runId:'run',preparationCancel:{preparationId:'old'}}),/STALE_EPOCH/);
 assert.equal(retired,false);
 rooms.recovery.read=async()=>({room_id:'room',run_epoch:'run-epoch',config_revision:1,current_run_id:'run',run_state:'running',terminal:false});
 await rooms.inspection.cancelPreparation({query:async()=>({rows:[{id:'p',generation:1,preparation_id:'current'}]})} as any,{kind:'desktop',sessionId:'d',sessionEpoch:'s'} as any,{serviceEpoch:'e',sessionEpoch:'s',roomId:'room',runId:'run',runEpoch:'run-epoch',configRevision:1,preparationCancel:{preparationId:'current'}});assert.equal(retired,true);
});

test('CR1 full current roster tolerates50 owner replacements, retains held-owner fence and isolates room failure',async()=>{
 const projection=new LiveProjection({} as any,{} as any,()=>1000);let generation=0,broken=false;
 const member=(id:string)=>({scope:{sessionId:id},digest:id,until:100000,parentId:'d',familyId:'f'});
 (projection as any).read=async(_tx:any,id:string)=>{if(broken&&id==='full')throw Error('scoped projection failure');const members=id==='full'?Array.from({length:32},(_,i)=>member(i===31?'returned-'+generation:'s'+i)):[member('healthy')];return {terminal:false,members,ownerSessionIds:members.map(m=>m.scope.sessionId)};};
 projection.watch('full');projection.watch('other');
 for(generation=0;generation<50;generation++){const tx={} as any;await projection.prepare(tx);publishCommitEffects(tx);assert.equal(projection.rooms.size,2);assert.equal(projection.rooms.get('full')!.poseFences.size,32);assert.ok(projection.member('other','healthy'));}
 const held=projection.rooms.get('full')!.poseFences.get('s0')!;held.lastPose=77;
 const oldRead=(projection as any).read;(projection as any).read=async(tx:any,id:string)=>{const r=await oldRead(tx,id);if(id==='full')r.members=r.members.filter((m:any)=>m.scope.sessionId!=='s0');return r;};
 let heldTx={} as any;await projection.prepare(heldTx);publishCommitEffects(heldTx);assert.equal(projection.rooms.get('full')!.poseFences.get('s0')!.lastPose,77);
 (projection as any).read=oldRead;heldTx={} as any;await projection.prepare(heldTx);publishCommitEffects(heldTx);assert.equal(projection.member('full','s0')!.lastPose,77);
 broken=true;const tx={} as any;await projection.prepare(tx);publishCommitEffects(tx);assert.equal(projection.rooms.has('full'),false);assert.ok(projection.member('other','healthy'));
 projection.invalidateAll();assert.equal(projection.rooms.size,0);
});
