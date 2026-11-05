// Fixture only. Observe existing guards; never export tokens, scopes or actor IDs.
import {AsyncLocalStorage} from 'node:async_hooks';
import {performance} from 'node:perf_hooks';
import {createHash} from 'node:crypto';
import {liveScopeDigest} from '../../../packages/domain/src/g3-live.js';
import {readRoomAdmissions} from '../src/g3/admission.js';

export function traceLiveAuthentication(service:any,refusalCount:()=>number=()=>0,readAdmissions=readRoomAdmissions){
 const live=service.live,projection=live.projection,context=new AsyncLocalStorage<any>();
 const began=performance.now(),events:any[]=[],sockets=new WeakMap<object,number>();let attempt=0,socketCount=0,firstFailure:any,firstRetire:any;
 const push=(value:any)=>{const event={elapsedMs:Math.floor(performance.now()-began),...value};events.push(event);if(events.length>64)events.shift();if(value.outcome==='FORBIDDEN'&&!firstFailure)firstFailure=structuredClone(event);};
 const authenticate=live.authenticate.bind(live);
 live.authenticate=(c:any,f:any,renew=false)=>{
  let socket=sockets.get(c.socket);if(!socket){socket=++socketCount;sockets.set(c.socket,socket);}
  const state={c,scope:f.scope,actor:undefined as any,event:{attempt:++attempt,socket,connectionTag:createHash('sha256').update(String(f.connectionId)).digest('hex').slice(0,16),renew,clientGeneration:f.clientGeneration,frameAuthorityGeneration:f.authorityGeneration,stage:'token-authenticate'} as any,transactions:[] as any[],refusals:refusalCount(),failed:false};
  return context.run(state,async()=>{await authenticate(c,f,renew);if(!state.failed)push({...state.event,outcome:c.retired?'retired-other':'accepted'});});
 };
 // Reuse the database's sanitized observer. Admission/lock failure may happen
 // before Auth.authenticate runs, so token-authenticate alone is not a cause.
 const previousDiagnostic=service.db.transactionDiagnostic;
 service.db.transactionDiagnostic=(event:any)=>{previousDiagnostic?.(event);const state=context.getStore();if(!state)return;
  const safe:any={};for(const [key,pattern]of Object.entries({phase:/^(admission|acquire|begin|lock|epoch|work|projection|commit|publish)$/,category:/^(service-admission|transaction|deadline|committed)$/,code:/^[A-Z_]{1,64}$/,sqlState:/^[0-9A-Z]{5}$/,abortCause:/^(client-error|transaction-deadline|caller-abort)$/,commit:/^(not-attempted|uncertain|confirmed)$/}))if(typeof event[key]==='string'&&pattern.test(event[key]))safe[key]=event[key];
  state.transactions.push(safe);if(state.transactions.length>8)state.transactions.shift();
 };
 const auth=service.auth.authenticate.bind(service.auth);
 service.auth.authenticate=async(...args:any[])=>{const state=context.getStore();const actor=await auth(...args);if(state){state.actor=actor;state.event.stage='token-scope';state.event.identity={game:actor.kind==='game',principal:actor.accountId===state.scope.principalId,session:actor.sessionId===state.scope.sessionId,epoch:actor.sessionEpoch===state.scope.sessionEpoch};}return actor;};
 const read=projection.read.bind(projection);
 projection.read=async(tx:any,id:string)=>{const r=await read(tx,id),state=context.getStore();if(state?.event.stage==='projection-authorize'&&id===state.scope.roomId)state.event.projection={terminal:!!r.terminal,exactScope:r.members.some((m:any)=>m.digest===liveScopeDigest(state.scope)),sameSession:r.members.some((m:any)=>m.scope.sessionId===state.scope.sessionId)};return r;};
 const authorize=projection.authorize.bind(projection);
 projection.authorize=async(tx:any,scope:any)=>{
  const state=context.getStore();if(state)state.event.stage='projection-authorize';
  try{await authorize(tx,scope);if(state)state.event.stage='transaction-prepare';}
  catch(error){
   if(state&&(error as any)?.code==='FORBIDDEN'){
    state.event.fixtureRefusalConsumed=refusalCount()>state.refusals;
    // Failure-only second canonical read, in the SAME namespace-locked tx.
    // The original projection result above remains the exact guard evidence.
    try{const r=await readAdmissions(tx,service.db,projection.authority.compatibility,service.auth.now(),scope.roomId);
     const world=r.worlds.find(w=>liveScopeDigest(w.scope)===liveScopeDigest(scope))??r.worlds.find(w=>w.scope.worldToken===scope.worldToken);
     const reason=world?.decision.reason??'';
     state.event.canonical={terminal:r.terminal,worldPresent:!!world,...(world?{exactScope:liveScopeDigest(world.scope)===liveScopeDigest(scope),sameSession:world.scope.sessionId===scope.sessionId,sameAttachment:world.scope.attachmentId===scope.attachmentId,generation:world.generation,authorized:world.decision.authorized,playing:world.decision.playing,decisionTerminal:world.decision.terminal,reason:['','host_ban','account_disabled','access_revoked','retired','owner_binding_changed','inspection_required','report_invalid','initial_review_expired','lease_expired','entry_required'].includes(reason)?reason:'other',reportRevision:world.currentReport.revision??null,reportProtected:world.currentReport.protected,reportComplete:world.currentReport.complete,reportLoaded:world.currentReport.loaded,reportContentValid:world.currentReport.contentValid,coreMode:world.mode==='core',ghostCapability:world.capabilities.includes('core.ghost')}: {})};
    }catch{state.event.canonical={diagnosticReadFailed:true};}
   }throw error;
  }
 };
 const member=projection.member.bind(projection);
 projection.member=(room:string,session:string)=>{const m=member(room,session),state=context.getStore();if(state&&['transaction-prepare','transaction-token-check','post-transaction-member'].includes(state.event.stage)&&room===state.scope.roomId&&session===state.scope.sessionId){state.event.stage='post-transaction-member';state.event.member={present:!!m,digest:!!m&&m.digest===liveScopeDigest(state.scope),parent:!!m&&m.parentId===state.actor?.parentId,family:!!m&&m.familyId===state.actor?.familyId};}return m;};
 // Effects run synchronously after tokens() returns. Keep the stage until the
 // first post-transaction member lookup, rather than losing callback context.
 const tokens=live.tokens.bind(live);live.tokens=(tx:any)=>{const state=context.getStore();if(state)state.event.stage='transaction-token-check';return tokens(tx);};
 const retire=live.retire.bind(live);
 live.retire=(c:any,code:string,...args:any[])=>{const state=context.getStore();if(!c.retired){
  const stack=new Error().stack??'',origin=stack.includes('authenticateObserved')?'authentication':stack.includes('drain')?'outbound-encode':stack.includes('tick')?'tick':stack.includes('live-service')?'live-frame':'other',outcome=/^[A-Z_]{1,64}$/.test(code)?code:'OTHER';
  const event=state?.c===c?{...state.event,outcome,bound:!!c.binding,busy:!!c.busy,origin,transactions:structuredClone(state.transactions)}:{stage:'outside-authentication',outcome,bound:!!c.binding,busy:!!c.busy,origin};
  if(!firstRetire)firstRetire=structuredClone(event);if(state?.c===c)state.failed=true;push(event);
 }return retire(c,code,...args);};
 return {status:()=>({revision:2,attempts:attempt,events:structuredClone(events),firstFailure:firstFailure?structuredClone(firstFailure):null,firstRetire:firstRetire?structuredClone(firstRetire):null})};
}
