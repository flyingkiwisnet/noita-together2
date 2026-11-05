import {validateRoomSemantics} from './room-semantics.js';
export class ServiceError extends Error {
 constructor(code,retryAfterMs=0){super(code);this.code=code;this.retryAfterMs=retryAfterMs;}
}
export const browserSignInCapacityMessage='Browser sign-in is at capacity. Sign out of an unused tab for this account, then try again. Abandoned sign-ins become reclaimable after their access credentials expire plus 30 minutes.';
export function validate(schema,name,value){
 let fields=0;
 function visit(type,obj,depth){
  const def=schema.messages[type];if(!def||!obj||typeof obj!=='object'||Array.isArray(obj)||depth>schema.limits.depth)throw new ServiceError('INVALID_MESSAGE');
  for(const [key,v] of Object.entries(obj)){
   if(++fields>schema.limits.fields)throw new ServiceError('CAPACITY');const f=def.fields[key];if(!f)throw new ServiceError('INVALID_MESSAGE');
   const item=x=>{if(f.type==='string'){if(typeof x!=='string'||new TextEncoder().encode(x).length>(f.repeated?f.itemMax??128:f.max??128))throw new ServiceError('INVALID_MESSAGE');}
    else if(f.type==='bool'){if(typeof x!=='boolean')throw new ServiceError('INVALID_MESSAGE');}
    else if(f.type==='uint32'){if(!Number.isSafeInteger(x)||x<0||x>4294967295)throw new ServiceError('INVALID_MESSAGE');}
    else visit(f.type,x,depth+1);};
   if(f.repeated){if(!Array.isArray(v)||v.length>(f.max??32))throw new ServiceError('CAPACITY');v.forEach(item);}else item(v);
  }
  if(def.oneof&&def.oneof.filter(k=>Object.hasOwn(obj,k)).length!==1)throw new ServiceError('INVALID_MESSAGE');
 }
 visit(name,value,0);if(new TextEncoder().encode(JSON.stringify(value)).length>schema.limits.bytes)throw new ServiceError('CAPACITY');return value;
}
export function canonicalDiscoveryInfo(info){
 const {roomSemantics,developmentFeatures,...canonical}=info;
 if(Object.hasOwn(info,'developmentFeatures')){
  const exact=(value,keys)=>!!value&&typeof value==='object'&&!Array.isArray(value)&&Object.keys(value).sort().join(',')===[...keys].sort().join(',');
  if(!exact(developmentFeatures,['testingStartingLoadout']))throw new ServiceError('INVALID_MESSAGE');
  const feature=developmentFeatures.testingStartingLoadout;
  if(!exact(feature,['available','settingKey','default'])||feature.available!==true||feature.settingKey!=='NT_testing_mode'||feature.default!==false||!Array.isArray(info.compatibility?.settingKeys)||info.compatibility.settingKeys.filter(k=>k==='NT_testing_mode').length!==1)throw new ServiceError('INVALID_MESSAGE');
 }
 return canonical;
}
export function sameDesktop(a,b){return !!a&&!!b&&['sessionId','sessionEpoch','accountId','deviceId','kind'].every(k=>a[k]===b[k]);}
export class HostClient {
 constructor(transport,uuid=()=>crypto.randomUUID(),now=()=>Date.now()){this.transport=transport;this.uuid=uuid;this.now=now;this.credentials=null;this.pendingRefresh=null;this.pendingLogin=null;this.loginSubject=null;this.pageClosed=false;this.info=null;this.schema=null;this.active=false;this.generation=0;this.device='browser-'+uuid();}
 dispose(){this.credentials=null;this.pendingRefresh=null;this.pendingLogin=null;this.pendingChat=null;this.pendingMutations?.clear();this.loginSubject=null;++this.generation;}
 async exclusive(fn){if(this.active)throw new ServiceError('CAPACITY',250);this.active=true;try{return await fn();}finally{this.active=false;}}
 requireInitialized(){if(!this.schema||!this.info)throw new ServiceError('INITIALIZATION_REQUIRED');}
 async initialize(){return this.exclusive(async()=>{this.schema=null;this.info=null;const [schema,info]=await Promise.all([this.transport('/schema.json'),this.transport('/v1/info')]);if(schema.major!==1||schema.revision!=='g2-draft-5')throw new ServiceError('VERSION_MISMATCH');const roomSemantics=info.roomSemantics;validate(schema,'Info',canonicalDiscoveryInfo(info));if(Object.hasOwn(info,'roomSemantics')){try{validateRoomSemantics(roomSemantics,info.compatibility);}catch{throw new ServiceError('INCOMPATIBLE_GAME');}}if(info.major!==schema.major||info.revision!==schema.revision||!info.serviceId||!info.serviceEpoch)throw new ServiceError('VERSION_MISMATCH');this.schema=schema;this.info=info;return info;});}
 async c1(payload){return this.ruleRpc('c1',payload);}
 async c2(payload){return this.ruleRpc('c2',payload);}
 async ruleRpc(lane,payload){return this.exclusive(async()=>{
  if(lane==='c2'&&!['core.world-rules','core.hourglass'].every(c=>this.info?.compatibility?.capabilities?.includes(c)))throw new ServiceError('UNSUPPORTED_CONTENT');
  if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');if(this.pendingRefresh||Date.parse(this.credentials.expiresAt)-this.now()<60000)await this.recoverRefresh();
  const generation=this.generation,c=this.credentials,serviceId=this.info.serviceId,serviceEpoch=this.info.serviceEpoch;
  const current=()=>{if(this.generation!==generation||this.credentials!==c||this.info.serviceId!==serviceId||this.info.serviceEpoch!==serviceEpoch)throw new ServiceError('CANCELLED');};
  if(this[lane+'Available']===undefined){const info=await this.transport('/v3/'+lane+'/info');current();if(info.serviceId!==serviceId||info.serviceEpoch!==serviceEpoch)throw new ServiceError('STALE_EPOCH');this[lane+'Available']=info.implemented===true;}if(!this[lane+'Available'])throw new ServiceError('UNSUPPORTED_CONTENT');
  this[lane+'Schema']??=await this.transport('/v3/'+lane+'/schema');current();const schema=this[lane+'Schema'];let pending;
  if(payload.rulesSet){this.pendingMutations??=new Map();const key=lane+':'+payload.roomId,content=JSON.stringify(payload);pending=this.pendingMutations.get(key);if(pending&&(pending.content!==content||pending.generation!==generation||pending.sessionId!==c.sessionId||pending.serviceId!==serviceId||pending.serviceEpoch!==serviceEpoch))throw new ServiceError('MUTATION_UNCERTAIN');if(!pending){if(this.pendingMutations.size>=8)throw new ServiceError('CAPACITY');pending={lane,schema,content,generation,accountId:c.accountId,sessionId:c.sessionId,sessionEpoch:c.sessionEpoch,serviceId,serviceEpoch,request:validate(schema,'Request',structuredClone({major:1,revision:schema.revision,serviceId,serviceEpoch,sessionEpoch:c.sessionEpoch,requestId:this.uuid(),...payload}))};this.pendingMutations.set(key,pending);}return this.sendRuleRequest(key,pending,current);}
  const request=validate(schema,'Request',{major:1,revision:schema.revision,serviceId,serviceEpoch,sessionEpoch:c.sessionEpoch,requestId:this.uuid(),...payload});return this.ruleTransport(lane,schema,request,current);
 });}
 async ruleTransport(lane,schema,request,current,onUncertain=()=>{}){let response;for(let attempt=0;attempt<2;attempt++){current();try{response=await this.transport('/v3/'+lane+'/rpc',request,this.credentials.accessToken);break;}catch(e){if(!(e instanceof ServiceError))onUncertain();if(e instanceof ServiceError||attempt===1)throw e;}}
  current();validate(schema,'Response',response);if(response.major!==1||response.revision!==schema.revision||response.requestId!==request.requestId||response.serviceId!==request.serviceId||response.serviceEpoch!==request.serviceEpoch)throw new ServiceError('STALE_EPOCH');if(response.error){const e=new ServiceError(response.error.code);e.verifiedResponse=true;throw e;}const snapshot=lane==='c1'?response.snapshot:response.snapshot?.rules;if(!snapshot||!Number.isInteger(snapshot.revision)||snapshot.revision<1||!snapshot.rules)throw new ServiceError('INVALID_MESSAGE');return response;
 }
 async sendRuleRequest(key,pending,current){if(pending.requiresReview)throw new ServiceError('MUTATION_UNCERTAIN');try{const response=await this.ruleTransport(pending.lane,pending.schema,pending.request,current,()=>{pending.uncertain=true;});if(this.pendingMutations.get(key)===pending)this.pendingMutations.delete(key);return response;}catch(e){if(e.verifiedResponse&&!pending.uncertain&&e.code!=='SERVICE_UNAVAILABLE'){if(this.pendingMutations.get(key)===pending)this.pendingMutations.delete(key);}else{if(e.verifiedResponse)pending.requiresReview=true;pending.uncertain=true;}throw e;}}
 async rejoin(payload){return this.exclusive(async()=>{
  if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');
  if(this.pendingRefresh||Date.parse(this.credentials.expiresAt)-this.now()<60000)await this.recoverRefresh();
  const generation=this.generation,transport=this.transport,identity=this.credentials,serviceId=this.info.serviceId,epoch=this.info.serviceEpoch,deadline=this.now()+30000;
  const guard=()=>{if(this.generation!==generation||this.transport!==transport||this.credentials!==identity||this.info.serviceId!==serviceId||this.info.serviceEpoch!==epoch)throw new ServiceError('CANCELLED');if(this.now()>=deadline)throw new ServiceError('EXPIRED');};
  this.rejoinSchema??=await transport('/v3/rejoin/schema');guard();
  const request=validate(this.rejoinSchema,'Request',{major:1,revision:this.rejoinSchema.revision,serviceId,serviceEpoch:epoch,sessionEpoch:identity.sessionEpoch,requestId:this.uuid(),...payload});
  let result;for(let attempt=0;attempt<2;attempt++){guard();try{result=await transport('/v3/rejoin/rpc',request,identity.accessToken);break;}catch(e){if(e instanceof ServiceError||attempt===1)throw e;}}
  guard();validate(this.rejoinSchema,'Response',result);if(result.serviceId!==serviceId||result.requestId!==request.requestId||result.revision!==this.rejoinSchema.revision||result.major!==1||!result.serviceEpoch)throw new ServiceError('INVALID_MESSAGE');
  this.info.serviceEpoch=result.serviceEpoch;if(result.error)throw new ServiceError(result.error.code);return result;
 });}

