import type {Tx} from '../g1/database.js';
import type {Actor} from '../g1/auth.js';
import {commitEffectsCheckpoint} from '../g1/commit-hooks.js';
import {demand,PolicyError} from '../g1/security.js';
import type {Request,Response,BankBatchEntryResult} from '../../../../packages/contracts/g3/g3.js';
import type {G3Authority} from './authority.js';
export const BANK_BATCH_LIMITS={entries:4,requestBytes:196608,responseBytes:258048} as const;
const commands=['goldIntent','goldStatus','goldReceipt','itemDeposit','itemClaim','itemStatus','itemReceipt'] as const;
export async function executeBankBatch(authority:G3Authority,tx:Tx,actor:Actor,req:Request,effects:(()=>void)[]):Promise<Partial<Response>>{
 const entries=req.bankBatch?.entries??[];demand(entries.length>=1&&entries.length<=4&&Buffer.byteLength(JSON.stringify(req))<=BANK_BATCH_LIMITS.requestBytes,'INVALID_MESSAGE');
 const results:BankBatchEntryResult[]=[];
 for(const [index,entry]of entries.entries()){
  const command=commands.find(k=>entry[k]!==undefined),operationId=command?entry[command]?.operationId??'':'';
  const checkpoint=commitEffectsCheckpoint(tx),effectCount=effects.length;
  await tx.query('SAVEPOINT bank_entry');
  try{
   demand(command&&commands.filter(k=>entry[k]!==undefined).length===1&&entry.scope,'INVALID_MESSAGE');
   for(const key of ['roomId','runId','runEpoch','configRevision'] as const)demand(entry.scope[key]===req.scope?.[key],'FORBIDDEN');
   const input:Request={major:req.major!,revision:req.revision!,serviceId:req.serviceId!,serviceEpoch:req.serviceEpoch!,sessionEpoch:req.sessionEpoch!,requestId:req.requestId!,scope:entry.scope,...(entry.currentScope?{currentScope:entry.currentScope}:{}),[command]:entry[command]};
   const response=await authority.execute(tx,actor,input,effects),result={index,operationId,...response};
   // Reserve space for all remaining minimal error results and outer envelope.
   demand(Buffer.byteLength(JSON.stringify([...results,result]))<=BANK_BATCH_LIMITS.responseBytes-2048,'BATCH_TOO_LARGE');
   results.push(result);await tx.query('RELEASE SAVEPOINT bank_entry');
  }catch(e){
   await tx.query('ROLLBACK TO SAVEPOINT bank_entry');checkpoint();effects.length=effectCount;
   await tx.query('RELEASE SAVEPOINT bank_entry');
   if(!(e instanceof PolicyError))throw e;
   results.push({index,operationId,error:{code:e.code}});
  }
 }
 return {bankBatch:{entries:results}};
}
