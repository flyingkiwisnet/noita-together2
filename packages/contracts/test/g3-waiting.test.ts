import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {encode,decode,validate,parseJson} from '../src/g3-waiting.js';
import {schema as waitingSchema} from '../src/g3-waiting.js';
import {schema as admissionSchema} from '../src/g3-connection.js';
import protobuf from 'protobufjs';
const golden=JSON.parse(readFileSync(new URL('../g3-waiting/golden.json',import.meta.url),'utf8'));
test('Waiting candidate strict JSON/protobuf preserves no-play state separately from admitted Start',()=>{
 for(const f of golden.fixtures){assert.deepEqual(decode(f.type,encode(f.type,f.json)),f.json);assert.deepEqual(parseJson(f.type,JSON.stringify(f.json)),f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);}
 const w=golden.fixtures[0].json.waiting;assert.equal(w.mayPlay,false);assert.equal(w.intent.originalStartId,undefined);
 assert.throws(()=>validate('WaitingIntent',{...w.intent,originalStartId:''}));assert.throws(()=>validate('WaitingState',{...w,ownerRemainingMs:30001}));assert.throws(()=>validate('Recover',{waitingProof:'x'.repeat(44)}));
 assert.throws(()=>parseJson('Request','{"status":{},"status":{}}'));
 for(const [name,definition]of Object.entries(admissionSchema.messages))if(name.startsWith('Connection'))assert.deepEqual(waitingSchema.messages[name],definition,'one existing canonical admission shape');
});
test('Entry vectors are provisional identity only and require explicit offline; strict false play holds in JSON/protobuf',()=>{
 for(const rejected of golden.rejected){assert.throws(()=>validate(rejected.type,rejected.json));assert.throws(()=>parseJson(rejected.type,JSON.stringify(rejected.json)));}
 // Generator golden output contains only type/json/hex; inspect canonical types.
 const preview=golden.fixtures.find((f:any)=>f.type==='EntryPreview')?.json;assert.ok(preview);assert.equal(preview.mayPlay,false);assert.ok(preview.scope.baseFingerprint);
 const root=protobuf.Root.fromJSON(JSON.parse(readFileSync(new URL('../g3-waiting/descriptor.json',import.meta.url),'utf8')));
 const type=root.lookupType('nt2.waiting.EntryPreview');assert.throws(()=>decode('EntryPreview',type.encode(type.create({...preview,mayPlay:true})).finish()));
 assert.throws(()=>parseJson('EntryPreview',JSON.stringify({...preview,mayPlay:true})));
 const cancelled=golden.fixtures.find((f:any)=>f.type==='EntryState'&&f.json.state==='cancelled').json;assert.throws(()=>validate('EntryState',{...cancelled,state:'ready'}));
 const cancel=golden.fixtures.find((f:any)=>f.type==='Request'&&f.json.entryCancel).json;assert.throws(()=>validate('Request',{...cancel,entryCancel:{...cancel.entryCancel,worldOffline:false}}));
 const activated=golden.fixtures.find((f:any)=>f.type==='Request'&&f.json.entryActivate).json;assert.equal(activated.entryActivate.observation.attachmentId,undefined);assert.equal(activated.entryActivate.scope.originalStartId,undefined);assert.throws(()=>validate('EntryScope',{...activated.entryActivate.scope,originalStartId:'invented'}));
});
test('Pending and waiting states enforce false-only play, required samples and bounded durations in strict JSON/protobuf',()=>{
 const root=protobuf.Root.fromJSON(JSON.parse(readFileSync(new URL('../g3-waiting/descriptor.json',import.meta.url),'utf8')));
 for(const type of ['PendingState','WaitingState']){
  const value=golden.fixtures.find((f:any)=>f.type===type)?.json;assert.ok(value,'new recovery fixture '+type);
  assert.throws(()=>validate(type,{...value,mayPlay:true}));assert.throws(()=>parseJson(type,JSON.stringify({...value,mayPlay:true})));
  const message=root.lookupType('nt2.waiting.'+type);assert.throws(()=>decode(type,message.encode(message.create({...value,mayPlay:true})).finish()));
 }
 const pending=golden.fixtures.find((f:any)=>f.type==='PendingState').json;
 assert.throws(()=>validate('PendingState',{...pending,sample:undefined}));assert.throws(()=>validate('PendingState',{...pending,preparationRemainingMs:600001}));
 const missing={...pending};delete missing.sample;assert.throws(()=>validate('PendingState',missing));
 const missingPreparation={...pending};delete missingPreparation.preparationState;assert.throws(()=>validate('PendingState',missingPreparation));
 assert.throws(()=>validate('PendingState',{...pending,preparationState:'ready'}));
 assert.throws(()=>parseJson('PendingState',JSON.stringify({...pending,preparationState:'ready'})));
 const pendingType=root.lookupType('nt2.waiting.PendingState');assert.throws(()=>decode('PendingState',pendingType.encode(pendingType.create({...pending,preparationState:'ready'})).finish()));
 for(const preparationState of ['issued','consumed','loaded','cancelled','expired'])assert.deepEqual(decode('PendingState',encode('PendingState',{...pending,preparationState})),{...pending,preparationState});
 assert.ok(golden.fixtures.some((f:any)=>f.type==='Request'&&f.json.pendingRecover&&!f.json.pendingRecover.observation.attachmentId));
 assert.ok(golden.fixtures.some((f:any)=>f.type==='Request'&&f.json.reserve?.worldToken));
});
