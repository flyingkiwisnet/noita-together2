import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {resolve} from 'node:path';
import {tmpdir} from 'node:os';
import {createServer} from 'node:net';
import {readFile} from 'node:fs/promises';
import {schema} from '../../../packages/contracts/src/g2-next.js';
const script=resolve('deploy/g2-next/start-server.ps1');
const check=(env:NodeJS.ProcessEnv={})=>spawnSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',script,'-Check'],{cwd:tmpdir(),env:{...process.env,...env},encoding:'utf8',timeout:20000});
test('human host Check refuses production, unreviewed configuration or unavailable custom DB without starting a service or Compose',{skip:process.platform!=='win32'},async()=>{
 const production=check({NODE_ENV:'production'});assert.notEqual(production.status,0);assert.match(production.stderr,/mock-only/);
 const config=JSON.parse(await readFile('deploy/g2-next/config.example.json','utf8'));if(config.contractRevision!==schema.revision){const pending=check({NODE_ENV:'development'});assert.notEqual(pending.status,0);assert.match(pending.stderr,/configuration or pinned contract differs/);return;}
 const reservation=createServer();await new Promise<void>(done=>reservation.listen(0,'127.0.0.1',done));const address=reservation.address();assert.ok(address&&typeof address!=='string');await new Promise<void>((done,fail)=>reservation.close(e=>e?fail(e):done()));
 const absent=check({NODE_ENV:'development',NT2_G2_DATABASE_URL:`postgres://nt2_g1:local-fixture-only@127.0.0.1:${address.port}/nt2_g1`});assert.notEqual(absent.status,0);if(absent.stderr.includes('Port4356 is already in use'))assert.match(absent.stderr,/leave4352 running/);else{assert.match(absent.stderr,/custom PostgreSQL is not running/);assert.match(absent.stderr,/Check mode starts nothing/);}
});