 async revokeBrowser(credentials){
  if(credentials?.kind!=='browser'||credentials.deviceId!==this.device)return false;
  const request=this.envelope({logout:{},sessionEpoch:credentials.sessionEpoch});
  try{const value=await this.transport('/v1/rpc',request,credentials.accessToken,{keepalive:true});validate(this.schema,'Response',value);return !value.error;}catch{return false;}
 }
 async closePage(){const old=this.credentials;this.pageClosed=true;this.dispose();return {confirmed:old?await this.revokeBrowser(old):false};}
 restorePage(){this.dispose();this.pageClosed=false;}
 async login(subject){return this.exclusive(async()=>{this.requireInitialized();
  if(this.pageClosed)throw new ServiceError('CANCELLED');
  if(this.credentials){if(this.loginSubject!==subject)throw new ServiceError('INVALID_STATE');if(this.pendingRefresh||Date.parse(this.credentials.expiresAt)-this.now()<60000)await this.recoverRefresh();return this.credentials;}
  let pending=this.pendingLogin;
  if(pending&&pending.subject!==subject)throw new ServiceError('INVALID_STATE');
  if(pending&&this.now()-pending.createdAt>=30000){this.pendingLogin=null;throw new ServiceError('EXPIRED');}
  if(!pending){pending={subject,generation:this.generation,createdAt:this.now(),body:validate(this.schema,'BrowserLogin',{subject,deviceId:this.device,requestId:this.uuid()})};this.pendingLogin=pending;}
  try{
   const response=await this.transport('/v1/mock/browser-login',pending.body);validate(this.schema,'BrowserLoginResponse',response);if(response.error)throw new ServiceError(response.error.code);
   if(this.generation!==pending.generation||this.pendingLogin!==pending||this.pageClosed){await this.revokeBrowser(response.credentials);throw new ServiceError('CANCELLED');}
   this.adopt(response.credentials);this.loginSubject=subject;this.pendingLogin=null;return this.credentials;
  }catch(e){if(this.pendingLogin===pending&&e instanceof ServiceError&&e.code!=='SERVICE_UNAVAILABLE')this.pendingLogin=null;throw e;}
 });}
 adopt(c){validate(this.schema,'Credentials',c);if(!c.accessToken||!c.refreshSecret||!c.sessionId||!c.sessionEpoch||!c.accountId||c.kind!=='browser'||c.deviceId!==this.device||!Number.isFinite(Date.parse(c.expiresAt)))throw new ServiceError('INVALID_MESSAGE');this.credentials=c;}
 envelope(payload,room){this.requireInitialized();return validate(this.schema,'Request',{major:1,revision:this.schema.revision,serviceId:this.info.serviceId,serviceEpoch:this.info.serviceEpoch,requestId:this.uuid(),...(this.credentials?{sessionEpoch:this.credentials.sessionEpoch}:{}),...(room?{roomId:room.roomId,expectedRevision:room.revision,configRevision:room.configRevision,runId:room.runId??'',runEpoch:room.runEpoch??''}:{}),...payload});}
 async send(request,auth=this.credentials,{deadline=this.now()+30000,guard=()=>{}}={}){
  const generation=this.generation;let result;
  for(let attempt=0;attempt<2;attempt++){
   guard();
   if(this.now()>=deadline||generation!==this.generation)throw new ServiceError('CANCELLED');
   try{result=await this.transport('/v1/rpc',request,auth?.accessToken);break;}catch(e){if(e instanceof ServiceError||attempt===1)throw e;}
  }
  guard();if(this.now()>=deadline||generation!==this.generation)throw new ServiceError('CANCELLED');validate(this.schema,'Response',result);
  if(result.major!==1||result.revision!==this.schema.revision||result.serviceId!==this.info.serviceId||result.requestId!==request.requestId||!result.serviceEpoch)throw new ServiceError('INVALID_MESSAGE');
  this.info.serviceEpoch=result.serviceEpoch;
  if(result.error){const error=new ServiceError(result.error.code,result.error.retryAfterMs??0);error.verifiedResponse=true;error.operation=request.finalStart?'finalStart':null;throw error;}return result;
 }
 refreshScope(p,checkEpoch=true){return this.pendingRefresh===p&&this.generation===p.generation&&this.transport===p.transport&&this.device===p.identity.deviceId&&sameDesktop(p.identity,this.credentials)&&this.credentials.refreshSecret===p.request.refresh.secret&&this.info.serviceId===p.serviceId&&(!checkEpoch||this.info.serviceEpoch===p.serviceEpoch)&&this.schema.major===p.major&&this.schema.revision===p.revision&&(this.info.profile?.id??'')===p.profileId;}
 async recoverRefresh(){
  if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');
  if(!this.pendingRefresh){
   const original=this.credentials,request=this.envelope({refresh:{secret:original.refreshSecret,deviceId:this.device}});Object.freeze(request.refresh);Object.freeze(request);
   this.pendingRefresh=Object.freeze({request,deadline:this.now()+30000,generation:this.generation,transport:this.transport,identity:Object.freeze({accountId:original.accountId,sessionId:original.sessionId,sessionEpoch:original.sessionEpoch,deviceId:original.deviceId,kind:original.kind}),serviceId:this.info.serviceId,serviceEpoch:this.info.serviceEpoch,major:this.schema.major,revision:this.schema.revision,profileId:this.info.profile?.id??''});
  }
  const pending=this.pendingRefresh;
  try{
   const guard=()=>{if(this.pendingRefresh!==pending||this.generation!==pending.generation)throw new ServiceError('CANCELLED');if(!this.refreshScope(pending))throw new ServiceError('STALE_EPOCH');if(this.now()>=pending.deadline)throw new ServiceError('EXPIRED');};guard();
   const response=await this.send(pending.request,null,{deadline:pending.deadline,guard});
   // A verified exact refresh reply can report a new service epoch after restart.
   // The original request was never retagged; all identity/profile fences still apply.
   if(this.now()>=pending.deadline)throw new ServiceError('EXPIRED');
   if(!this.refreshScope(pending,false)||this.info.serviceEpoch!==response.serviceEpoch||!sameDesktop(pending.identity,response.credentials))throw new ServiceError('STALE_EPOCH');
   this.adopt(response.credentials);this.pendingRefresh=null;
  }catch(e){
   const expired=this.now()>=pending.deadline;
   if(this.pendingRefresh===pending&&this.generation===pending.generation&&(expired||['EXPIRED','REFRESH_REUSED','UNAUTHENTICATED','FORBIDDEN','STALE_EPOCH','INVALID_MESSAGE'].includes(e.code)))this.dispose();
   if(expired)throw new ServiceError('EXPIRED');throw e;
  }
 }
 async renew(){return this.exclusive(()=>this.recoverRefresh());}
 async rpc(payload,room){return this.exclusive(async()=>{this.requireInitialized();if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');if(this.pendingRefresh||Date.parse(this.credentials.expiresAt)-this.now()<60000)await this.recoverRefresh();return this.send(this.envelope(payload,room));});}
 async mutateRoom(payload,room){return this.exclusive(async()=>{this.requireInitialized();if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');const type=Object.keys(payload)[0];if(Object.keys(payload).length!==1||!['roomCreate','configure','roomPolicy','roomAccess','roomModerate','cancelRun','endRun','roomDelete'].includes(type))throw new ServiceError('INVALID_MESSAGE');if(this.pendingRefresh||Date.parse(this.credentials.expiresAt)-this.now()<60000)await this.recoverRefresh();this.pendingMutations??=new Map();const key=type+':'+(room?.roomId??'new'),content=JSON.stringify(payload);let pending=this.pendingMutations.get(key);if(pending&&(pending.generation!==this.generation||pending.content!==content||pending.serviceId!==this.info.serviceId||pending.serviceEpoch!==this.info.serviceEpoch||pending.accountId!==this.credentials.accountId||pending.sessionId!==this.credentials.sessionId))throw new ServiceError('MUTATION_UNCERTAIN');if(!pending){if(this.pendingMutations.size>=8)throw new ServiceError('CAPACITY');pending={generation:this.generation,accountId:this.credentials.accountId,sessionId:this.credentials.sessionId,sessionEpoch:this.credentials.sessionEpoch,content,serviceId:this.info.serviceId,serviceEpoch:this.info.serviceEpoch,request:this.envelope(structuredClone(payload),room)};this.pendingMutations.set(key,pending);}try{if(pending.expired)throw new ServiceError('RECEIPT_EXPIRED');const result=await this.send(pending.request);if(type==='roomDelete'?!result.deletedRoomId:!result.room)throw new ServiceError('INVALID_MESSAGE');if(this.pendingMutations.get(key)===pending)this.pendingMutations.delete(key);return result;}catch(e){if(e.verifiedResponse&&e.code==='EXPIRED')pending.expired=true;if(this.pendingMutations.get(key)===pending&&e.verifiedResponse&&!['SERVICE_UNAVAILABLE','EXPIRED'].includes(e.code))this.pendingMutations.delete(key);throw e;}});}
 async retryRoomMutation(key){return this.exclusive(async()=>{this.requireInitialized();if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');if(this.pendingRefresh||Date.parse(this.credentials.expiresAt)-this.now()<60000)await this.recoverRefresh();const pending=this.pendingMutations?.get(key),c=this.credentials,generation=this.generation;if(!pending||!c||pending.generation!==generation||pending.serviceId!==this.info.serviceId||pending.serviceEpoch!==this.info.serviceEpoch||pending.accountId!==c.accountId||pending.sessionId!==c.sessionId)throw new ServiceError('STALE_EPOCH');if(pending.lane)return this.sendRuleRequest(key,pending,()=>{if(this.generation!==generation||this.credentials!==c||this.info.serviceId!==pending.serviceId||this.info.serviceEpoch!==pending.serviceEpoch)throw new ServiceError('STALE_EPOCH');});try{if(pending.expired)throw new ServiceError('RECEIPT_EXPIRED');const result=await this.send(pending.request);if(this.pendingMutations.get(key)===pending)this.pendingMutations.delete(key);return result;}catch(e){if(e.verifiedResponse&&e.code==='EXPIRED')pending.expired=true;if(this.pendingMutations.get(key)===pending&&e.verifiedResponse&&!['SERVICE_UNAVAILABLE','EXPIRED'].includes(e.code))this.pendingMutations.delete(key);throw e;}});}
 // Called only after an explicit UI review/acknowledgment. This forgets an
 // expired receipt; it neither declares the old action failed nor sends anything.
 acknowledgeExpiredOutcome(key,pending){
  const current=key==='chat'?this.pendingChat:this.pendingMutations?.get(key),c=this.credentials;
  if(this.active||!pending||current!==pending||!(pending.expired||pending.requiresReview)||!c||pending.generation!==this.generation||pending.accountId!==c.accountId||pending.sessionId!==c.sessionId||pending.sessionEpoch!==c.sessionEpoch||pending.serviceId!==this.info?.serviceId||pending.serviceEpoch!==this.info?.serviceEpoch)throw new ServiceError('STALE_EPOCH');
  if(key==='chat')this.pendingChat=null;else this.pendingMutations.delete(key);
 }
 async sendRoomChat(text,room){return this.exclusive(async()=>{this.requireInitialized();if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');if(this.pendingRefresh||Date.parse(this.credentials.expiresAt)-this.now()<60000)await this.recoverRefresh();const c=this.credentials;let pending=this.pendingChat;if(pending&&(pending.generation!==this.generation||pending.accountId!==c.accountId||pending.sessionId!==c.sessionId||pending.roomId!==room.roomId))this.pendingChat=pending=null;if(pending&&pending.text!==text)throw new ServiceError('CHAT_SEND_UNCERTAIN');if(!pending){const request=this.envelope({chatSend:{text}},room);pending={generation:this.generation,accountId:c.accountId,sessionId:c.sessionId,sessionEpoch:c.sessionEpoch,serviceId:this.info.serviceId,serviceEpoch:this.info.serviceEpoch,roomId:room.roomId,text,request};this.pendingChat=pending;}try{if(pending.expired)throw new ServiceError('RECEIPT_EXPIRED');const result=await this.send(pending.request);if(this.pendingChat===pending)this.pendingChat=null;return result;}catch(e){if(e.verifiedResponse&&e.code==='EXPIRED')pending.expired=true;if(this.pendingChat===pending&&e.verifiedResponse&&!['SERVICE_UNAVAILABLE','EXPIRED'].includes(e.code))this.pendingChat=null;throw e;}});}
 async logout(){return this.exclusive(async()=>{const old=this.credentials;const request=this.envelope({logout:{}});this.dispose();if(old)await this.send(request,old);});}
}
export async function fetchTransport(path,body,token,{keepalive=false,credentials='omit',csrf}={}){
 const reply=await fetch(path,{method:body?'POST':'GET',headers:{...(body?{'content-type':'application/json'}:{}),...(token?{authorization:'Bearer '+token}:{}),...(body&&csrf?{'x-nt2-csrf':csrf}:{})},...(body?{body:JSON.stringify(body)}:{}),...(keepalive?{keepalive:true}:{}),signal:AbortSignal.timeout(5000),cache:'no-store',credentials,redirect:'error'});
 const reader=reply.body.getReader();let size=0,text='';const decoder=new TextDecoder('utf-8',{fatal:true});
 try{for(;;){const {done,value}=await reader.read();if(done)break;size+=value.byteLength;if(size>65536)throw new ServiceError('CAPACITY');text+=decoder.decode(value,{stream:true});}text+=decoder.decode();}finally{await reader.cancel();}
 const value=JSON.parse(text);if(!reply.ok)throw new ServiceError(value.error?.code??'SERVICE_UNAVAILABLE',value.error?.retryAfterMs??1000);return value;
}
// Production website sessions carry only public identity/epoch metadata in the
// page. Access and refresh tokens stay in the service's HttpOnly cookie session.
export class CookieHostClient extends HostClient {
 constructor(transport=fetchTransport){let self;super((p,b,t,o={})=>transport(p,b,t,{...o,credentials:'same-origin',csrf:self?.csrf}));self=this;this.authentication=null;this.account=null;this.csrf='';}
 dispose(){super.dispose();this.account=null;this.csrf='';}
 async initialize(){const generation=this.generation,info=await super.initialize();try{const value=await this.transport('/v1/auth/info');if(generation!==this.generation||this.pageClosed||this.info!==info)throw new ServiceError('CANCELLED');if(value.revision!=='auth1-1'||value.serviceId!==info.serviceId||value.serviceEpoch!==info.serviceEpoch||!Array.isArray(value.providers)||value.providers.length!==3||value.providers.map(p=>p.id).sort().join(',')!=='google,local,twitch'||value.providers.some(p=>typeof p.available!=='boolean'))throw new ServiceError('VERSION_MISMATCH');this.authentication=value;await this.refreshAccount();}catch(e){if(e.code!=='NOT_FOUND'||info.mockProvider!==true)throw e;this.authentication=null;}return info;}
 async refreshAccount(){if(!this.authentication)return;const generation=this.generation,info=this.info,value=await this.transport('/v1/web/session');if(generation!==this.generation||this.pageClosed||this.info!==info)throw new ServiceError('CANCELLED');if(!value.authenticated){this.dispose();return;}if(!value.session||value.session.kind!=='browser'||value.account?.accountId!==value.session.accountId||!['user','power-user','admin','dev'].includes(value.account.role)||Object.hasOwn(value.session,'accessToken')||Object.hasOwn(value.session,'refreshSecret'))throw new ServiceError('INVALID_MESSAGE');if(this.credentials&&!sameDesktop(this.credentials,value.session))this.dispose();this.credentials=value.session;this.device=value.session.deviceId;this.account=value.account;this.csrf=value.csrf;}
 async recoverRefresh(){if(!this.authentication)return super.recoverRefresh();await this.refreshAccount();if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');}
 async login(subject){if(this.authentication)throw new ServiceError('FORBIDDEN');return super.login(subject);}
 async logout(){if(!this.authentication)return super.logout();try{await this.transport('/v1/web/logout',{});}finally{this.dispose();this.account=null;this.csrf='';}}
 async closePage(){if(!this.authentication)return super.closePage();this.pageClosed=true;this.dispose();return {confirmed:true};}
}
