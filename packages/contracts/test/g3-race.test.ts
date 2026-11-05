import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {encode,decode,validate,parseJson} from '../src/g3-race.js';
import {pureRaceObservation,validateRaceConfig} from '../../domain/src/g3-race-config.js';
const read=(file:string)=>JSON.parse(readFileSync(new URL(file,import.meta.url),'utf8'));
test('Race generated golden JSON/protobuf includes terminal, receipt and snapshot nested projections',()=>{
 const golden=read('../g3-race/golden.json');
 for(const f of golden.fixtures){validate(f.type,f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);assert.deepEqual(decode(f.type,Buffer.from(f.hex,'hex')),f.json);}
 assert.ok(golden.fixtures.some((f:any)=>f.type==='Commit'&&f.json.terminal==='dead'));
 assert.ok(golden.fixtures.some((f:any)=>f.type==='Response'&&f.json.receipt.status==='finished'));
 assert.ok(golden.fixtures.some((f:any)=>f.type==='Snapshot'&&f.json.entrants[0].acceptedSequence===2));
 for(const f of golden.rejected)assert.throws(()=>validate(f.type,f.json));
});
test('Race bounded observations, duplicate JSON keys and canonical config reject malformed input',()=>{
 assert.throws(()=>validate('Commit',{eventId:'x',terminal:'none',sequence:1,observations:new Array(33).fill({kind:'orb',orbId:0})}));
 assert.throws(()=>validate('Request',{read:{},commit:{}}));assert.throws(()=>parseJson('Commit','{"sequence":1,"sequence":2}'));
 for(const value of ['NaN','Infinity','-1','01','1e3','2147483648','500.123456789012345678901'])assert.throws(()=>pureRaceObservation({kind:'max_hp',value}));
 assert.deepEqual(pureRaceObservation({kind:'max_hp',value:'502.50000953674316'}),{kind:'max_hp',value:502.50000953674316});
 assert.deepEqual(pureRaceObservation({kind:'max_hp',value:'500.125'}),{kind:'max_hp',value:500.125});
 assert.throws(()=>pureRaceObservation({kind:'gold',value:'1.5'}));
 const goal={instanceId:'hp500',id:'reach_max_hp',target:500},race={registryRevision:'race-goals-1',durationMs:0,goals:[goal]};
 const config={mode:'race',seed:1,seedPolicy:'shared-fixed',requiredCapabilities:['race.goals.v1'],race};validateRaceConfig(config);
 for(const patch of [{seedPolicy:'individual-random'},{settings:[{key:'NT_testing_mode',value:'true'}]},{settings:[{key:'NT_send_gold',value:'true'}]},{requiredCapabilities:[]},{race:{...race,durationMs:-1}},{race:{...race,goals:[{...goal,target:499}]}}])assert.throws(()=>validateRaceConfig({...config,...patch}));
 const lobby=read('../g2-next/schema.json'),schema=read('../g3-race/schema.json');assert.deepEqual(lobby.messages.RaceGoal,schema.messages.RaceGoal);assert.deepEqual(lobby.messages.RaceConfig,schema.messages.RaceConfig);
 const rust=readFileSync(new URL('../g3-race/g3-race.rs',import.meta.url),'utf8');assert.match(rust,/pub self_row_requested:/);assert.doesNotMatch(rust,/pub self:/);
});
