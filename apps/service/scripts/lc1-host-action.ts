// Isolated fixture host intent. Never retry transport/uncertain errors or Start.
import {validate as validateC1} from '../../../packages/contracts/src/g3-c1.js';
import {fingerprint} from '../src/g1/security.js';
import {validate as validateLobby} from '../../../packages/contracts/src/g2-next.js';
export class FixtureRpcRejection extends Error{
 constructor(readonly code:string,readonly status:number,readonly sameService:boolean){super(code);}
}

// Optional fixture chat only. A CAPACITY response can follow committed work,
// so replay exact bytes/token and let canonical request-fingerprint dedup own it.
type RetryTiming={now:()=>number;wait:(ms:number)=>Promise<void>};
const retryTiming:RetryTiming={now:()=>performance.now(),wait:ms=>new Promise<void>(resolve=>setTimeout(resolve,ms))};
async function confirmedCapacity<T>(bytes:string,token:string,deadline:number,send:(bytes:string,token:string,timeoutMs:number)=>Promise<T>,busy:(attempt:number)=>void,timing:RetryTiming,prefix:'FIXTURE_CHAT'|'FIXTURE_C1'):Promise<T>{
 for(let attempt=1;attempt<=4;attempt++){
  const remaining=Math.floor(deadline-timing.now());if(remaining<=0)throw Error(prefix+'_DEADLINE');
  try{return await send(bytes,token,Math.min(5000,remaining));}
  catch(error){if(!(error instanceof FixtureRpcRejection)||error.code!=='CAPACITY'||error.status!==200||!error.sameService)throw error;busy(attempt);if(attempt===4||deadline-timing.now()<=250)throw error;await timing.wait(250);}
 }
 throw Error(prefix+'_RETRY_BOUNDS');
}

export async function fixtureChatCapacity<T>(bytes:string,token:string,deadline:number,send:(bytes:string,token:string,timeoutMs:number)=>Promise<T>,busy:(attempt:number)=>void=()=>{},timing=retryTiming):Promise<T>{
 const command=JSON.parse(bytes);if(!command.roomGet&&!command.chatSend)throw Error('FIXTURE_CHAT_RETRY_COMMAND');return confirmedCapacity(bytes,token,deadline,send,busy,timing,'FIXTURE_CHAT');
}
export async function fixtureC1ReadRefreshCapacity<T>(kind:'read'|'refresh',bytes:string,token:string,deadline:number,send:(bytes:string,token:string,timeoutMs:number)=>Promise<T>,busy:(attempt:number)=>void=()=>{},timing=retryTiming):Promise<T>{
 const command=JSON.parse(bytes);if(kind==='read'){validateC1('Request',command);if(!command.rulesGet)throw Error('FIXTURE_C1_RETRY_COMMAND');}else if(kind==='refresh'){validateLobby('Request',command);if(!command.refresh)throw Error('FIXTURE_C1_RETRY_COMMAND');}else throw Error('FIXTURE_C1_RETRY_COMMAND');
 return confirmedCapacity(bytes,token,deadline,send,busy,timing,'FIXTURE_C1');
}

// End's optional prerequisite read only. Never replay the End mutation or auth.
export async function fixtureHostReadCapacity<T>(bytes:string,token:string,deadline:number,send:(bytes:string,token:string,timeoutMs:number)=>Promise<T>,busy:(attempt:number)=>void=()=>{},timing={now:()=>performance.now(),wait:(ms:number)=>new Promise<void>(resolve=>setTimeout(resolve,ms))}):Promise<T>{
 const request=validateLobby('Request',JSON.parse(bytes));if(!request.roomGet)throw Error('FIXTURE_HOST_READ_COMMAND');
 return fixtureChatCapacity(bytes,token,deadline,send,busy,timing);
}
type Room={roomId:string;runId:string;runEpoch:string;configRevision:number;revision:number;state:string;config:unknown};
export type HostAction='moderate'|'nextRun'|'end'|'chat';
const scope=(r:Room)=>fingerprint({roomId:r.roomId,runId:r.runId,runEpoch:r.runEpoch,configRevision:r.configRevision,config:r.config});
export async function fixtureHostAction<T>(operation:HostAction,read:()=>Promise<Room>,submit:(room:Room,payload:any)=>Promise<T>,intent:(room:Room)=>any):Promise<T>{
 const first=structuredClone(await read()),key=scope(first),payload=structuredClone(intent(first));let current=first;
 if(operation==='nextRun'&&first.state!=='ended')throw Error('FIXTURE_END_REQUIRED');
 // End's INVALID_STATE combines revision/state/run checks. Keep it fail-closed;
 // only explicit STALE_REVISION from moderation/configure permits a retry.
 for(let attempt=0;attempt<3;attempt++){
  try{return await submit(structuredClone(current),structuredClone(payload));}
  catch(error){
   if(operation==='end'||!(error instanceof FixtureRpcRejection)||error.code!=='STALE_REVISION'||error.status!==200||!error.sameService||attempt===2)throw error;
   const next=structuredClone(await read());
   if(scope(next)!==key||next.state!==first.state||next.revision<=current.revision)throw Error('FIXTURE_HOST_INTENT_CHANGED');
   current=next;
  }
 }
 throw Error('FIXTURE_HOST_ACTION_BOUNDS');
}
