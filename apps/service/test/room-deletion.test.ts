import test from 'node:test';import assert from 'node:assert/strict';
import {roomDeletionFailure} from '../../../packages/domain/src/g1.js';
import {encode,decode} from '../../../packages/contracts/src/g2-next.js';
test('only a registered browser host may delete an inactive unprepared lobby',()=>{
 const v={kind:'browser',registered:true,host:true,state:'waiting',prepared:false,attached:false};
 for(const state of ['waiting','ended'])assert.equal(roomDeletionFailure({...v,state}),undefined);
 for(const state of ['staging','preparing','loading','running','suspended'])assert.equal(roomDeletionFailure({...v,state}),'INVALID_STATE');
 for(const change of [{kind:'game'},{kind:'desktop'},{registered:false},{host:false}])assert.equal(roomDeletionFailure({...v,...change}),'FORBIDDEN');
 for(const change of [{prepared:true},{attached:true}])assert.equal(roomDeletionFailure({...v,...change}),'INVALID_STATE');
 assert.deepEqual(decode('Request',encode('Request',{roomDelete:{}})),{roomDelete:{}});
 assert.deepEqual(decode('Response',encode('Response',{deletedRoomId:'room'})),{deletedRoomId:'room'});
});
