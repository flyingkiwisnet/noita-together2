import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {schema,validate,encode,decode,parseJson} from '../src/g3-stats.js';
import {encode as encodeLobby,decode as decodeLobby} from '../src/g2-next.js';
test('D1 canonical fixtures, zero progression and explicit privacy OFF survive binary transport',()=>{
 const golden=JSON.parse(readFileSync(new URL('../g3-stats/golden.json',import.meta.url),'utf8'));
 for(const f of golden.fixtures){assert.deepEqual(decode(f.type,Buffer.from(f.hex,'hex')),f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);}
 assert.equal(decode('Observe',encode('Observe',{eventId:'original',kind:'orb',orbId:'128',progression:0})).progression,0);
 assert.equal(decodeLobby('RoomPolicy',encodeLobby('RoomPolicy',{publicStats:false})).publicStats,false);
});
test('D1 rejects arbitrary metric/count/target, forged IDs, unbounded lists and ambiguous envelopes',()=>{
 for(const kind of ['arbitrary','gold','total'])assert.throws(()=>validate('Observe',{kind}));
 for(const extra of [{count:999},{principalId:'forged'},{targetAccountId:'forged'}])assert.throws(()=>validate('Observe',{kind:'heart',...extra}));
 assert.throws(()=>validate('Request',{discover:{},observe:{kind:'heart'}}));assert.throws(()=>validate('Discover',{limit:33}));assert.throws(()=>validate('Summary',{rows:Array.from({length:33},()=>({}))}));
 assert.throws(()=>parseJson('Observe','{"kind":"heart","kind":"win"}'));assert.throws(()=>decode('Request',Buffer.from([0xf8,0x07,0x01])));assert.equal(schema.limits.bytes,32768);
});
