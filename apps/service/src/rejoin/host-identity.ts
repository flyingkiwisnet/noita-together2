import {generateKeyPairSync,randomBytes} from 'node:crypto';import type {Keys} from '../g1/security.js';import {Security} from '../g1/security.js';import {RejoinDatabase} from './database.js';
// Invoked only by an explicit DEV host start, never check-only. Namespace guard
// prevents touching the prior LAN schema. Same DB retains identity on restart.
export async function persistentRejoinIdentity(databaseUrl:string,serviceId:string,namespace:string,issuer:string,configuredAuthentication=false):Promise<Keys>{
 if(!namespace.startsWith('g3_rejoin_')||process.env.NODE_ENV==='production'&&!configuredAuthentication)throw Error('Explicit new rejoin namespace and configured authentication required');
 const db=new RejoinDatabase(databaseUrl,serviceId,namespace,{},true);try{
  // The service still checks every migration before recovery. Existing hosts
  // need only their signer here; avoid traversing the full migration chain twice.
  const identity=await db.pool.query("SELECT to_regclass('rejoin_host_identity') AS identity");
  if(!identity.rows[0]?.identity)await db.migrate();
  const tx=await db.pool.connect();try{
  await tx.query('BEGIN');await tx.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-rejoin-host-identity:'+namespace]);
  let row=(await tx.query('SELECT * FROM rejoin_host_identity FOR UPDATE')).rows[0];if(!row){const pair=generateKeyPairSync('ed25519'),keys:Keys={privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}).toString(),publicKey:pair.publicKey.export({type:'spki',format:'pem'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer};row=(await tx.query('INSERT INTO rejoin_host_identity(service_id,issuer,keys) VALUES($1,$2,$3) RETURNING *',[serviceId,issuer,keys])).rows[0];}
  if(row.service_id!==serviceId||row.issuer!==issuer||row.keys?.issuer!==issuer)throw Error('Persistent host identity differs; never reset keys automatically');
  const keys=row.keys as Keys,security=new Security(keys,serviceId),now=Date.now(),token=await security.sign('identity-probe','identity-probe','identity-probe','desktop','identity-probe',now);await security.verify(token,now);
  await tx.query('COMMIT');return keys;
 }catch(e){await tx.query('ROLLBACK');throw e;}finally{tx.release();}}finally{await db.close();}
}
