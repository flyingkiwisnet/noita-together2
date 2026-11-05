import test from 'node:test';import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {validate,parseJson,encode,decode} from '../src/g3-history.js';
test('separate history discovery codec round trips JSON/binary and refuses principal/scope injection',async()=>{
 const gold=JSON.parse(await readFile(new URL('../g3-history/golden.json',import.meta.url),'utf8'));
 for(const v of gold.fixtures){assert.deepEqual(decode(v.type,Buffer.from(v.hex,'hex')),v.json);assert.equal(Buffer.from(encode(v.type,v.json)).toString('hex'),v.hex);}
 assert.throws(()=>validate('Request',{discover:{},principalId:'forged'}));assert.throws(()=>validate('Request',{discover:{},scope:{}}));assert.throws(()=>parseJson('Request','{"discover":{},"discover":{}}'));assert.throws(()=>validate('Discover',{afterSequence:'1'.repeat(20)}));assert.throws(()=>validate('Response',{runs:Array(33).fill({})}));
});
