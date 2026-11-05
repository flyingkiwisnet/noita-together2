import test from 'node:test';import assert from 'node:assert/strict';import {createRequire} from 'node:module';
import {ITEM_CATALOGUE as C,normalizeItem,itemPresentation} from '../../../packages/domain/src/g3-items.js';
import {encode,decode,validate} from '../../../packages/contracts/src/g3.js';import {fingerprint} from '../src/g1/security.js';
import {readFileSync} from 'node:fs';import protobuf from 'protobufjs';
const require=createRequire(import.meta.url),examples=require('../../../docs/g3/server-item-transfer-examples.json'),vectors=require('../../../packages/contracts/g3-items/operation-digest.json');
test('Descriptors preserve unusual wand stats, slots, permanent casts, spell charges and exact numeric strings through canonical codecs',()=>{
 for(const item of Object.values(examples.descriptors) as any[]){const normalized=normalizeItem(item);assert.deepEqual(normalizeItem(decode('ItemDescriptor',encode('ItemDescriptor',normalized))),normalized);}
 const w=normalizeItem({...examples.descriptors.wand,wand:{...examples.descriptors.wand.wand,uiName:'雪 [wand]',reloadTime:'-2147483648',fireRateWait:'-100',spreadDegrees:'-360.125',deckCapacity:'2147483647',deck:[{slot:0,spell:{actionId:C.actions[0].id,usesRemaining:'-1'}},{slot:14,spell:{actionId:C.actions[1].id,usesRemaining:'0'}}]}}).wand!;
 assert.deepEqual(w.deck!.map(v=>v.slot),[0,14]);assert.equal(w.deck![1]!.spell!.usesRemaining,'0');assert.equal(w.deckCapacity,'2147483647');assert.equal(w.fireRateWait,'-100');assert.equal(w.permanent!.length,examples.descriptors.wand.wand.permanent.length);
 for(const amount of ['0.125','4.9406564584124654e-324','1.7976931348623157e308'])assert.equal(normalizeItem({catalogueRevision:C.revision,container:{containerType:'potion',color:'4294967295',contents:[{materialId:C.materials[0].id,amount}]}}).container!.contents![0]!.amount,amount);
 for(const v of vectors.vectors){if(v.direction==='deposit')assert.deepEqual(normalizeItem(v.original.payload),v.original.payload);assert.deepEqual(JSON.parse(v.canonicalJson),v.original);assert.equal(fingerprint(v.original),v.sha256);assert.equal(Buffer.from(v.canonicalJson).toString('hex'),v.utf8Hex);}
});
test('Unsupported paths, malformed numeric data, slots, materials and invalid strings reject whole descriptors',()=>{
 const base=examples.descriptors.wand;
 for(const change of [{spriteId:'data/entities/evil.xml'},{manaMax:'NaN'},{manaMax:'Infinity'},{manaMax:'1e999'},{reloadTime:'1.5'},{deckCapacity:'-1'},{uiName:'bad\ud800'},{uiName:'nul\0name'},{deck:[{slot:1,spell:examples.descriptors.spell.spell},{slot:1,spell:examples.descriptors.spell.spell}]},{deck:[{slot:0,spell:{actionId:'unknown',usesRemaining:'-1'}}]}])assert.throws(()=>normalizeItem({...base,wand:{...base.wand,...change}}));
 for(const amount of ['0','-1','NaN','Infinity','1e309','1e-999'])assert.throws(()=>normalizeItem({catalogueRevision:C.revision,container:{containerType:'powder',color:'0',contents:[{materialId:C.materials[0].id,amount}]}}));
 assert.throws(()=>normalizeItem({catalogueRevision:C.revision,container:{containerType:'potion',color:'0',contents:[{materialId:'1',amount:'1'}]}}));
 assert.throws(()=>normalizeItem({catalogueRevision:C.revision,container:{containerType:'potion',color:'0',contents:[{materialId:C.materials[0].id,amount:'1'},{materialId:C.materials[0].id,amount:'2'}]}}));
 assert.throws(()=>normalizeItem({...examples.descriptors.spell,catalogueRevision:'other'}));
 assert.throws(()=>validate('InventoryPublish',{revision:1,wands:Array.from({length:9},(_,slot)=>({slot,wand:base.wand}))}));
});
test('Base metadata covers all legacy objects with three explicit absent assets and trusted search labels',()=>{
 assert.equal(C.actions.length,422);assert.equal(C.materials.length,466);assert.equal(C.sprites.length,1046);assert.equal(C.objects.length,43);assert.equal(C.objects.filter(v=>v.available).length,40);
 for(const o of C.objects){const item={catalogueRevision:C.revision,object:{objectId:o.id}};if(o.available){assert.equal(normalizeItem(item).object!.objectId,o.id);assert.ok(itemPresentation(item).displayName);}else{assert.equal(o.unavailableReason,'missing-base-asset');assert.throws(()=>normalizeItem(item));}}
});
test('Published protobuf and JSON descriptor encode repeated spell filters identically, including type zero',()=>{
 const type=protobuf.parse(readFileSync(new URL('../../../packages/contracts/g3/g3.proto',import.meta.url),'utf8')).root.lookupType('nt2.g3.ItemBank');
 const value={tab:'spells',useFilters:true,spellTypes:[0,1,7]};const bytes=type.encode(type.fromObject(value)).finish();assert.deepEqual(decode('ItemBank',bytes),value);assert.deepEqual(Buffer.from(bytes),Buffer.from(encode('ItemBank',value)));
});


test('Reviewed base wand XML and PNG appearances retain exact trusted IDs through canonical codecs',()=>{
 assert.equal(C.revision,'nt2-items-base-2');
 const paths=['data/items_gfx/handgun','data/items_gfx/bomb_wand','data/items_gfx/flute','data/items_gfx/kantele','data/items_gfx/machinegun',...['1','2'].map(n=>'data/entities/items/wands/experimental/experimental_wand_'+n)];
 for(const prefix of paths)for(const extension of ['xml','png']){const path=prefix+(prefix.includes('/experimental/')&&extension==='xml'?'_sprite':'')+'.'+extension,entry=C.sprites.find(v=>v.path===path);assert.ok(entry,path);const item=normalizeItem({...examples.descriptors.wand,wand:{...examples.descriptors.wand.wand,spriteId:entry.id}});assert.deepEqual(normalizeItem(decode('ItemDescriptor',encode('ItemDescriptor',item))),item);assert.equal(itemPresentation(item).iconId,entry.id);}
 for(const path of ['data/particles/ray.xml','data/items_gfx/unknown.png','../../evil.xml'])assert.throws(()=>normalizeItem({...examples.descriptors.wand,wand:{...examples.descriptors.wand.wand,spriteId:path}}));
 assert.throws(()=>normalizeItem({...examples.descriptors.wand,catalogueRevision:'nt2-items-base-1'}),/INCOMPATIBLE_CATALOGUE/);
});
