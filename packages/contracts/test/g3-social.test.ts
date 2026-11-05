import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {schema,encode,decode,parseJson,validate} from '../src/g3-social.js';
test('separate social receiving codec agrees with generated binary/JSON goldens',async()=>{
 const golden=JSON.parse(await readFile(new URL('../g3-social/golden.json',import.meta.url),'utf8'));
 assert.equal(golden.revision,schema.revision);
 for(const f of golden.fixtures){assert.deepEqual(decode(f.type,Buffer.from(f.hex,'hex')),f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);assert.deepEqual(parseJson(f.type,JSON.stringify(f.json)),f.json);}
 for(const f of golden.rejected)assert.throws(()=>f.hex?decode(f.type,Buffer.from(f.hex,'hex')):validate(f.type,f.json));
 assert.throws(()=>parseJson('Request','{"major":1,"major":1,"snapshot":{}}'));
 const binary=Buffer.from(golden.fixtures[0].hex,'hex');assert.throws(()=>decode('Request',Buffer.concat([binary,Buffer.from([8,1])])));
 assert.throws(()=>decode('Request',binary.subarray(0,binary.length-1)));
 assert.throws(()=>parseJson('Poi',JSON.stringify({text:'x'.repeat(32769)})));
 assert.throws(()=>validate('Poi',{text:'safe',path:'evil.lua'}));
});
