import {readFile} from 'node:fs/promises';
import {createG3CandidateApp} from './candidate.js';
import type {Keys} from '../g1/security.js';
if(process.env.NT2_G3_MOCK!=='1'||process.env.NODE_ENV==='production')throw Error('G3 launcher requires NT2_G3_MOCK=1 and refuses production');
const namespace=process.env.NT2_G3_NAMESPACE;
if(!namespace||!/^g3_candidate_dev_[a-z0-9_]{1,35}$/.test(namespace))throw Error('Set an explicit separate g3_candidate_dev_ namespace');
const profile=JSON.parse(await readFile(new URL('../../../../deploy/profiles/g3-simulated-candidate.json',import.meta.url),'utf8'));
const keys=JSON.parse(await readFile(process.env.NT2_G3_KEYS_PATH??'secrets/g3-dev.json','utf8')) as Keys;
if(keys.issuer!==profile.issuerExample)throw Error('G3 keys must bind only the separate loopback4360 issuer');
const {app}=await createG3CandidateApp({databaseUrl:process.env.NT2_G3_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',serviceId:profile.serviceId,namespace,keys,mockEnabled:true,website:true,compatibility:profile.compatibility});
try{await app.listen({host:'127.0.0.1',port:4360});}catch(e){await app.close();throw e;}
for(const signal of ['SIGINT','SIGTERM'] as const)process.once(signal,()=>{void app.close();});
console.log('G3 simulated candidate on loopback4360; gameplay is fixture-only; statistics at /statistics.');
