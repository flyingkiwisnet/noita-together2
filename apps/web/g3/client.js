import {ServiceError,validate} from '../candidate/client.js';
export class StatisticsClient {
 constructor(browser,transport){this.browser=browser;this.transport=transport;this.schema=null;}
 async request(scope,payload){
  return this.call('/v3','g3-draft-1',{scope,...payload});
 }
 async discover(afterSequence='0',throughSequence,limit=32){
  const r=await this.call('/v3/history','g3-history-draft-1',{discover:{afterSequence,limit,...(throughSequence===undefined?{}:{throughSequence})}});
  const decimal=v=>typeof v==='string'&&/^(0|[1-9][0-9]{0,18})$/.test(v)&&BigInt(v)<=9223372036854775807n;
  if(!decimal(r.nextSequence)||!decimal(r.highSequence)||BigInt(r.nextSequence)<BigInt(afterSequence)||(throughSequence!==undefined&&r.highSequence!==throughSequence))throw new ServiceError('INVALID_MESSAGE');
  let previous=BigInt(afterSequence);for(const run of r.runs??[]){if(!decimal(run.sequence)||BigInt(run.sequence)<=previous||BigInt(run.sequence)>BigInt(r.highSequence)||!run.scope||['roomId','runId','runEpoch'].some(k=>!/^[a-f0-9]{8}(-[a-f0-9]{4}){3}-[a-f0-9]{12}$/.test(run.scope[k]??''))||!['empty','available','expired','deleted'].includes(run.collectionState))throw new ServiceError('INVALID_MESSAGE');previous=BigInt(run.sequence);}
  if(BigInt(r.nextSequence)<previous)throw new ServiceError('INVALID_MESSAGE');return r;
 }
 async call(path,revision,payload){
  return this.browser.exclusive(async()=>{
   if(!this.browser.credentials)throw new ServiceError('UNAUTHENTICATED');
   if(this.browser.pendingRefresh||Date.parse(this.browser.credentials.expiresAt)-this.browser.now()<60000)await this.browser.recoverRefresh();
   const generation=this.browser.generation;
   const [schema,info]=await Promise.all([this.transport(path+'/schema'),this.transport(path+'/info')]);
   if(schema.revision!==revision||info.revision!==schema.revision||info.serviceId!==this.browser.info.serviceId)throw new ServiceError('VERSION_MISMATCH');
   if(generation!==this.browser.generation||!this.browser.credentials)throw new ServiceError('CANCELLED');
   const request=validate(schema,'Request',{major:1,revision:schema.revision,serviceId:info.serviceId,serviceEpoch:info.serviceEpoch,sessionEpoch:this.browser.credentials.sessionEpoch,requestId:this.browser.uuid(),...payload});
   const response=await this.transport(path+'/rpc',request,this.browser.credentials.accessToken);
   if(generation!==this.browser.generation)throw new ServiceError('CANCELLED');
   validate(schema,'Response',response);
   if(response.requestId!==request.requestId||response.serviceId!==info.serviceId||response.serviceEpoch!==info.serviceEpoch||response.revision!==schema.revision)throw new ServiceError('INVALID_MESSAGE');
   if(response.error)throw new ServiceError(response.error.code);this.schema=schema;return response;
  });
 }
}
