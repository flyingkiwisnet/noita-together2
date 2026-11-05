// Disposable fixture evidence only; no request bodies, secrets, SQL or messages.
import {AsyncLocalStorage} from 'node:async_hooks';
import {randomUUID} from 'node:crypto';
import type {TransactionDiagnostic} from '../src/g1/database.js';
import type {startG3Fixture} from './g3-loot01-fixture.js';
type Service=Awaited<ReturnType<typeof startG3Fixture>>['service'];
type Step={stage:string;requestId?:string;httpStatus?:number;responseCode?:string;serviceCompleted?:boolean;elapsedMs?:number;transactions:TransactionDiagnostic[]};
type Review={attempt:number;limit:number;previousOperationId?:string;inspectionChanges:{worldId:string;fields:string[]}[]};
type Report={operation:'start';operationId:string;stages:Step[];failedStage?:string;category?:string;code?:string;startRequestId?:string;outcome:'not-submitted'|'committed'|'not-committed'|'unknown'|'different-start';outcomeCheck?:string;review?:Review};
export const diagnosticCode=(e:unknown)=>e instanceof Error&&/^[A-Z_]{1,64}$/.test(e.message)?e.message:'FIXTURE_CONTROL_FAILED';
export class StartDiagnostics{
 private context=new AsyncLocalStorage<Step|undefined>();
 private reports:Report[]=[];private current:Report|undefined;private activeStep:Step|undefined;
 install(service:Service){
  const previous=service.db.transactionDiagnostic;
  service.db.transactionDiagnostic=e=>{previous?.(e);const step=this.context.getStore();if(step&&step.transactions.length<8)step.transactions.push({...e});};
  service.app.addHook('preHandler',(req,_reply,done)=>{
   const id=(req.body as {requestId?:unknown})?.requestId;
   const step=typeof id==='string'?this.reports.flatMap(r=>r.stages).find(s=>s.requestId===id):undefined;
   this.context.run(step,done);
  });
  service.app.addHook('onResponse',async req=>{const id=(req.body as {requestId?:unknown})?.requestId;if(typeof id!=='string')return;const step=this.reports.flatMap(r=>r.stages).find(s=>s.requestId===id);if(step)step.serviceCompleted=true;});
 }
 begin(){if(this.current)throw Error('FIXTURE_START_BUSY');if(this.reports.some(r=>r.outcome==='unknown'))throw Error('FIXTURE_START_UNCERTAIN');this.current={operation:'start',operationId:randomUUID(),stages:[],outcome:'not-submitted'};this.reports.push(this.current);if(this.reports.length>16)this.reports.shift();return this.current;}
 async step<T>(stage:string,fn:()=>Promise<T>):Promise<T>{
  const report=this.current!;if(report.stages.length>=40)throw Error('FIXTURE_START_BOUNDS');
  const step:Step={stage,transactions:[]};report.stages.push(step);this.activeStep=step;const began=performance.now();
  try{return await this.context.run(step,fn);}catch(e){
   if(!report.failedStage){report.failedStage=stage;report.code=diagnosticCode(e);const db=[...step.transactions].reverse().find(t=>t.category!=='committed');
    const network=e instanceof Error&&['ECONNRESET','ECONNREFUSED','EPIPE','ETIMEDOUT'].includes(String((e as NodeJS.ErrnoException).code??e.message));
    report.category=stage==='descriptor'||stage==='state'?'postcommit-control':db?.category??(e instanceof Error&&e.message==='Fixture deadline exceeded'?'deadline':network||step.httpStatus===503&&step.responseCode==='FIXTURE_TRANSPORT_CUT'?'transport-cut':step.httpStatus!==undefined?'service-admission':'control');}
   throw e;
  }finally{step.elapsedMs=Math.round(performance.now()-began);this.activeStep=undefined;}
 }
 request(requestId:string,final:boolean){if(!this.current||!this.activeStep)return;this.activeStep.requestId=requestId;if(final){this.current.startRequestId=requestId;this.current.outcome='unknown';}}
 response(status:number,code?:string,transportCut=false){if(this.activeStep){this.activeStep.httpStatus=status;if(code)this.activeStep.responseCode=transportCut?'FIXTURE_TRANSPORT_CUT':code;}}
 finish(){const report=structuredClone(this.current!);this.current=undefined;this.activeStep=undefined;return report;}
 snapshot(){return structuredClone(this.reports);}
 review(value:Review){if(!this.current||value.inspectionChanges.length>32||value.inspectionChanges.some(v=>v.fields.length>64))throw Error('FIXTURE_START_BOUNDS');this.current.review=structuredClone(value);}
}
