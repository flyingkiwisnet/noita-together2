import test from 'node:test';import assert from 'node:assert/strict';
import {TESTING_STARTING_LOADOUT as C,testingStartingLoadout,testingLoadoutCompatibility} from '../../../packages/domain/src/testing-starting-loadout.js';
import {CandidateRooms} from '../src/g2-next/rooms.js';
import {encode,decode} from '../../../packages/contracts/src/g2-next.js';
import {readFileSync} from 'node:fs';
import {validateNativeRejoinLanConfig} from '../src/rejoin/lan.js';
const compatibility={gameVersion:'g',modVersion:'m',bridgeVersion:'b',capabilities:['core.seed'],modes:['core'],settingKeys:[]};
test('Testing loadout canonical Nightmare pickup catalogue, default off and existing Config serialization',()=>{
 assert.equal(C.settingKey,'NT_testing_mode');assert.equal(C.default,false);assert.equal(C.pickups.tinkerPerk,'EDIT_WANDS_EVERYWHERE');assert.equal(C.pickups.tinkerCount,1);assert.equal(C.pickups.noMoreShufflePerk,'NO_MORE_SHUFFLE');assert.equal(C.pickups.noMoreShuffleCount,1);assert.equal(C.pickups.randomPerks,3);assert.equal(C.pickups.randomWands,3);assert.equal(C.pickups.wandIds.length,6);assert.deepEqual(C.pickups.randomPerkExclusions,['EDIT_WANDS_EVERYWHERE','NO_MORE_SHUFFLE']);
 assert.equal(testingStartingLoadout({}),false);assert.equal(testingStartingLoadout({settings:[{key:C.settingKey,value:'false'}]}),false);
 for(const value of C.values){const config:import('../../../packages/contracts/g2-next/g2-next.js').Config={seed:42,mode:'core',requiredCapabilities:['core.seed'],settings:[{key:C.settingKey,value}]};assert.deepEqual(decode('Config',encode('Config',config)),config);assert.equal(testingStartingLoadout(config),value==='true');}
});
test('Testing loadout settings require explicit development service enablement; malformed values never coerce',()=>{
 const rooms=new CandidateRooms({} as any,{} as any,testingLoadoutCompatibility(compatibility,true)),config={seed:42,mode:'core',requiredCapabilities:['core.seed']};
 rooms.validateConfig(config);for(const value of C.values)assert.throws(()=>rooms.validateConfig({...config,settings:[{key:C.settingKey,value}]}),/FORBIDDEN/);
 rooms.testingStartingLoadoutEnabled=true;for(const value of C.values)rooms.validateConfig({...config,settings:[{key:C.settingKey,value}]});for(const value of ['TRUE','1','',true,null])assert.throws(()=>rooms.validateConfig({...config,settings:[{key:C.settingKey,value}]} as any));
 assert.equal(testingLoadoutCompatibility({...compatibility,settingKeys:[C.settingKey]},false).settingKeys.includes(C.settingKey),false);assert.deepEqual(compatibility.settingKeys,[]);
});
test('Development LAN configuration keeps Testing mode unavailable unless a literal boolean enables it',()=>{
 const base=JSON.parse(readFileSync(new URL('../../../deploy/g3-rejoin/config.c1-20261027.json',import.meta.url),'utf8'));
 assert.equal(validateNativeRejoinLanConfig(base).testingStartingLoadout,undefined);assert.equal(validateNativeRejoinLanConfig({...base,testingStartingLoadout:false}).testingStartingLoadout,false);assert.equal(validateNativeRejoinLanConfig({...base,testingStartingLoadout:true}).testingStartingLoadout,true);for(const value of ['true',1,{},null])assert.throws(()=>validateNativeRejoinLanConfig({...base,testingStartingLoadout:value}));
});
