import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { componentPlan } from '../scripts/components.mjs';
const root=new URL('../../../',import.meta.url);
const pkg=JSON.parse(readFileSync(new URL('package.json',root),'utf8'));
test('component registration resolves existing scripts and client tools on each platform',()=>{
  for (const platform of ['win32','linux']) for (const action of ['check','test','build']) {
    for (const step of componentPlan(action,platform)) {
      if (step.kind==='npm') assert.ok(pkg.scripts[step.name],step.name);
      if (step.kind==='node' || step.kind==='powershell') assert.ok(existsSync(new URL(step.args[0],root)),step.args[0]);
    }
  }
  for (const action of ['test:client','build:native','build:client']) {
    const [step]=componentPlan(action,'win32');
    assert.equal(step.kind,'powershell');
    assert.ok(existsSync(new URL(step.args[0],root)));
    assert.throws(()=>componentPlan(action,'linux'),/requires/);
  }
  assert.deepEqual(componentPlan('test','win32').filter(s=>s.kind==='npm').map(s=>s.name),
    ['contracts:check','test:server','test:web','test:client']);
  assert.deepEqual(componentPlan('build','win32').filter(s=>s.kind==='npm').map(s=>s.name),
    ['contracts:check','build:server','build:web','build:native','build:client']);
  assert.equal(componentPlan('build','linux').at(-1).kind,'skipped');
  assert.throws(()=>componentPlan('unknown'),/Unknown/);
});
