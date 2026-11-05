import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {encode,decode,parseJson,validate} from '../src/g3-live.js';
import {validateLiveFrame,acceptLiveFrame} from '../../domain/src/g3-live.js';
test('live generated examples, negative bounds, exact scopes and late-frame fences',async()=>{
 const golden=JSON.parse(await readFile(new URL('../g3-live/golden.json',import.meta.url),'utf8'));
 for(const f of golden.fixtures){validate(f.type,f.json);assert.deepEqual(decode(f.type,encode(f.type,f.json)),f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);validateLiveFrame(f.type,f.json);}
 for(const f of golden.rejected)assert.throws(()=>validate(f.type,f.json));
 const pose=golden.fixtures[1].json,binding={serviceId:pose.serviceId,serviceEpoch:pose.serviceEpoch,connectionId:pose.connectionId,clientGeneration:pose.clientGeneration,authorityGeneration:pose.authorityGeneration,scope:pose.scope,lastSequence:1,lastPoseSequence:0,authRevision:1,accessUntil:2000,authorityUntil:1000};
 assert.equal(acceptLiveFrame(pose,binding,999).pose.sequence,1);
 for(const patch of [{scope:{...pose.scope,worldToken:'foreign'}},{clientGeneration:2},{authorityGeneration:2},{connectionId:'00000000-0000-4000-8000-000000000099'},{sequence:1}])assert.throws(()=>acceptLiveFrame({...pose,...patch},binding,999),/STALE/);
 assert.throws(()=>acceptLiveFrame(pose,binding,1000),/EXPIRED/);
 assert.throws(()=>acceptLiveFrame(pose,{...binding,lastPoseSequence:1},999),/STALE/);
 const {pose:ignored,...header}=pose;assert.equal(acceptLiveFrame({...header,reauthenticate:{accessToken:'example',authRevision:2}},binding,999).reauthenticate.authRevision,2);
 assert.throws(()=>acceptLiveFrame({...header,reauthenticate:{accessToken:'example',authRevision:1}},binding,999),/STALE/);
 assert.throws(()=>parseJson('ClientFrame','{"major":1,"major":1}'));
 assert.throws(()=>decode('ClientFrame',Buffer.concat([encode('ClientFrame',pose),Buffer.from('f80701','hex')])));
 assert.throws(()=>decode('ClientFrame',new Uint8Array(32769)));
});
