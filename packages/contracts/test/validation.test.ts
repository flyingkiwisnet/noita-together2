import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parseCommand, BoundaryError } from '../src/index.js';
import { MockProvider, roomFixtures, command } from '@nt2/test-kit';
const session = new MockProvider().issue('host','game').session;
const valid = command(session,roomFixtures()[0]!, 'room.join');
test('shared JSON fixtures agree with the actual receiving validator', () => {
  const fixture = (name:string) => JSON.parse(readFileSync(new URL(`../fixtures/${name}`, import.meta.url),'utf8'));
  assert.equal(parseCommand(fixture('join.valid.json')).kind,'room.join');
  assert.throws(() => parseCommand(fixture('forged-sender.invalid.json')), BoundaryError);
});
test('reject scalar, null, event control names and identity injection before dispatch', () => {
  for (const value of [null, 1, [], 'error', {...valid,userId:'victim'}, {...valid,kind:'error'}, {...valid,body:{accountId:'victim'}}]) {
    assert.throws(() => parseCommand(value), BoundaryError);
  }
});
test('reject invalid revision, epoch, seed, chat length and unexpected body keys', () => {
  for (const value of [
    {...valid,expectedRevision:-1}, {...valid,expectedRevision:0.5}, {...valid,connectionEpoch:''},
    {...valid,contractRevision:'legacy'}, {...valid,kind:'game.seedAccepted',body:{seed:NaN}},
    {...valid,kind:'game.seedAccepted',body:{seed:4294967296}},
    {...valid,kind:'chat.send',body:{text:'x'.repeat(501)}}, {...valid,kind:'chat.send',body:{text:' '}}
  ]) assert.throws(() => parseCommand(value), BoundaryError);
  assert.deepEqual(parseCommand(valid),valid);
});
