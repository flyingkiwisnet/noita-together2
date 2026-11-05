// Rebuild canonical vectors from sanitized service evidence, with explicitly
// synthetic request proofs. Never copy real credentials or proof stores.
import {readFile,writeFile} from 'node:fs/promises';
const dir=new URL('../g3-waiting/',import.meta.url),docs=new URL('../../../docs/g3/',import.meta.url);
const load=async name=>JSON.parse((await readFile(new URL(name,docs),'utf8')).replace(/^\uFEFF/,''));
const schema=JSON.parse(await readFile(new URL('schema.json',dir),'utf8'));
let fixtures=JSON.parse(await readFile(new URL('fixture-inputs.json',dir),'utf8')).filter(f=>!f.label?.startsWith('startup-repair'));
for(const f of fixtures)if(['Request','Response'].includes(f.type))f.json.revision=schema.revision;
const pending=await load('server-startup-repair-pending-fixture.json'),selected=await load('server-startup-repair-selected-true-fixture.json'),excluded=await load('server-startup-repair-excluded-fixture.json');
const add=(type,json,label)=>fixtures.push({type,json,label:'startup-repair '+label});
const p=pending.reserve.pending,i=p.intent;
add('PendingState',p,'pre-generation unknown individual seed');add('WaitingState',pending.recovered.waiting,'protected no-play after recovery');
const states=await load('server-startup-repair-preparation-state-fixture.json');
for(const state of ['issued','consumed','loaded'])add('PendingState',states[state],'current preparation '+state);
add('Response',pending.reserve,'reserve response no proof');add('Response',pending.recovered,'recovery response sanitized');add('Response',pending.outcome,'current-auth sanitized original outcome');
const base={major:1,revision:schema.revision,serviceId:i.serviceId,serviceEpoch:i.serviceEpoch,sessionEpoch:i.originalDesktopEpoch,requestId:'strict-startup-repair-fixture'};
add('Request',{...base,reserve:{preparationId:i.preparationId,worldToken:i.worldToken,contextId:i.contextId}},'reserve before seed');
add('Request',{...base,pendingId:i.pendingId,pendingStatus:{}},'status');
add('Request',{...base,pendingOutcome:pending.outcome.pendingOutcome.original},'exact outcome');
add('Request',{...base,pendingId:i.pendingId,generation:1,pendingRecover:{pendingProof:'s'.repeat(43),contextId:'fresh-contact',gameVersion:'synthetic-game',modVersion:'synthetic-mod',bridgeVersion:'synthetic-bridge',capabilities:[],observation:{gameInstanceId:'synthetic-contact',worldToken:i.worldToken,observationRevision:1,actualSeed:'18446744073709551615',seedStatus:'known',activeMods:['noita_together2'],inventoryComplete:true,collectorFailure:'',beta:'unknown',protected:true,loaded:true,modVersion:'synthetic-mod',bridgeVersion:'synthetic-bridge',compatibilityProvenance:'game-thread-client-observation'}}},'recover before attachment known');
add('Request',{...base,pendingId:i.pendingId,generation:1,pendingOffline:{pendingProof:'s'.repeat(43)}},'offline');
add('Response',{...base,requestId:selected.pending.sample.originRequestId,pending:selected.pending,waiting:selected.waiting,participant:selected.participant,canonicalAdmission:selected.canonicalAdmission},'selected existing Start admission');
add('Response',{...base,requestId:excluded.pending.sample.originRequestId,pending:excluded.pending,waiting:excluded.waiting,error:{code:'NOT_SELECTED_BY_START',retryAfterMs:0}},'excluded remains unavailable');
const replacement=await load('server-startup-repair-replacement-fixture.json'),opaque=await load('server-startup-repair-opaque-lost-reply-fixture.json');
add('PendingState',replacement.replacement,'distinct issued replacement');add('PendingState',replacement.retired,'superseded original terminal');
add('PendingOutcome',opaque.originalAttach,'opaque original attach effect is historical');
for(const snapshot of opaque.snapshots)add('WaitingState',snapshot.waiting,'public current owner after lost recovery reply');
await writeFile(new URL('fixture-inputs.json',dir),JSON.stringify(fixtures,null,2)+'\n');
