import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {encode,decode,validate,parseJson} from '../src/g3-connection.js';
import {encode as liveEncode,decode as liveDecode,schema as liveSchema} from '../src/g3-live.js';
import {canonicalOperationJson,operationFingerprint} from '../src/connection-fingerprint.js';
const golden=JSON.parse(readFileSync(new URL('../g3-connection/golden.json',import.meta.url),'utf8'));
test('CR2 original resume/offline canonical fingerprint vectors preserve exact original bytes and identity',()=>{
 const vectors=JSON.parse(readFileSync(new URL('../g3-connection/fingerprint-vectors.json',import.meta.url),'utf8'));
 for(const v of vectors.vectors){assert.equal(canonicalOperationJson(v.original),v.canonicalJson);assert.equal(operationFingerprint(v.original),v.sha256);assert.notEqual(operationFingerprint({...v.original,sessionEpoch:'replacement'}),v.sha256);assert.equal(operationFingerprint(Object.fromEntries(Object.entries(v.original).reverse())),v.sha256);}
});
test('CR2 canonical initial/held/current/resumed fixtures round trip and reject malformed durations',()=>{
 for(const f of golden.fixtures){assert.deepEqual(decode(f.type,encode(f.type,f.json)),f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);}
 for(const f of golden.rejected)assert.throws(()=>validate(f.type,f.json));
 assert.throws(()=>parseJson('ConnectionDecision','{"authorized":true,"authorized":false}'));
 assert.throws(()=>validate('ConnectionDecision',{}));
});
test('CR2 WSS ages/durations/ack are bounded and binary carries parallel pose ages',()=>{
 const f=golden.fixtures[0].json;
 const frame={major:1,revision:liveSchema.revision,serviceId:f.serviceId,serviceEpoch:f.serviceEpoch,connectionId:f.scope.sessionId,clientGeneration:1,authorityGeneration:1,sequence:2,scope:f.scope,ackClientSequence:1,authorityRemainingMs:12000,accessRemainingMs:800000,credentialId:'credential',snapshot:{acceptedAt:'2026-10-15T01:00:00.000Z',peers:[{principalId:f.scope.principalId,sessionId:f.scope.sessionId,attachmentId:f.scope.attachmentId,worldToken:f.scope.worldToken,acceptedAt:'2026-10-15T01:00:00.000Z',peerAgeMsAtSend:12,poseAgesMsAtSend:[30,12],poses:[1,2].map(sequence=>({sequence,xMilli:'1',yMilli:'0',vxMilli:'0',vyMilli:'0'}))}]}};
 assert.deepEqual(liveDecode('ServerFrame',liveEncode('ServerFrame',frame)),frame);
 for(const [key,value]of [['authorityRemainingMs',30001],['accessRemainingMs',900001]])assert.throws(()=>liveEncode('ServerFrame',{...frame,[key as string]:value}));
 assert.throws(()=>liveEncode('ServerFrame',{...frame,snapshot:{...frame.snapshot,peers:[{...frame.snapshot.peers[0],peerAgeMsAtSend:2001}]}}));
});
