import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {encode,decode,parseJson,validate} from '../src/g2.js';
const golden=createRequire(import.meta.url)('../g2/golden.json') as {fixtures:{type:string;json:unknown;hex:string}[];rejected:{type:string;json?:unknown;hex?:string}[]};
test('G2 published JSON/binary fixtures and hostile text roundtrip',()=>{for(const f of golden.fixtures){assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);assert.deepEqual(decode(f.type,Buffer.from(f.hex,'hex')),f.json);assert.deepEqual(parseJson(f.type,JSON.stringify(f.json)),f.json);}for(const f of golden.rejected)assert.throws(()=>f.hex?decode(f.type,Buffer.from(f.hex,'hex')):validate(f.type,f.json));assert.throws(()=>parseJson('ChatSend','{"text":"a","text":"b"}'));assert.throws(()=>validate('ChatSend',{text:'🔥'.repeat(257)}));});
