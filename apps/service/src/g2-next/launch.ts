import {readFile} from 'node:fs/promises';
import {dirname,resolve} from 'node:path';
import {createHash,generateKeyPairSync,randomBytes} from 'node:crypto';
import {createServer} from 'node:net';
import {lanConfig,validateLanConfig,localInterface,validateCertificate} from '../g2/lan.js';
import {inspectionMetadata,verifyInspectionArtifacts} from './profile.js';
import {schema} from '../../../../packages/contracts/src/g2-next.js';
const root=new URL('../../../../',import.meta.url);
export function requireCandidateMock(env:NodeJS.ProcessEnv=process.env){if(env.NT2_G2_NEXT_MOCK!=='1'||env.NODE_ENV==='production')throw Error('Successor launcher requires NT2_G2_NEXT_MOCK=1 and refuses production');}
export function candidateLanConfig(ip:unknown,port:unknown){
 const base=lanConfig(ip,port);if([4351,4352,4355].includes(base.port))throw Error('Successor port must be separate from working and existing fixture listeners');
 return {...base,serviceId:`nt2-g2-next-lan-${base.ip.replaceAll('.','-')}-${base.port}`,namespace:`g2_next_lan_${base.ip.replaceAll('.','_')}_${base.port}`};
}
export function validateCandidateLaunch(value:unknown){
 if(!value||typeof value!=='object'||Array.isArray(value))throw Error('Invalid successor launch configuration');const v=value as Record<string,unknown>,c=candidateLanConfig(v.ip,v.port);
 const expected={...c,profileId:inspectionMetadata().profile.id,contractRevision:schema.revision,contractSha256:'f31bfb01f82ab8b2ceb366f0cbd31e137555f035a1747de96a282bb30b3fbe21',tlsSourceConfig:v.tlsSourceConfig};
 if(typeof v.tlsSourceConfig!=='string'||!v.tlsSourceConfig||Object.keys(v).sort().join()!==Object.keys(expected).sort().join()||Object.entries(expected).some(([k,x])=>v[k]!==x))throw Error('Successor origin/identity/namespace/profile/contract must match canonical isolated configuration');
 return {...expected,tlsSourceConfig:v.tlsSourceConfig};
}
export async function ensureUnusedPort(ip:string,port:number){
 const probe=createServer();await new Promise<void>((accept,reject)=>{probe.once('error',()=>reject(Error('Configured successor port is unavailable; no service was started')));probe.listen({host:ip,port,exclusive:true},()=>probe.close(error=>error?reject(error):accept()));});
}
export async function prepareCandidateLaunch(configPath:string){
 requireCandidateMock();const config=validateCandidateLaunch(JSON.parse(await readFile(configPath,'utf8')));localInterface(config.ip);
 const actualHash=createHash('sha256').update(await readFile(new URL('packages/contracts/g2-next/schema.json',root))).digest('hex');if(actualHash!==config.contractSha256)throw Error('Pinned successor contract hash changed');
 const provenance=await verifyInspectionArtifacts(),sourcePath=resolve(dirname(resolve(configPath)),config.tlsSourceConfig),source=validateLanConfig(JSON.parse(await readFile(sourcePath,'utf8')));
 if(source.ip!==config.ip||source.port===config.port||source.namespace===config.namespace||source.serviceId===config.serviceId)throw Error('TLS source must be the existing separate same-IP LAN configuration');
 const publicFolder=new URL('deploy/g2-lan/public/',root),bundle=JSON.parse(await readFile(new URL('bundle.json',publicFolder),'utf8'));
 if(bundle.origin!==source.origin||bundle.serviceId!==source.serviceId)throw Error('Existing public TLS bundle does not match its source configuration');
 const ca=await readFile(new URL('ca.cer',publicFolder));if(createHash('sha256').update(ca).digest('hex')!==bundle.caSha256)throw Error('Existing public CA fingerprint differs');
 validateCertificate(await readFile(new URL('server.cer',publicFolder)),ca,config.ip);
 const secrets=dirname(sourcePath),tlsSecret=JSON.parse(await readFile(resolve(secrets,'tls.json'),'utf8'));if(typeof tlsSecret.passphrase!=='string'||tlsSecret.passphrase.length<32)throw Error('Invalid existing TLS secret');
 const tls={pfx:await readFile(resolve(secrets,'server.pfx')),passphrase:tlsSecret.passphrase};
 await ensureUnusedPort(config.ip,config.port);
 // Fresh successor-only signing/recovery keys stay in memory; never reuse/export working service keys.
 const pair=generateKeyPairSync('ed25519'),keys={privateKey:pair.privateKey.export({format:'pem',type:'pkcs8'}).toString(),publicKey:pair.publicKey.export({format:'pem',type:'spki'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:config.issuer};
 return {config,provenance,options:{databaseUrl:process.env.NT2_G2_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',namespace:config.namespace,serviceId:config.serviceId,keys,mockEnabled:true,website:true,tls,canonicalOrigin:config.origin,...inspectionMetadata()}};
}
