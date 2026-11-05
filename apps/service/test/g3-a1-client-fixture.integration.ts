import test from 'node:test';import assert from 'node:assert/strict';import {spawn} from 'node:child_process';import {readFile,access} from 'node:fs/promises';import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url);
for(const a1 of [false,true])for(const social of [false,true])test('CLI fixture preserves '+(a1?'A1':'old')+' '+(social?'social':'primary')+' bundle fields and isolated cleanup',async()=>{
 const args=['--tls','--playing',social?'--native-social':'--native',...(a1?['--a1-runtime']:[])];
 const child=spawn(process.execPath,[fileURLToPath(new URL('node_modules/tsx/dist/cli.mjs',root)),'apps/service/scripts/g3-client-fixture.ts',...args],{cwd:fileURLToPath(root),windowsHide:true,stdio:['pipe','pipe','pipe']});
 let output='',errors='',timeout:ReturnType<typeof setTimeout>|undefined;child.stderr.on('data',b=>{errors+=b.toString();});
 const exited=new Promise<number|null>((resolve,reject)=>{child.once('error',reject);child.once('exit',resolve);});
 try{
  const summary:any=await new Promise((resolve,reject)=>{timeout=setTimeout(()=>reject(Error('CLI fixture summary timeout')),30000);child.stdout.on('data',b=>{output+=b.toString();for(const line of output.split(/\r?\n/)){if(line.startsWith('{')){try{const value=JSON.parse(line);if(value.bundlePath){clearTimeout(timeout);resolve(value);}}catch{}}}});child.once('exit',code=>reject(Error('CLI fixture exited before summary: '+code+'; '+errors)));});
  const bundle=JSON.parse(await readFile(summary.bundlePath,'utf8'));assert.equal(bundle.evidenceStatus,'simulated-game-facts');assert.equal(bundle.a1Runtime,a1?true:undefined);
  assert.ok(bundle.origin.startsWith('https://127.0.0.1:'));assert.ok(bundle.caPath);assert.equal(bundle.serviceId,a1?(social?'nt2-g3-native-social-development-a1':'nt2-g3-native-development-a1'):(social?'nt2-g3-native-social-development':'nt2-g3-native-development'));
  assert.equal(bundle.games.length,2);assert.equal(bundle.scopes.length,2);assert.ok(bundle.host&&bundle.desktops&&bundle.room);assert.equal(summary.state,'simulated-playing');
  const profile=social?bundle.nativeSocialDevelopmentProfile:bundle.nativeDevelopmentProfile;assert.equal(profile.id,social?(a1?'nt2-g3-native-social-development-2':'nt2-g3-native-social-development-1'):(a1?'nt2-g3-native-development-3':'nt2-g3-native-development-2'));assert.equal(profile.installApproved,false);
  if(a1){assert.equal(profile.runtimeApproval.sha256,social?'b0ce032108612ff1d2540008ae9a96f746748243857d5d6efaa8537b7be934c0':'959fddf3a0d5948709e685fdfda325de96004d98b798e377b70f65b5ad8502dd');assert.equal(profile.desktopApproval.sha256,'7e44483c30817c317830735f00e3ce04a5e16653493c81a923886afec51ad319');}
  assert.equal(profile.requiredLocalFiles.length,social?20:14);if(social)assert.deepEqual(bundle.social.runtimeProfile,profile.profile);
  child.stdin.write('quit\n');assert.equal(await exited,0);await assert.rejects(access(summary.bundlePath));await assert.rejects(access(summary.caPath));assert.ok(!output.includes(bundle.games[0].accessToken));
 }finally{clearTimeout(timeout);if(child.exitCode===null){child.stdin.end('quit\n');await exited;}}
});
