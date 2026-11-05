import {randomUUID} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {schema,decode,encode} from '../../../packages/contracts/src/g3.js';
import type {Operation,Request,Scope} from '../../../packages/contracts/g3/g3.js';
export async function b1Fixture(options:{worldRules?:boolean;itemTransfers?:boolean;sendWands?:boolean;sendFlasks?:boolean;sendItems?:boolean;sendGold?:boolean;includeGoldCapability?:boolean}={}){
 const x=await startLifecycleLc1Fixture({b1Gold:true,...options});
 try{
  const peer=await x.bootstrapPeer(),qa=x.request(),qb=x.request(peer.desktop,1,'b1-peer',peer.observation);qb.reconcile!.association=peer.association;qb.reconcile!.proof=peer.proof;
  await x.send(x.desktop,qa);await x.send(peer.desktop,qb);await x.start();qa.reconcile!.observation={...x.observation,protected:false,observationRevision:2};qb.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};
  const values=[await x.send(x.desktop,qa),await x.send(peer.desktop,qb)];
  const scope=(i:number):Scope=>Object.fromEntries(['roomId','runId','runEpoch','configRevision','attachmentId','worldToken'].map(k=>[k,values[i].canonicalAdmission.scope[k]]));
  const body=(i:number,payload:Partial<Request>,original=scope(i),credentials=values[i].credentials):Request=>({major:1,revision:schema.revision,serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,sessionEpoch:credentials.sessionEpoch,requestId:randomUUID(),scope:original,...payload});
  const raw=async(credentials:any,request:Request)=>{x.advance(20);const response=await x.fixture.call('/v3/rpc',request,credentials.accessToken);return JSON.parse(response.body);};
  const send=async(i:number,payload:Partial<Request>,original=scope(i))=>raw(values[i].credentials,body(i,payload,original));
  const ok=async(i:number,payload:Partial<Request>,original=scope(i))=>{const value=await send(i,payload,original);if(value.error)throw Error(value.error.code);decode('Response',encode('Response',value));return value;};
  const receipt=(o:Operation,outcome:string)=>({goldReceipt:{operationId:o.operationId!,deliveryId:o.deliveryId!,payloadDigest:o.payloadDigest!,receiptId:randomUUID(),outcome,evidenceDigest:'a'.repeat(64)}});
  return {x,peer,qa,qb,values,scope,body,raw,send,ok,receipt,close:x.close};
 }catch(e){await x.close();throw e;}
}
