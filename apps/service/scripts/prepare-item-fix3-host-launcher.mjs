// Candidate only. Root owns stable testing launcher publication.
import fs from 'node:fs';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';
const count=Number(process.argv[2]);assert.ok(Number.isInteger(count)&&count>256&&count<=512,'Client exact kit artifact count required');
const hash=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const old=JSON.parse(fs.readFileSync('docs/g3/server-b1-ui-package-dependency-20261025.json'));
const folder='deploy/g3-rejoin/items-fix3-host-runtime-20261025',manifest=JSON.parse(fs.readFileSync(folder+'/manifest.json'));
let script=fs.readFileSync('deploy/g3-rejoin/Start-Host.B1-UI.candidate.ps1','utf8');
for(const [from,to]of [[old.runtimeApproval.sha256,manifest.runtimeApproval.sha256],[old.manifest.sha256,hash(folder+'/manifest.json')],[old.bootstrap.sha256,hash(folder+'/boot.mjs')],['b1-ui','items'],['.Count -ne 219','.Count -ne 252'],['$taskManifest.Count -lt 40 -or $taskManifest.Count -gt 256',`$taskManifest.Count -ne ${count}`],["core.cosmetics,core.gold'","core.cosmetics,core.gold,core.items'"]]){assert.ok(script.includes(from),'missing launcher replacement '+from);script=script.replaceAll(from,to);}
script=script.replaceAll('items-host-runtime-20261025','items-fix3-host-runtime-20261025').replaceAll('nt2-items-isolated-host-20261025','nt2-items-fix3-isolated-host-20261025').replaceAll('config.items-20261025','config.items-fix3-20261025').replaceAll('g3-items-runtime.20261025','g3-items-runtime-fix3.20261025');
assert.ok(script.includes(`$taskManifest.Count -ne ${count}`));
fs.writeFileSync('deploy/g3-rejoin/Start-Host.Items-Fix3.candidate.ps1',script,{flag:'wx'});
console.log(JSON.stringify({candidate:'deploy/g3-rejoin/Start-Host.Items-Fix3.candidate.ps1',runtimeId:'nt2-g3-items-dev-20261025',requiredRuntimeFiles:252,exactKitArtifactCount:count,published:false}));
