import {nativePotionDiagnosticRuntimeResource,nativePotionDiagnosticSourceResource} from '../src/g3/native-potion-diagnostic-profile.js';
import test from 'node:test';import assert from 'node:assert/strict';import {spawn} from 'node:child_process';import {readFile,access} from 'node:fs/promises';import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url);
test('Separate potion diagnostic CLI fixture supplies exact complete profile/live selector and private bundle cleanup',async()=>{
 const args=['--potion-diagnostic','--live','--tls','--playing','--keepalive'];
 const child=spawn(process.execPath,[fileURLToPath(new URL('node_modules/tsx/dist/cli.mjs',root)),'apps/service/scripts/g3-client-fixture.ts',...args],{cwd:fileURLToPath(root),windowsHide:true,stdio:['pipe','pipe','pipe']});
 let output='',errors='',timeout:ReturnType<typeof setTimeout>|undefined;child.stderr.on('data',b=>{errors+=b.toString();});
 const exited=new Promise<number|null>((resolve,reject)=>{child.once('error',reject);child.once('exit',resolve);});
 try{
  const summary:any=await new Promise((resolve,reject)=>{timeout=setTimeout(()=>reject(Error('CLI fixture summary timeout')),30000);child.stdout.on('data',b=>{output+=b.toString();for(const line of output.split(/\r?\n/)){if(line.startsWith('{')){try{const value=JSON.parse(line);if(value.bundlePath){clearTimeout(timeout);resolve(value);}}catch{}}}});child.once('exit',code=>reject(Error('CLI fixture exited before summary: '+code+'; '+errors)));});
  const bundle=JSON.parse(await readFile(summary.bundlePath,'utf8'));assert.equal(bundle.evidenceStatus,'simulated-game-facts');assert.equal(bundle.a1Runtime,undefined);assert.equal(bundle.c2Runtime,undefined);assert.equal(bundle.potionDiagnostic,true);assert.equal(bundle.diagnosticOnly,true);assert.equal(bundle.live.endpoint,'/v3/live');assert.equal(summary.simulatedKeepalive,true);
  assert.ok(bundle.origin.startsWith('https://127.0.0.1:'));assert.ok(bundle.caPath);assert.equal(bundle.serviceId,'nt2-g3-c2-potion-diagnostic');
  assert.equal(bundle.games.length,2);assert.equal(bundle.scopes.length,2);assert.ok(bundle.host&&bundle.desktops&&bundle.room);assert.equal(summary.state,'simulated-playing');
  const profile=bundle.nativeSocialDevelopmentProfile;assert.equal(profile.id,'nt2-g3-c2-potion-diagnostic-1');assert.equal(profile.installApproved,false);assert.equal(profile.requiredLocalFiles.length,22);assert.deepEqual(bundle.social.runtimeProfile,profile.profile);assert.deepEqual(profile.runtimeApproval,nativePotionDiagnosticRuntimeResource);assert.deepEqual(profile.desktopApproval,nativePotionDiagnosticSourceResource);assert.equal(profile.profile.modVersion,'nt2-g3-c2-dev-1');assert.equal(profile.profile.bridgeVersion,'g3-c2-ipc-1');
  child.stdin.write('quit\n');assert.equal(await exited,0);await assert.rejects(access(summary.bundlePath));await assert.rejects(access(summary.caPath));assert.ok(!output.includes(bundle.games[0].accessToken));
 }finally{clearTimeout(timeout);if(child.exitCode===null){child.stdin.end('quit\n');await exited;}}
});
