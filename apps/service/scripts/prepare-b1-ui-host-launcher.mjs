// Prepare a candidate only. Root owns stable testing launcher publication.
import fs from 'node:fs';import {createHash} from 'node:crypto';
const hash=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const old=JSON.parse(fs.readFileSync('docs/g3/server-b1-package-dependency.json'));
const folder='deploy/g3-rejoin/b1-ui-host-runtime-20261025',manifest=JSON.parse(fs.readFileSync(folder+'/manifest.json'));
let script=fs.readFileSync('deploy/g3-rejoin/Start-Host.B1.candidate.ps1','utf8');
for(const [from,to]of [[old.runtimeApproval.sha256,manifest.runtimeApproval.sha256],[old.manifest.sha256,hash(folder+'/manifest.json')],[old.bootstrap.sha256,hash(folder+'/boot.mjs')],['b1-','b1-ui-'],['.Count -ne 182','.Count -ne 219']])script=script.replaceAll(from,to);
fs.writeFileSync('deploy/g3-rejoin/Start-Host.B1-UI.candidate.ps1',script);
console.log(JSON.stringify({candidate:'deploy/g3-rejoin/Start-Host.B1-UI.candidate.ps1',runtimeId:'nt2-g3-b1-ui-dev-20261025',published:false}));
