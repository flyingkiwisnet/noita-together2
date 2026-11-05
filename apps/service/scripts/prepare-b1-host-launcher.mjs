// Prepare a candidate only. Root owns stable testing launcher publication.
import fs from 'node:fs';import {createHash} from 'node:crypto';
const hash=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const old=JSON.parse(fs.readFileSync('docs/g3/server-a3-debug-package-dependency-20261023.json'));
const folder='deploy/g3-rejoin/b1-host-runtime-20261025',manifest=JSON.parse(fs.readFileSync(folder+'/manifest.json'));
let script=fs.readFileSync('deploy/g3-rejoin/Start-Host.A3-Debug.candidate.ps1','utf8');
for(const [from,to]of [[old.runtimeApproval.sha256,manifest.runtimeApproval.sha256],[old.manifest.sha256,hash(folder+'/manifest.json')],[old.bootstrap.sha256,hash(folder+'/boot.mjs')],['a3-debug','b1'],['20261023','20261025'],['candidate23 October A3 Debug fixes and diagnostics','candidate25 October B1 gold bank'],['.Count -ne 170','.Count -ne 182'],["core.emotes,core.cosmetics'","core.emotes,core.cosmetics,core.gold'"]])script=script.replaceAll(from,to);
fs.writeFileSync('deploy/g3-rejoin/Start-Host.B1.candidate.ps1',script);
console.log(JSON.stringify({candidate:'deploy/g3-rejoin/Start-Host.B1.candidate.ps1',runtimeId:'nt2-g3-b1-dev-20261025',published:false}));
