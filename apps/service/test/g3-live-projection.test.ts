import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {LiveProjection} from '../src/g3/live-projection.js';import {afterCommit,publishCommitEffects,discardCommitEffects} from '../src/g1/commit-hooks.js';
import type {Database,Tx} from '../src/g1/database.js';import type {CandidateRooms} from '../src/g2-next/rooms.js';
import {PolicyError} from '../src/g1/security.js';
test('full 128-room watched bound leaves transaction capacity for token effects and rollback publishes nothing',async()=>{
 const db={epoch:randomUUID()} as Database,authority={} as CandidateRooms,projection=new LiveProjection(db,authority,()=>0);
 // Absent canonical rooms project terminal; no caller-controlled pose authority.
 const tx={query:async()=>({rows:[]})} as unknown as Tx;let changed=0,tokens=0;projection.onChange=()=>{changed++;};
 for(let i=0;i<128;i++)projection.watch(randomUUID());assert.throws(()=>projection.watch(randomUUID()),/CAPACITY/);
 await projection.prepare(tx);afterCommit(tx,()=>{tokens++;},()=>{});afterCommit(tx,()=>{tokens++;},()=>{});discardCommitEffects(tx);
 assert.equal(changed,0);assert.equal(tokens,0);assert.equal(projection.rooms.size,128);
 await projection.prepare(tx);afterCommit(tx,()=>{tokens++;},()=>{});afterCommit(tx,()=>{tokens++;},()=>{});publishCommitEffects(tx);
 assert.equal(changed,128);assert.equal(tokens,2);assert.equal(projection.rooms.size,0);
});

test('failed projection reads reject preparation, never fabricate a committed terminal room or renew a prior lease',async()=>{
 let now=10;const projection=new LiveProjection({epoch:randomUUID()} as Database,{} as CandidateRooms,()=>now),room=projection.watch(randomUUID()),session=randomUUID(),member={scope:{sessionId:session},digest:'a'.repeat(64),generation:1,until:100,parentId:'parent',familyId:'family',lastPose:2};room.members.set(session,member);let changes=0;projection.onChange=()=>changes++;
 for(const error of [new PolicyError('CAPACITY'),Object.assign(Error('lock'),{code:'55P03'})]){const tx={query:async()=>{throw error;}} as unknown as Tx;await assert.rejects(projection.prepare(tx),e=>e===error);publishCommitEffects(tx);assert.equal(changes,0);assert.equal(room.terminal,false);assert.equal(projection.member(room.id,session),member);assert.equal(member.until,100);}
 now=100;assert.equal(projection.member(room.id,session),undefined,'failed reads never extend the original committed expiry');
 const tx={query:async()=>({rows:[]})} as unknown as Tx;await projection.prepare(tx);publishCommitEffects(tx);assert.equal(changes,1);assert.equal(room.terminal,true,'successful canonical terminal read still retires');
});
