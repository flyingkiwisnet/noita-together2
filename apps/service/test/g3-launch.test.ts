import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
test('G3 launcher refuses missing activation, production and nonisolated namespaces before key/database/listener work',()=>{
 for(const vars of [{NT2_G3_MOCK:''},{NT2_G3_MOCK:'1',NODE_ENV:'production'},{NT2_G3_MOCK:'1',NODE_ENV:'test',NT2_G3_NAMESPACE:'g2_next_live'}]){
  const child=spawnSync(process.execPath,['node_modules/tsx/dist/cli.mjs','apps/service/src/g3/main.ts'],{env:{...process.env,NT2_G3_KEYS_PATH:'absent-g3-fixture-keys.json',...vars},encoding:'utf8',timeout:10000});
  assert.notEqual(child.status,0);assert.match(child.stderr,/requires NT2_G3_MOCK|explicit separate/);assert.doesNotMatch(child.stderr,/ENOENT|connect ECONNREFUSED/);
 }
});
