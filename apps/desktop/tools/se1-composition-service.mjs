// Client-owned lifecycle/control wrapper. Canonical service, schema allocation,
// peer HTTP and response faults are supplied by the reviewed Server fixture.
import {startConnectionCr2Fixture} from '../../service/scripts/connection-cr2-fixture.ts';
import {appendFile,readFile,writeFile,stat} from 'node:fs/promises';
import {resolve,relative,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import readline from 'node:readline';
import {createHash} from 'node:crypto';
const repo=resolve(fileURLToPath(new URL('../../..',import.meta.url)));
const root=resolve(process.env.NT2_SE1_COMPOSITION_ROOT??'');
const owned=resolve(repo,'spikes/noita-entry/.build/g3-startup-entry-se1-composition-20261017-01');
if(process.env.NT2_SE1_COMPOSITION_EXECUTE!=='1'||!/^run-[a-f0-9]{32}$/.test(relative(owned,root)))throw Error('Explicit Root execution and fresh owned output required');
const emit=value=>process.stdout.write(JSON.stringify(value)+'\n');
const ledger=join(root,'service-lifecycle.jsonl');
let allocated,absent;const originalLog=console.log;
console.log=(...args)=>{for(const item of args){let v;try{v=typeof item==='string'?JSON.parse(item):item;}catch{continue}if(v?.allocatedFixtureSchema){if(!/^g3_rejoin_[a-f0-9]{32}$/.test(v.allocatedFixtureSchema)||v.databaseTarget!=='127.0.0.1:54328/nt2_g1')throw Error('Unexpected schema/database');allocated=v.allocatedFixtureSchema;}if(v?.verifiedAbsentSchema){if(v.verifiedAbsentSchema!==allocated)throw Error('Cleanup names another schema');absent=v.verifiedAbsentSchema;}if(v?.allocatedFixtureSchema||v?.verifiedAbsentSchema)emit(v);}};
let fixture,scenario,interval,watchdog,lines,faults,pulse=Promise.resolve(),advance=0,quitting=false;
process.stdout.on('error',()=>{quitting=true;lines?.close();});
function faultControl(f){
 if(typeof f.armEntryActivationReply!=='function'||typeof f.entryActivationReplySnapshot!=='function'||typeof f.releaseEntryActivationReply!=='function')throw Error('Exact Server entry reply fault seam unavailable');
 let arm;
 return {
  async arm(plan){arm=await f.armEntryActivationReply(plan);return arm;},
  async status(){const snapshot=f.entryActivationReplySnapshot();await writeFile(join(root,'fault-events.json'),JSON.stringify(snapshot,null,2));const events=snapshot.events.filter(e=>e.armId===arm?.armId&&e.pendingId===arm.pendingId&&e.worldToken===arm.worldToken);const committed=events.some(e=>e.kind==='committed');const applied=arm?.mode==='drop'?events.some(e=>e.kind==='dropped'):snapshot.active?.armId===arm?.armId&&snapshot.active?.phase==='delaying'&&events.some(e=>e.kind==='delayed');return {committed:committed&&applied};},
  release(){if(!arm||!f.releaseEntryActivationReply(arm.armId)||!f.entryActivationReplySnapshot().events.some(e=>e.armId===arm.armId&&e.pendingId===arm.pendingId&&e.worldToken===arm.worldToken&&e.kind==='released'))throw Error('Exact delayed original was not released');},
  async evidence(){await writeFile(join(root,'fault-events.json'),JSON.stringify(f.entryActivationReplySnapshot(),null,2));}
 };
}
try{
 // Pin and approve the concrete Server seam before starting any schema/service.
 const dependency=JSON.parse(await readFile(join(repo,'docs/g3/server-se1-composition-fixture-dependency.json'),'utf8'));
 if(!Array.isArray(dependency.sourcePins)||dependency.sourcePins.length<2)throw Error('Pinned Server fixture dependency missing');
 for(const pin of dependency.sourcePins){const source=resolve(repo,pin.path);if(relative(repo,source).startsWith('..')||!/^[a-f0-9]{64}$/.test(pin.sha256)||createHash('sha256').update(await readFile(source)).digest('hex')!==pin.sha256)throw Error('Server fixture source drift');}
 ({fixture,scenario}=await startConnectionCr2Fixture({now:()=>Date.now()+advance}));
 faults=faultControl(fixture);
 await appendFile(ledger,JSON.stringify({event:'created',namespace:fixture.namespace,pid:process.pid})+'\n');
 await scenario.start();await scenario.playing();
 const peerPulse=()=>pulse=pulse.then(()=>scenario.pulse());
 interval=setInterval(()=>{void peerPulse().catch(()=>{quitting=true;lines?.close();});},2000);
 lines=readline.createInterface({input:process.stdin});const end=Date.now()+330000;
 watchdog=setInterval(()=>{void stat(join(root,'quit.request')).then(()=>{quitting=true;lines.close();}).catch(()=>{});if(Date.now()>=end){quitting=true;lines.close();}},250);
 emit({event:'ready',origin:fixture.origin,caPath:fixture.caPath,databaseTarget:'127.0.0.1:54328/nt2_g1',entryRevision:'g3-waiting-entry-candidate-3',postCommitReplyFault:true});
 for await(const line of lines){
  if(line==='quit')break;if(quitting)break;
  if(line==='peer-check'){await peerPulse();emit({event:line,healthy:true});}
  else if(line.startsWith('advance:')){let left=Number(line.slice(8));if(!Number.isInteger(left)||left<0||left>120000)throw Error('Bounded owned clock control only');while(left){const step=Math.min(left,10000);advance+=step;left-=step;await peerPulse();}emit({event:line});}
  else if(/^arm-(drop|delay):[a-f0-9-]{36}:[a-f0-9]{32}$/.test(line)){const [kind,pendingId,worldToken]=line.split(':');await faults.arm({pendingId,worldToken,...(kind==='arm-drop'?{mode:'drop'}:{mode:'delay',delayMs:10000})});emit({event:line});}
  else if(line==='entry-fault-status'){const status=await faults.status();emit({event:line,fired:status.committed===true,postCommit:status.committed===true});}
  else if(line==='release-entry-fault'){await faults.release();emit({event:line});}
  else throw Error('Unknown bounded fixture control');
 }
}catch{
 emit({event:'error',reason:'SE1 fixture/control failure; canonical output never fabricated'});process.exitCode=1;
}finally{
 clearInterval(interval);clearInterval(watchdog);lines?.close();await pulse.catch(()=>{});
 try{await fixture?.close();}catch{process.exitCode=1;}
 await faults?.evidence();
 console.log=originalLog;
 const cleanup={cooperativeExit:true,namespace:allocated??null,verifiedAbsentSchema:!!allocated&&allocated===absent,processTerminationAttempted:false};
 await appendFile(ledger,JSON.stringify({event:'closed',...cleanup})+'\n');await writeFile(join(root,'service-cleanup.json'),JSON.stringify(cleanup,null,2));
 if(allocated&&!cleanup.verifiedAbsentSchema)process.exitCode=1;
}
