import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {schema,encode,decode,validate,parseJson,InvalidMessage} from '../src/g1.js';
const request={major:1,revision:schema.revision,serviceId:'nt2-g1-local',requestId:'golden-request',roomGet:{}};
test('G1 binary/JSON match the same bounded typed schema',()=>{assert.deepEqual(decode('Request',encode('Request',request)),request);assert.deepEqual(parseJson('Request',JSON.stringify(request)),request);});
test('G1 rejects unknown fields, conflicting payloads, overlong fields and out-of-range numbers',()=>{
 for(const value of [{...request,sender:'host'},{...request,logout:{}},{...request,serviceId:'a'.repeat(129)},{...request,major:4294967296},{...request,major:1.2}])assert.throws(()=>validate('Request',value),InvalidMessage);
 assert.throws(()=>parseJson('Request',' '.repeat(65537)),InvalidMessage);
});
test('G1 rejects unknown binary tags, duplicate singulars, noncanonical bools and truncation',()=>{
 const wire=encode('Request',request);for(const tail of [[0x78,1],[0x08,1],[0x88,0],[0xa2,1,3,1]])assert.throws(()=>decode('Request',Buffer.concat([wire,Buffer.from(tail)])),InvalidMessage);
 assert.throws(()=>decode('Presence',Buffer.from([8,2])),InvalidMessage);
});
test('G1 fixture binary is generated from canonical source',()=>{const golden=JSON.parse(readFileSync(new URL('../g1/golden.json',import.meta.url),'utf8'));for(const entry of golden.fixtures){assert.deepEqual(decode(entry.type,Buffer.from(entry.hex,'hex')),entry.json);assert.equal(Buffer.from(encode(entry.type,entry.json)).toString('hex'),entry.hex);}});
test('G1 published rejected fixtures and duplicate JSON keys fail',()=>{
 const golden=JSON.parse(readFileSync(new URL('../g1/golden.json',import.meta.url),'utf8'));for(const e of golden.rejected)assert.throws(()=>e.hex?decode(e.type,Buffer.from(e.hex,'hex')):validate(e.type,e.json),InvalidMessage);
 assert.throws(()=>parseJson('Request',JSON.stringify(request).replace('"major":1','"major":2,"major":1')),InvalidMessage);
});
