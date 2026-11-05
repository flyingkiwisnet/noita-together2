import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';
test('mock executable refuses accidental activation and production mode', async () => {
  for (const env of [{NT2_G0_MOCK:'0',NODE_ENV:'development'}, {NT2_G0_MOCK:'1',NODE_ENV:'production'}]) {
    const child = spawn(process.execPath,['--import','tsx',fileURLToPath(new URL('../src/main.ts',import.meta.url))],{
      env:{...process.env,...env}, stdio:['ignore','pipe','pipe']
    });
    let stderr = ''; child.stderr.on('data', data => {stderr += data.toString();});
    const [code] = await once(child,'exit'); assert.notEqual(code,0); assert.match(stderr,/refuses production/);
  }
});
