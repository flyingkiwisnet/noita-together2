import type {Tx} from './database.js';
import {PolicyError} from './security.js';
const effects=new WeakMap<Tx,{apply:()=>void;invalidate:()=>void}[]>();
const locals=new WeakMap<Tx,Map<symbol,unknown>>();
// Pool clients are reused: scoped bookkeeping must end on both commit and rollback.
export function transactionLocal<T>(tx:Tx,key:symbol,create:()=>T):T{let values=locals.get(tx);if(!values){values=new Map();locals.set(tx,values);}if(!values.has(key))values.set(key,create());return values.get(key) as T;}
export function commitEffectsCheckpoint(tx:Tx){const length=effects.get(tx)?.length??0,values=locals.get(tx),saved=values?new Map([...values].map(([k,v])=>[k,v instanceof Set?new Set(v):v])):undefined;return()=>{const current=effects.get(tx);if(current){current.length=length;if(!length)effects.delete(tx);}if(saved)locals.set(tx,saved);else locals.delete(tx);};}
// Synchronous, bounded projection effects; never perform SQL/network work here.
export function afterCommit(tx:Tx,apply:()=>void,invalidate:()=>void){
 const entries=effects.get(tx)??[];if(entries.length>=128)throw new PolicyError('CAPACITY');
 entries.push({apply,invalidate});effects.set(tx,entries);
}
export function discardCommitEffects(tx:Tx){effects.delete(tx);locals.delete(tx);}
export function publishCommitEffects(tx:Tx){
 const entries=effects.get(tx)??[];effects.delete(tx);locals.delete(tx);
 for(const e of entries)try{e.apply();}catch{try{e.invalidate();}catch{/* A committed reliable result must not be rewritten by projection failure. */}}
}
