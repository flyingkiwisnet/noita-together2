import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {C1_CATALOGUE as C,C1_DEFAULTS,normalizeC1Rules,normalizeC1Payload,c1SharingSkip} from '../../../packages/domain/src/g3-c1.js';
import {validate,encode,decode,parseJson} from '../../../packages/contracts/src/g3-c1.js';
test('C1 exact legacy defaults, conditional orb cap normalization and closed rule types',()=>{
 assert.deepEqual(normalizeC1Rules({}),C1_DEFAULTS);assert.equal(normalizeC1Rules({NT_sync_orbs:false,NT_sync_orb_count:40}).NT_sync_orb_count,0);assert.equal(normalizeC1Rules({NT_sync_orb_count:4294967295}).NT_sync_orb_count,4294967295);
 for(const v of [-1,1.5,4294967296])assert.throws(()=>normalizeC1Rules({NT_sync_orb_count:v}));assert.throws(()=>normalizeC1Rules({NT_death_penalty:'other'}));assert.throws(()=>normalizeC1Rules({NT_sync_hearts:'true'} as any));
});
test('C1 complete base catalogues, exact nine team perks, all/team/off precedence and Glass Cannon policy',()=>{
 assert.equal(C.perks.length,106);assert.ok(C.perks.every(p=>p.available));assert.equal(C.orbs.length,39);assert.equal(C.materials.length,466);
 assert.deepEqual(C.teamPerks,['EXTRA_MONEY','STRONG_KICK','GLOBAL_GORE','HEARTS_MORE_EXTRA_HP','ATTRACT_ITEMS','EXTRA_SHOP_ITEM','GENOME_MORE_HATRED','GENOME_MORE_LOVE','PEACE_WITH_GODS']);
 for(const perk of C.perks){const payload=normalizeC1Payload({perk:{perkId:perk.id}});assert.equal(c1SharingSkip(payload,C1_DEFAULTS),C.teamPerks.includes(perk.id)?undefined:'sharing_disabled');assert.equal(c1SharingSkip(payload,normalizeC1Rules({NT_sync_perks:true,NT_team_perks:false})),undefined);assert.equal(c1SharingSkip(payload,normalizeC1Rules({NT_team_perks:false})), 'sharing_disabled');}
 assert.equal(c1SharingSkip({perk:{perkId:'GLASS_CANNON'}},normalizeC1Rules({NT_sync_perks:true,NT_ban_perks:true})),'perk_banned');assert.throws(()=>normalizeC1Payload({perk:{perkId:'UNKNOWN'}}));
});
test('C1 actual main/PW orb IDs and named fungal mappings accept; paths, numbers, duplicate/missing materials and HP claims reject',()=>{
 for(const orb of C.orbs)assert.equal(normalizeC1Payload({orb:{orbId:orb.id}}).orb!.orbId,orb.id);assert.throws(()=>normalizeC1Payload({orb:{orbId:999999}}));
 const mapping={fromMaterials:['water','blood'],toMaterial:'gold',displayMaterial:'water'};assert.deepEqual(normalizeC1Payload({fungal:mapping}),{fungal:mapping});
 for(const p of [{fromMaterials:[],toMaterial:'gold'},{fromMaterials:['water','water'],toMaterial:'gold'},{fromMaterials:['123'],toMaterial:'gold'},{fromMaterials:['water'],toMaterial:'../../evil.lua'}])assert.throws(()=>normalizeC1Payload({fungal:p}));
 assert.throws(()=>normalizeC1Payload({heart:{hp:500}} as any));assert.throws(()=>normalizeC1Payload({heart:{},orb:{orbId:0}}));
});
test('C1 strict JSON/protobuf fixtures and exact NT_* Rust serde keys remain coherent',()=>{
 const golden=JSON.parse(readFileSync('packages/contracts/g3-c1/golden.json','utf8'));for(const f of golden.fixtures){assert.deepEqual(decode(f.type,encode(f.type,f.json)),f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);assert.deepEqual(parseJson(f.type,JSON.stringify(f.json)),f.json);}for(const f of golden.rejected)assert.throws(()=>f.hex?decode(f.type,Buffer.from(f.hex,'hex')):validate(f.type,f.json));
 assert.throws(()=>parseJson('Rules','{"NT_sync_hearts":true,"NT_sync_hearts":false}'));const rust=readFileSync('packages/contracts/g3-c1/g3-c1.rs','utf8');for(const key of Object.keys(C1_DEFAULTS))assert.ok(rust.includes('serde(rename = "'+key+'"'));
});

test('C1 winning original retains validated captured progression including zero and rejects it on ordinary death',()=>{for(const progression of [0,1,4294967295]){const payload={death:{state:'won',cause:'normal',progression}};assert.deepEqual(normalizeC1Payload(payload),payload);assert.deepEqual(decode('Payload',encode('Payload',payload)),payload);}assert.throws(()=>normalizeC1Payload({death:{state:'terminal',cause:'normal',progression:0}}));});
