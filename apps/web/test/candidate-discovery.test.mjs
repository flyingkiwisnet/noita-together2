import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {HostClient} from '../candidate/client.js';
const schema=JSON.parse(await readFile(new URL('../../../packages/contracts/g2-next/schema.json',import.meta.url),'utf8'));
const caps=['core.seed','core.ghost','core.hud','core.poi'],semantics={revision:'g3-room-semantics-1',kind:'core-social',requiredCapabilities:caps};
const info={major:1,revision:schema.revision,serviceId:'development',serviceEpoch:'epoch',profile:{id:'one'},compatibility:{capabilities:caps},roomSemantics:semantics};
const client=value=>new HostClient(async path=>path==='/schema.json'?schema:structuredClone(value));
test('real browser discovery accepts semantic metadata across artifact IDs and refuses malformed/unknown extensions',async()=>{
 for(const id of ['old-runtime','nt2-g3-native-c2-development-1'])assert.deepEqual((await client({...info,profile:{id}}).initialize()).roomSemantics,semantics);
 for(const declaration of [null,{...semantics,revision:'future'},{...semantics,kind:'unknown'},{...semantics,requiredCapabilities:['core.seed']},{...semantics,extra:true}])await assert.rejects(client({...info,roomSemantics:declaration}).initialize(),/INCOMPATIBLE_GAME/);
 await assert.rejects(client({...info,unknownGrant:true}).initialize(),/INVALID_MESSAGE/);
 const legacy={...info};delete legacy.roomSemantics;assert.equal((await client(legacy).initialize()).roomSemantics,undefined);
});
