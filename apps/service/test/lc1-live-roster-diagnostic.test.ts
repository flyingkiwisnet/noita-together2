import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {LiveService} from '../src/g3/live-service.js';import {traceLiveRoster} from '../scripts/lc1-live-roster-diagnostic.js';
test('private first-empty evidence observes actual drain-time pose expiry without changing frame behavior or exposing scopes',async()=>{
 let now=0;const self=randomUUID(),peer=randomUUID(),roomId=randomUUID(),scope={sessionId:self,roomId,worldToken:'PRIVATE_SCOPE'},peerScope={sessionId:peer,roomId};
 const room={members:new Map([[self,{scope,generation:1,lastPose:1}],[peer,{scope:peerScope,generation:1,lastPose:3}]]),terminal:false,normalEnd:false};let changes=0;const failure=Object.assign(Error('lock'),{code:'55P03'});
 const live:any=Object.create(LiveService.prototype);live.db={serviceId:'test',epoch:randomUUID()};live.auth={now:()=>now};live.metrics={receivedPoses:3};live.samples=new Map();live.projection={rooms:new Map([[roomId,room]]),read:async()=>{throw failure;},onChange:()=>changes++};
 const c={binding:{scope,connectionId:randomUUID(),authorityGeneration:1,authorityUntil:30000,accessUntil:900000},serverSequence:0,retired:false,busy:false},original=live.frame,read=live.projection.read,change=live.projection.onChange,trace=traceLiveRoster(live,()=>now);
 live.frame(c,{snapshot:{peers:[]}});assert.equal(trace.status().firstEmptySnapshot,null,'initial empty is not a loss');
 live.samples.set(peer,{acceptedAt:0,poses:[{sequence:3}],poseTimes:[0],generation:1});const payload={snapshot:{peers:[{sessionId:peer,poses:[{sequence:3}]}]}};assert.equal(live.frame(c,payload).snapshot.peers.length,1);
 now=2000;assert.equal(live.frame(c,payload).snapshot.peers.length,0);const loss=trace.status().firstEmptySnapshot;assert.equal(loss.sourcePeers,1);assert.equal(loss.wirePeers,0);assert.equal(loss.samples[0].ageMs,2000);assert.equal(loss.samples[0].present,true);assert.equal(loss.eligiblePeers,1);
 await assert.rejects(live.projection.read({},roomId),e=>e===failure);assert.equal(trace.status().firstProjectionReadFailure.code,'55P03');live.projection.onChange(room);room.members.delete(peer);live.projection.onChange(room);assert.equal(changes,2);assert.equal(trace.status().firstProjectionShrink.previousMembers,2);
 assert.equal(JSON.stringify(trace.status()).includes('PRIVATE_SCOPE'),false);assert.equal(JSON.stringify(trace.status()).includes(peer),false);trace.close();assert.equal(live.frame,original);assert.equal(live.projection.read,read);assert.equal(live.projection.onChange,change);
});
