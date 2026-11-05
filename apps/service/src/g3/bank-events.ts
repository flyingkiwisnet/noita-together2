import type {Tx} from '../g1/database.js';
import {afterCommit,transactionLocal} from '../g1/commit-hooks.js';
const dirtyKey=Symbol('bank changes');
export interface BankChange {runId:string;revision:string}
export function markBankDirty(tx:Tx,runId:string){transactionLocal(tx,dirtyKey,()=>new Set<string>()).add(runId);}
export async function bankRevision(tx:Tx,runId:string){return String((await tx.query('SELECT revision FROM g3_bank_versions WHERE run_id=$1',[runId])).rows[0]?.revision??'0');}
export async function prepareBankEvents(tx:Tx,publish:(changes:BankChange[])=>void){
 const dirty=transactionLocal(tx,dirtyKey,()=>new Set<string>());if(!dirty.size)return;
 const changes:BankChange[]=(await tx.query('SELECT run_id,revision FROM g3_bank_versions WHERE run_id=ANY($1::uuid[])',[[...dirty]])).rows.map(r=>({runId:r.run_id,revision:String(r.revision)}));
 if(changes.length)afterCommit(tx,()=>publish(changes),()=>{});
}
// Public data only. Caller authorization and private notices/originals are never cached.
export class CommittedBankReads {
 private entries=new Map<string,{value:unknown;bytes:number;until:number}>();private bytes=0;
 readonly stats={hits:0,misses:0,published:0};
 get<T>(key:string,now:number):T|undefined{const v=this.entries.get(key);if(v&&v.until>now){this.stats.hits++;return structuredClone(v.value) as T;}if(v){this.bytes-=v.bytes;this.entries.delete(key);}this.stats.misses++;return undefined;}
 put(tx:Tx,key:string,value:unknown,now:number){const bytes=Buffer.byteLength(JSON.stringify(value));if(bytes>1024*1024)return;const copy=structuredClone(value);afterCommit(tx,()=>{const old=this.entries.get(key);if(old){this.bytes-=old.bytes;this.entries.delete(key);}while(this.entries.size>=128||this.bytes+bytes>8*1024*1024){const first=this.entries.entries().next().value;if(!first)break;this.entries.delete(first[0]);this.bytes-=first[1].bytes;}this.entries.set(key,{value:copy,bytes,until:now+30000});this.bytes+=bytes;this.stats.published++;},()=>{});}
}
