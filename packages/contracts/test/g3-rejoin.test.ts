import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {encode,decode,validate,parseJson} from '../src/g3-rejoin.js';
test('RJ1 canonical golden wire roundtrip and bounded JSON/protobuf refusal',async()=>{
 const golden=JSON.parse(await readFile(new URL('../g3-rejoin/golden.json',import.meta.url),'utf8'));for(const f of golden.fixtures){assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);assert.deepEqual(decode(f.type,Buffer.from(f.hex,'hex')),f.json);}for(const f of golden.rejected)assert.throws(()=>validate(f.type,f.json));
 assert.throws(()=>parseJson('Request','{"status":{},"status":{}}'));assert.throws(()=>decode('Request',Buffer.from('a00601','hex')));assert.throws(()=>validate('Resume',{resumeProof:'a'.repeat(44)}));
});
