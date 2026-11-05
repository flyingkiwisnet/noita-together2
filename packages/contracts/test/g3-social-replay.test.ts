import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {SOCIAL_REPLAY_POLICY,transientEventId} from '../src/g3-social-replay.js';
test('published social replay identity vectors bind UTF-8 authority, kind and nonwrapping sequence',async()=>{
 const policy=JSON.parse(await readFile(new URL('../g3-social/replay-policy.json',import.meta.url),'utf8'));
 assert.equal(policy.policy,SOCIAL_REPLAY_POLICY);
 for(const v of policy.vectors){
  assert.equal(Buffer.from(v.canonicalJson).toString('hex'),v.utf8Hex);
  assert.equal(createHash('sha256').update(v.canonicalJson).digest('hex'),v.sha256);
  assert.equal(transientEventId(v.scope,v.gameSessionId,v.kind,v.sequence),v.eventId);
  assert.equal(v.eventId[14],'8');assert.match(v.eventId[19],/[89ab]/);
  assert.notEqual(transientEventId(v.scope,v.gameSessionId,v.kind,v.sequence-1),v.eventId);
  assert.notEqual(transientEventId({...v.scope,worldToken:'foreign'},v.gameSessionId,v.kind,v.sequence),v.eventId);
  assert.notEqual(transientEventId(v.scope,'66666666-6666-4666-8666-666666666666',v.kind,v.sequence),v.eventId);
 }
 assert.equal(new Set(policy.vectors.map((v:any)=>v.eventId)).size,3);
});
