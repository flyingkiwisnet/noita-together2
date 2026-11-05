import test from 'node:test';import assert from 'node:assert/strict';
import {authorizeSocial,SOCIAL_CATALOG} from '../../../packages/domain/src/g3-social.js';
import {encode,decode,schema} from '../../../packages/contracts/src/g3-social.js';
const sender={principalId:'a',gameSessionId:'s',worldToken:'world',roomId:'room',runId:'run',mode:'core',live:true,capabilities:['core.hud','core.poi']};
test('A2 known zero HP, unknown facts and biome remain distinct and bounded',()=>{
 const hud={catalogId:'player.hud' as const,hpMilli:0,maxHpMilli:1000,sampo:false,hpKnown:true,sampoKnown:true,biome:'$biome_coalmine'};
 assert.doesNotThrow(()=>authorizeSocial(sender,hud,[sender]));assert.doesNotThrow(()=>authorizeSocial(sender,{...hud,maxHpMilli:0,hpKnown:false,sampoKnown:false},[sender]));
 for(const bad of [{...hud,maxHpMilli:0},{...hud,hpKnown:false,hpMilli:1},{...hud,sampoKnown:false,sampo:true},{...hud,biome:'雪'.repeat(43)},{...hud,biome:'\u0000'},{...hud,biome:'\ud800'}])assert.throws(()=>authorizeSocial(sender,bad,[sender]));
 assert.equal(SOCIAL_CATALOG.find(c=>c.id==='player.hud')!.maxPerSecond,2);
 const value={hpMilli:0,maxHpMilli:0,sampo:false,hpKnown:false,sampoKnown:false};assert.deepEqual(decode('Hud',encode('Hud',value)),{}); // Protobuf default scalars are absent; service normalizes to unknown/zero.
 assert.deepEqual(decode('Response',encode('Response',{major:1,revision:schema.revision,players:[],rosterComplete:true})),{major:1,revision:schema.revision,rosterComplete:true});
 assert.throws(()=>encode('Player',{state:'fabricated'}));assert.throws(()=>encode('PoiOutcome',{eventId:'x',status:'accepted'}));assert.throws(()=>encode('PoiOutcome',{eventId:'x',status:'retry-new-id'}));
});
test('worst bounded UTF8 marker pages require chunking below32KB',()=>{
 const s='x'.repeat(80),scope={roomId:s,runId:s,runEpoch:s,configRevision:0xffffffff,attachmentId:s,worldToken:s};
 const marker={poiSequence:'9223372036854775807',eventId:s,principalId:s,gameSessionId:s,displayName:'"'.repeat(80),scope,poi:{text:'😀'.repeat(99),xMilli:'-2147483647',yMilli:'-2147483647'},capturedAt:'2'.repeat(40),provenance:'validated-client-report'};
 const envelope={major:1,revision:schema.revision,serviceId:s,serviceEpoch:s,sessionEpoch:s,requestId:s,scope,nextSequence:'9223372036854775807',highSequence:'9223372036854775807'};
 const old={...envelope,markers:Array(32).fill(marker)};assert.ok(Buffer.byteLength(JSON.stringify(old))>schema.limits.bytes);assert.throws(()=>encode('Response',old));
 const page={...envelope,markers:Array(16).fill(marker)};assert.ok(Buffer.byteLength(JSON.stringify(page))<schema.limits.bytes);assert.ok(encode('Response',page).length<schema.limits.bytes);console.log(JSON.stringify({a2PageBytes:{oldJson:Buffer.byteLength(JSON.stringify(old)),json:Buffer.byteLength(JSON.stringify(page)),protobuf:encode('Response',page).length,limit:schema.limits.bytes}}));
});
