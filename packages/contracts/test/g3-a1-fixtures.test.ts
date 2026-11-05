import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {validate as movementValidate,encode as movementEncode,decode as movementDecode} from '../src/g3.js';
import {validate as socialValidate,encode as socialEncode,decode as socialDecode} from '../src/g3-social.js';
import {transientEventId} from '../src/g3-social-replay.js';import {authorizeSocial} from '../../domain/src/g3-social.js';
const load=async()=>JSON.parse(await readFile(new URL('../../test-kit/fixtures/g3-a1-delivery.json',import.meta.url),'utf8'));
test('A1 delivery fixtures are exact current JSON/protobuf requests without invented frame timestamps or source authority',async()=>{
 const fixture=await load();for(const request of [fixture.movement.first,fixture.movement.replacement,fixture.movement.read]){movementValidate('Request',request);assert.deepEqual(movementDecode('Request',movementEncode('Request',request)),request);}
 assert.deepEqual(fixture.movement.first.movement.frames.map((f:any)=>f.sequence),[1,2,4,7]);assert.ok(fixture.movement.first.movement.frames.every((f:any)=>!('capturedAt' in f)));assert.match(fixture.movement.timing,/no per-frame source capture time/);
});
test('A2 prepared HUD/POI examples match canonical identity, wire bounds and existing authenticated domain payload policy',async()=>{
 const fixture=await load();for(const request of Object.values(fixture.a2).filter((v:any)=>v&&typeof v==='object'&&v.revision)){socialValidate('Request',request);const decoded=socialDecode('Request',socialEncode('Request',request));socialValidate('Request',decoded);}
 const hud=fixture.a2.hudPublish,poi=fixture.a2.poiPublish;assert.equal(hud.publish.eventId,transientEventId(hud.scope,fixture.actors.senderGameSessionId,'hud',hud.publish.sequence));assert.ok(!['principalId','displayName','provenance'].some(key=>key in hud.publish));
 const sender={principalId:fixture.actors.principalId,gameSessionId:fixture.actors.senderGameSessionId,worldToken:hud.scope.worldToken,roomId:hud.scope.roomId,runId:hud.scope.runId,mode:'core',live:true,capabilities:['core.hud','core.poi']};
 assert.equal(authorizeSocial(sender,{catalogId:'player.hud',...hud.publish.hud,hpKnown:true,sampoKnown:true},[sender]).payload.catalogId,'player.hud');assert.equal(authorizeSocial(sender,{catalogId:'world.poi',...poi.publish.poi,xMilli:Number(poi.publish.poi.xMilli),yMilli:Number(poi.publish.poi.yMilli)},[sender]).provenance,'validated-client-report');
});
