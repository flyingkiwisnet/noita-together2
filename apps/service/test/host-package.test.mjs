import test from 'node:test';import assert from 'node:assert/strict';
import fs from 'node:fs/promises';import path from 'node:path';import os from 'node:os';import {createHash,randomUUID} from 'node:crypto';import {execFile} from 'node:child_process';import {promisify} from 'node:util';
const run=promisify(execFile),sha=b=>createHash('sha256').update(b).digest('hex');
const candidate=path.resolve(process.env.NT2_HOST_PACKAGE??'.build/host-package-ready/host');
test('Runtime host closure, single verifier and fail-closed inventory work independently of checkout dependencies',async()=>{
 const scratch=await fs.mkdtemp(path.join(os.tmpdir(),'nt2-host-package-')),host=path.join(scratch,'host');
 try{
  await fs.cp(candidate,host,{recursive:true});const manifestPath=path.join(host,'manifest.json'),original=await fs.readFile(manifestPath),m=JSON.parse(original);
  const verify=(expected=sha(original),config=path.join(host,'config.json'))=>run(process.execPath,[path.join(host,'boot.mjs'),expected,config,'--verify-only'],{cwd:os.tmpdir(),windowsHide:true,maxBuffer:65536});
  const success=await verify();assert.match(success.stdout,/"verified":true/);assert.match(success.stdout,/"databaseTouched":false/);assert.equal((success.stdout.match(/Host inventory:/g)??[]).length,1);
  assert.ok(m.files.length<1500);assert.equal(m.files.filter(p=>p.path.startsWith('deploy/profiles/')).length,2);
  for(const p of m.files){assert.doesNotMatch(p.path,/\.(ts|tsx|map)$/);assert.doesNotMatch(p.path,/(^|\/)(test|tests|examples|\.github|coverage)\//);}
  for(const {name}of m.thirdPartyDependencies)assert.ok(m.files.some(p=>p.path.startsWith('node_modules/'+name+'/')&&(/\/(LICENSE|LICENCE|COPYING|NOTICE)(\.|$)/i.test(p.path)||/\/readme(?:\.|$)/i.test(p.path))),'license retained '+name);
  await assert.rejects(verify('0'.repeat(64)),/manifest mismatch/);
  const asset=path.join(host,'apps/web/candidate/client.js'),bytes=await fs.readFile(asset);await fs.appendFile(asset,'\n// tampered');await assert.rejects(verify(),/asset mismatch/);await fs.writeFile(asset,bytes);
  const moved=path.join(scratch,'missing.js');await fs.rename(asset,moved);await assert.rejects(verify(),/Missing host asset/);await fs.rename(moved,asset);
  const unlisted=path.join(host,'unexpected.js');await fs.writeFile(unlisted,'');await assert.rejects(verify(),/Unexpected host asset/);await fs.rename(unlisted,path.join(scratch,'unexpected.js'));
  for(const change of [v=>v.files.push(v.files[0]),v=>v.files.push({...v.files[0],path:'../escape'}),v=>v.files.push({...v.files[0],path:v.files[0].path.toUpperCase()}),v=>v.config.sha256='0'.repeat(64)]){const bad=structuredClone(m);change(bad);const b=Buffer.from(JSON.stringify(bad));await fs.writeFile(manifestPath,b);await assert.rejects(verify(sha(b)),/Invalid or duplicate|binding mismatch/);}await fs.writeFile(manifestPath,original);
  const link=path.join(host,'linked');await fs.symlink(scratch,link,process.platform==='win32'?'junction':'dir');await assert.rejects(verify(),/contains a link/);await fs.rename(link,path.join(scratch,'rejected-link'));
  const wrongConfig=path.join(scratch,'wrong-config.json');await fs.writeFile(wrongConfig,'{}');await assert.rejects(verify(sha(original),wrongConfig),/configuration mismatch/);
  // The actual generated PowerShell entry point works from an arbitrary cwd.
  const support=path.join(scratch,'support');await fs.mkdir(support);const launch=path.join(support,'Start-Host.ps1');await fs.copyFile(path.resolve(candidate,'../support/Start-Host.ps1'),launch);
  const launched=await run(process.platform==='win32'?'powershell.exe':'pwsh',['-NoProfile','-File',launch,'-VerifyOnly'],{cwd:os.tmpdir(),windowsHide:true,maxBuffer:65536});assert.match(launched.stdout,/"verified":true/);assert.equal((launched.stdout.match(/Host inventory:/g)??[]).length,1);
  console.log(JSON.stringify({files:m.files.length,closureOutsideCheckout:true,inventoryPasses:1,licenses:true,tamperCases:8,verifyOnly:success.stdout.trim()}));
 }finally{assert.ok(path.resolve(scratch).startsWith(path.resolve(os.tmpdir())+path.sep+'nt2-host-package-'));await fs.rm(scratch,{recursive:true,force:true});}
});
