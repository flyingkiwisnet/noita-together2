import {createHash} from 'node:crypto';
// Exact original envelope, including original proof, revision and epochs. The
// status request carries ONLY the digest. No Unicode normalization or re-tagging.
// Protocol object keys are ASCII; sort by ordinal UTF-16 code units, recursively.
export function canonicalOperationJson(value:unknown):string{
 const stable=(v:any):any=>Array.isArray(v)?v.map(stable):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort((a,b)=>a<b?-1:a>b?1:0).map(k=>[k,stable(v[k])])):v;
 return JSON.stringify(stable(value));
}
export const operationFingerprint=(value:unknown)=>createHash('sha256').update(canonicalOperationJson(value),'utf8').digest('hex');
