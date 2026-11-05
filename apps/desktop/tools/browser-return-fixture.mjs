// Client-owned isolated browser proof. Never launches/reconfigures the working LAN app.
import {readFile,writeFile} from 'node:fs/promises';
import {generateKeyPairSync,randomBytes,randomUUID} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import pg from 'pg';
import {createCandidateApp} from '../../../apps/service/src/g2-next/app.ts';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../..');
// Use the repository's existing TS loader against source, never stale/shared dist files.
const loopback=process.argv.includes('--loopback');
const coreTest=process.argv.includes('--core-test');
if(coreTest&&!loopback)throw Error('Core fault fixture is loopback only');
let clockOffset=0;
const host=loopback?'127.0.0.1':'192.168.0.104';
const origin=loopback?'http://127.0.0.1:4355':'https://192.168.0.104:4355';
const secret=path.join(root,'secrets/g2-lan-192-168-0-104-4352');
// Reuse the existing local leaf in memory; no export, regeneration or trust change.
let tls;
if(!loopback){
 const {passphrase}=JSON.parse(await readFile(path.join(secret,'tls.json'),'utf8'));
 tls={pfx:await readFile(path.join(secret,'server.pfx')),passphrase};
}
const pair=generateKeyPairSync('ed25519');
const namespace='g2_next_client_browser_'+randomUUID().replaceAll('-','');
const databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
let service;
const audit=[];
async function cleanup(){
  if(service){await service.app.close();service=undefined;}
  if(process.env.NT2_BROWSER_PROBE_AUDIT)await writeFile(process.env.NT2_BROWSER_PROBE_AUDIT,JSON.stringify(audit,null,2));
  if(!/^g2_next_client_browser_[a-f0-9]{32}$/.test(namespace))throw Error('Unsafe fixture namespace');
  const pool=new pg.Pool({connectionString:databaseUrl});
  try{await pool.query(`DROP SCHEMA IF EXISTS "${namespace}" CASCADE`);}finally{await pool.end();}
}
try{
 service=await createCandidateApp({databaseUrl,namespace,serviceId:'nt2-client-browser-candidate',mockEnabled:true,website:true,...(tls?{tls,canonicalOrigin:origin}:{}),...(coreTest?{now:()=>Date.now()+clockOffset}:{}),keys:{privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}),publicKey:pair.publicKey.export({type:'spki',format:'pem'}),recoveryKey:randomBytes(32).toString('base64url'),issuer:origin},compatibility:{gameVersion:coreTest?'Noita-Jan25-2025':'browser-test',modVersion:coreTest?'nt2-g2-inspect-1':'browser-test',bridgeVersion:coreTest?'g2-inspect-1':'browser-test',capabilities:['core.seed'],modes:['core'],settingKeys:[]}});
 if(coreTest)service.app.post('/fixture/advance',async req=>{const step=Number(req.headers['x-fixture-step']);if(!Number.isInteger(step)||step<1||step>20000)throw Error('Bounded fixture clock step required');clockOffset+=step;return {advanced:true};});
 service.app.addHook('onResponse',async(req,reply)=>{if(audit.length<32)audit.push({method:req.method,route:req.url.startsWith('/signin/')?'signin':req.url,status:reply.statusCode,originMatched:req.headers.origin===origin,originNull:req.headers.origin==='null',cookiePresent:!!req.headers.cookie});});
 await service.app.listen({port:4355,host});
 process.stdout.write('ISOLATED_BROWSER_FIXTURE_READY\n');
 // Owner's redirected stdin closes only this fixture and its generated namespace.
 process.stdin.resume();
 await new Promise(resolve=>{process.stdin.once('end',resolve);process.stdin.once('data',resolve);process.once('SIGTERM',resolve);process.once('SIGINT',resolve);});
}finally{await cleanup();}
