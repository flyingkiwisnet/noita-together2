import {isIP} from 'node:net';
import type {Tx} from '../g1/database.js';
import {demand,hash} from '../g1/security.js';
export const SIGNIN_ADMISSION={outstandingPerSource:32,startsPerWindow:128,windowMs:300000,globalOutstanding:128} as const;
export function signInSource(ip:string){
 // Collapse mapped IPv4 and textual IPv6 aliases. Proxy trust is configured by
 // Fastify, never by a caller's arbitrary forwarding header or device/app ID.
 const raw=ip.startsWith('::ffff:')&&isIP(ip.slice(7))===4?ip.slice(7):ip;
 demand(isIP(raw)!==0,'INVALID_MESSAGE');
 const canonical=isIP(raw)===6?new URL('http://['+raw+']/').hostname:raw;
 return hash(canonical);
}
export async function admitSignIn(tx:Tx,source:string,now:number,kind:'app'|'web'){
 // Counts and insertion share a transaction and a schema-local lock. Concurrent
 // requests from new sources cannot overbook either global pending pool.
 await tx.query("SELECT pg_advisory_xact_lock(hashtext(current_schema()||':signin-admission'))");
 const table=kind==='app'?'account_signin_attempts':'auth1_flows',pending=kind==='app'?"state IN ('pending','completed')":'consumed_at IS NULL';
 const counts=(await tx.query(`SELECT count(*)::int AS total,count(*) FILTER(WHERE source_hash=$2)::int AS own FROM ${table} WHERE ${pending} AND expires_at>$1`,[new Date(now),source])).rows[0];
 demand(counts.total<SIGNIN_ADMISSION.globalOutstanding&&counts.own<SIGNIN_ADMISSION.outstandingPerSource,'CAPACITY');
 const row=(await tx.query(`INSERT INTO auth1_admission_sources(source_hash,starts,until_at) VALUES($1,1,$2)
  ON CONFLICT(source_hash) DO UPDATE SET starts=CASE WHEN auth1_admission_sources.until_at<=$3 THEN 1 ELSE auth1_admission_sources.starts+1 END,
  until_at=CASE WHEN auth1_admission_sources.until_at<=$3 THEN $2 ELSE auth1_admission_sources.until_at END RETURNING starts`,[source,new Date(now+SIGNIN_ADMISSION.windowMs),new Date(now)])).rows[0];
 demand(row.starts<=SIGNIN_ADMISSION.startsPerWindow,'CAPACITY');
}
