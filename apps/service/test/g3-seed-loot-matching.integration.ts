import test from 'node:test';import assert from 'node:assert/strict';import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {nativeSeedLootRuntimeResource,nativeSeedLootSourceResource} from '../src/g3/native-seed-loot-profile.js';
import {spawn} from 'node:child_process';import {readFile,access} from 'node:fs/promises';import {fileURLToPath} from 'node:url';
for(const [seedPolicy,randomizeLoot] of [['shared-random',true],['shared-random',false],['shared-fixed',true],['individual-random',false]] as const)test('U2-S-PREP matched one-issuer automatic flow '+seedPolicy+' loot='+randomizeLoot,async()=>{
 let now=Date.now();const f=await startG3Fixture({seedLootRuntime:true,live:true,tls:true,now:()=>now});try{
  const info=JSON.parse((await f.call('/v1/info')).body);assert.equal(info.serviceId,'nt2-g3-c2-potion-correction');assert.equal(info.connectionFlowRevision,'g3-flow-1');assert.equal(info.profile.id,'nt2-g3-seed-loot-1');assert.equal(info.roomSemantics.kind,'core-social-seed-loot');assert.deepEqual(info.roomSemantics.requiredCapabilities,f.profile.compatibility.capabilities);
  const s=await provisionG3Fixture(f,{seedPolicy,randomizeLoot,beforeRpc:()=>{now+=300;}});assert.equal(s.previews.length,2);assert.equal(s.commandNames.filter(k=>k==='authBegin').length,2);assert.equal(s.commandNames.filter(k=>k==='authExchange').length,2);assert.equal(s.commandNames.filter(k=>k==='roomPreview').length,2);assert.equal(s.commandNames.filter(k=>k==='roomJoin').length,2);assert.ok(!s.commandNames.some(k=>k==='ready'||k==='prepare'));assert.ok(s.previews.every(p=>Object.keys(p).sort().join(',')==='config,configRevision,name,revision,roomId,state'));assert.ok(s.room.members!.every((m:any)=>m.ready&&m.included));assert.equal(s.room.config!.seedPolicy,seedPolicy);assert.equal(s.room.config!.randomizeLoot,randomizeLoot);
  assert.ok(s.desktops.every(d=>d.kind==='desktop'&&d.principal?.kind==='registered'));assert.equal(s.room.state,'staging');assert.ok(seedPolicy==='individual-random'?s.reports[0]!.actualSeed!==s.reports[1]!.actualSeed:s.reports.every(r=>r.actualSeed===String(s.room.config!.seed)));
  const resource=await f.call('/v3/social/runtime');assert.equal(resource.body,f.profile.runtimeResourceJson);const social=JSON.parse((await f.call('/v3/social/info')).body);assert.equal(social.runtimeProfile.id,f.profile.id);assert.deepEqual(social.runtimeApproval,nativeSeedLootRuntimeResource);assert.deepEqual(social.desktopApproval,nativeSeedLootSourceResource);
  await s.start();assert.ok(s.room.members!.every((m:any)=>m.selected));for(const [i,g] of s.games.entries()){const r=(await s.rpc(g,{roomGet:{}})).release!;assert.equal(r.seedPolicy,seedPolicy);assert.equal(r.randomizeLoot,randomizeLoot);assert.equal(r.actualSeed,s.observations[i]!.actualSeed);assert.equal(r.serviceId,info.serviceId);}
  await s.playing();assert.equal(s.room.state,'running');await s.pulse();
 }finally{await f.close();}
});
test('U2-S-PREP matching selector refuses mixed/nonTLS native profile before any fixture database work',async()=>{
 await assert.rejects(startG3Fixture({seedLootRuntime:true,live:true}),/C2 runtime/);await assert.rejects(startG3Fixture({seedLootRuntime:true,potionCorrection:true,live:true,tls:true}),/Separate correction/);
});
test('U2-S-PREP CLI private exact34 bundle is consumable and cleans temporary credentials/TLS',async()=>{
 const root=new URL('../../../',import.meta.url),child=spawn(process.execPath,[fileURLToPath(new URL('node_modules/tsx/dist/cli.mjs',root)),'apps/service/scripts/g3-client-fixture.ts','--seed-loot-runtime','--seed-policy','individual-random','--live','--tls','--playing'],{cwd:fileURLToPath(root),windowsHide:true,stdio:['pipe','pipe','pipe']});let output='',errors='',timer:ReturnType<typeof setTimeout>|undefined;child.stderr.on('data',b=>errors+=b.toString());const exit=new Promise<number|null>((resolve,reject)=>{child.once('error',reject);child.once('exit',resolve);});
 try{const summary:any=await new Promise((resolve,reject)=>{timer=setTimeout(()=>reject(Error('CLI summary timeout; '+errors)),30000);child.stdout.on('data',b=>{output+=b.toString();for(const line of output.split(/\r?\n/))if(line.startsWith('{')){try{const s=JSON.parse(line);if(s.bundlePath){clearTimeout(timer);resolve(s);}}catch{}}});child.once('exit',code=>reject(Error('CLI ended '+code+'; '+errors)));});
  const bundle=JSON.parse(await readFile(summary.bundlePath,'utf8'));assert.equal(bundle.seedLootRuntime,true);assert.equal(bundle.evidenceStatus,'simulated-game-facts');assert.equal(bundle.serviceId,'nt2-g3-c2-potion-correction');assert.equal(bundle.room.config.seedPolicy,'individual-random');assert.equal(bundle.nativeSocialDevelopmentProfile.requiredLocalFiles.length,34);assert.deepEqual(bundle.nativeSocialDevelopmentProfile.runtimeApproval,nativeSeedLootRuntimeResource);assert.ok(!output.includes(bundle.games[0].accessToken));
  child.stdin.write('quit\n');assert.equal(await exit,0);await assert.rejects(access(summary.bundlePath));await assert.rejects(access(summary.caPath));
 }finally{clearTimeout(timer);if(child.exitCode===null){child.stdin.end('quit\n');await exit;}}
});
// @ts-expect-error Production browser controller intentionally has no Node type layer.
import {createHostController} from '../../web/candidate/app.js';
// @ts-expect-error Execute production browser client against real TLS handler.
import {HostClient,ServiceError} from '../../web/candidate/client.js';
// @ts-expect-error DOM adapter only; no authority injected.
import {controllerDom} from '../../web/test/dom-fixture.mjs';
import {randomUUID} from 'node:crypto';
test('U2-S-PREP actual website initializes exact six-capability semantics and commits default shared random/loot then individual draft',async()=>{
 let now=Date.now();const f=await startG3Fixture({seedLootRuntime:true,live:true,tls:true,now:()=>now});let controller:any;
 try{const calls:any[]=[],transport=async(path:string,body?:any,token?:string)=>{now+=300;if(body?.roomCreate||body?.configure)calls.push(structuredClone(body));const r=await f.call(path,body,token);assert.equal(r.status,200);const value=JSON.parse(r.body);if(value.error)throw new ServiceError(value.error.code);return value;};
  const client=new HostClient(transport,randomUUID),dom=await controllerDom('../candidate/index.html');controller=createHostController({...dom,client,schedule:()=>{}});await controller.ready;assert.ok(client.schema,dom.get('notice').textContent);await dom.click('mock-website');assert.ok(client.credentials);
  assert.equal(dom.get('create').elements.seedPolicy.value,'shared-random');assert.equal(dom.get('create').elements.randomizeLoot.checked,true);await dom.submit('create',{name:'Matched policy website',mode:'core',visibility:'public'});assert.equal(dom.get('notice').dataset.error,'false',dom.get('notice').textContent);
  const roomId=dom.get('invite').value,room=(await client.rpc({roomId,roomGet:{}})).room;assert.deepEqual(room.config.requiredCapabilities,f.profile.compatibility.capabilities);assert.ok(room.config.seed>0);assert.equal(room.config.seedPolicy,'shared-random');assert.equal(room.config.randomizeLoot,true);assert.equal(calls[0].roomCreate.config.seed,0);
  const count=calls.length;await dom.click('configure-random');assert.equal(calls.length,count);assert.equal(dom.get('configure').elements.seed.value,0);
  dom.get('configure').elements.randomizeLoot.checked=false;await dom.submit('configure',{seed:0,seedPolicy:'individual-random',mode:'core'});assert.equal(dom.get('notice').dataset.error,'false',dom.get('notice').textContent);const updated=(await client.rpc({roomId,roomGet:{}})).room;assert.equal(updated.config.seedPolicy,'individual-random');assert.equal(updated.config.randomizeLoot,false);assert.match(dom.get('scope').textContent,/Separate random worlds/);
 }finally{controller?.dispose();await f.close();}
});
