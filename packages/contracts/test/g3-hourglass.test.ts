import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {validate,encode,decode,parseJson} from '../src/g3-hourglass.js';
test('separate hourglass golden JSON/binary codec rejects executable fields, attribution/target injection and malformed frames',async()=>{
 const golden=JSON.parse(await readFile(new URL('../g3-hourglass/golden.json',import.meta.url),'utf8'));
 for(const fixture of golden.fixtures){assert.deepEqual(decode(fixture.type,Buffer.from(fixture.hex,'hex')),fixture.json);assert.equal(Buffer.from(encode(fixture.type,fixture.json)).toString('hex'),fixture.hex);assert.deepEqual(parseJson(fixture.type,JSON.stringify(fixture.json)),fixture.json);}
 for(const field of ['callback','entityPath','source','target','settings'])assert.throws(()=>validate('Activate',{activationId:'id',effectId:'ambrosia',materialId:'magic_liquid_protection_all',[field]:'forged'}));
 for(const fixture of golden.rejected)assert.throws(()=>validate(fixture.type,fixture.json));
 assert.throws(()=>parseJson('Request','{"pending":{},"pending":{}}'));assert.throws(()=>validate('Request',{pending:{},activate:{}}));assert.throws(()=>validate('Response',{deliveries:Array(33).fill({})}));
 assert.throws(()=>decode('Activate',Uint8Array.from([0xf8,0x07,0x01])));assert.throws(()=>decode('Activate',Uint8Array.from([0x12,1,0x61,0x12,1,0x62])));assert.throws(()=>decode('Activate',Uint8Array.from([0x12,1,0xff])));
 assert.throws(()=>parseJson('Request','{"pending":{},"noise":"'+'x'.repeat(32768)+'"}'));
});
