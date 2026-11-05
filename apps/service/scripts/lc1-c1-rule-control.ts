import {fixtureC1Rules,validateFixtureC1Rules} from './lc1-c1-controls.js';
import {FixtureRpcRejection} from './lc1-host-action.js';
import {InvalidMessage} from '../../../packages/contracts/src/g3-c1.js';
type Stage='arguments'|'host-refresh'|'rules-read'|'rules-write';
export const C1_RULE_CONTROL_LIMITS={attempts:4,totalDeadlineMs:8000} as const;
const codes=new Set(['CAPACITY','CONFLICT','UNAUTHENTICATED','FORBIDDEN','INVALID_MESSAGE','NOT_FOUND','INVALID_STATE','STALE_REVISION','FIXTURE_C1_RULES','FIXTURE_C1_RESPONSE','FIXTURE_C1_RULE_BOUNDS','FIXTURE_C1_DISABLED','FIXTURE_C1_DEADLINE','FIXTURE_C1_RETRY_COMMAND','FIXTURE_C1_RETRY_BOUNDS']);
export class C1RuleDiagnostic{
 private stage:Stage='arguments';private events:any[]=[];
 begin(){this.stage='arguments';this.events=[];}
 event(stage:Stage,outcome:'begin'|'accepted'|'capacity'|'failed',error?:unknown,attempt?:number){this.stage=stage;const raw=error instanceof FixtureRpcRejection?error.code:error instanceof Error?error.message:'';this.events.push({stage,outcome,...(error?{code:error instanceof InvalidMessage||error instanceof SyntaxError?'INVALID_MESSAGE':codes.has(raw)?raw:'UNKNOWN_FAILURE',correlated:error instanceof FixtureRpcRejection&&error.sameService,...(error instanceof FixtureRpcRejection&&Number.isInteger(error.status)&&error.status>=100&&error.status<=599?{status:error.status}:{})}:{}),...(attempt!==undefined?{attempt}:{})});this.events=this.events.slice(-24);}
 snapshot(){return {revision:1,stage:this.stage,limits:C1_RULE_CONTROL_LIMITS,events:structuredClone(this.events)};}
 fail(error:unknown){if(this.events.at(-1)?.outcome!=='failed')this.event(this.stage,'failed',error);}
}
export async function runC1RuleControl(input:Parameters<typeof fixtureC1Rules>[0],deps:{before:()=>void;ensureHost:(deadline:number,busy:(attempt:number)=>void)=>Promise<void>;send:(payload:any,deadline:number,busy:(attempt:number)=>void)=>Promise<any>},diagnostic:C1RuleDiagnostic,now=()=>performance.now()){
 diagnostic.begin();const deadline=now()+C1_RULE_CONTROL_LIMITS.totalDeadlineMs;
 const step=async<T>(stage:Stage,fn:(busy:(attempt:number)=>void)=>Promise<T>):Promise<T>=>{diagnostic.event(stage,'begin');try{if(now()>=deadline)throw Error('FIXTURE_C1_DEADLINE');const result=await fn(attempt=>diagnostic.event(stage,'capacity',new FixtureRpcRejection('CAPACITY',200,true),attempt));if(now()>=deadline)throw Error('FIXTURE_C1_DEADLINE');diagnostic.event(stage,'accepted');return result;}catch(error){diagnostic.event(stage,'failed',error);throw error;}};
 await step('arguments',async()=>{deps.before();validateFixtureC1Rules(input);});
 await step('host-refresh',busy=>deps.ensureHost(deadline,busy));
 try{return await fixtureC1Rules(input,payload=>step(payload.rulesGet?'rules-read':'rules-write',busy=>deps.send(payload,deadline,busy)));}catch(error){diagnostic.fail(error);throw error;}
}
