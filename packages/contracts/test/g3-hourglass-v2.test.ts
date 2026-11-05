import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {decode,encode,parseJson,validate} from '../src/g3-hourglass-v2.js';
test('isolated hourglass2 codec preserves fixed bounds and canonical JSON/protobuf fixtures',async()=>{
 const golden=JSON.parse(await readFile(new URL('../g3-hourglass-v2/golden.json',import.meta.url),'utf8'));
 for(const f of golden.fixtures){assert.deepEqual(decode(f.type,Buffer.from(f.hex,'hex')),f.json);assert.deepEqual(parseJson(f.type,JSON.stringify(f.json)),f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);}
 assert.throws(()=>validate('Request',{status:{},source:{}}));assert.throws(()=>parseJson('Request','{"status":{},"status":{}}'));assert.throws(()=>validate('Response',{recipientStatuses:Array(33).fill({})}));
});
