export class ServiceError extends Error {
 constructor(code,retryAfterMs=0){super(code);this.code=code;this.retryAfterMs=retryAfterMs;}
}
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
export function sameDesktop(a,b){return !!a&&!!b&&['sessionId','sessionEpoch','accountId','deviceId','kind'].every(k=>a[k]===b[k]);}
export class HostClient {
 constructor(transport,uuid=()=>crypto.randomUUID(),now=()=>Date.now()){this.transport=transport;this.uuid=uuid;this.now=now;this.credentials=null;this.pendingRefresh=null;this.info=null;this.schema=null;this.active=false;this.generation=0;this.device='browser-'+uuid();}
 dispose(){this.credentials=null;this.pendingRefresh=null;++this.generation;}
 async exclusive(fn){if(this.active)throw new ServiceError('CAPACITY',250);this.active=true;try{return await fn();}finally{this.active=false;}}
 async initialize(){return this.exclusive(async()=>{const [schema,info]=await Promise.all([this.transport('/schema.json'),this.transport('/v1/info')]);if(schema.major!==1||schema.revision!=='g2-draft-1')throw new ServiceError('VERSION_MISMATCH');validate(schema,'Info',info);if(info.major!==schema.major||info.revision!==schema.revision||!info.serviceId||!info.serviceEpoch)throw new ServiceError('VERSION_MISMATCH');this.schema=schema;this.info=info;return info;});}
 async login(subject){return this.exclusive(async()=>{this.dispose();const generation=this.generation;const body=validate(this.schema,'BrowserLogin',{subject,deviceId:this.device,requestId:this.uuid()});const response=await this.transport('/v1/mock/browser-login',body);validate(this.schema,'BrowserLoginResponse',response);if(response.error)throw new ServiceError(response.error.code);if(generation!==this.generation)throw new ServiceError('CANCELLED');this.adopt(response.credentials);return this.credentials;});}
 adopt(c){validate(this.schema,'Credentials',c);if(!c.accessToken||!c.refreshSecret||!c.sessionId||!c.sessionEpoch||!c.accountId||c.kind!=='browser'||c.deviceId!==this.device||!Number.isFinite(Date.parse(c.expiresAt)))throw new ServiceError('INVALID_MESSAGE');this.credentials=c;}
 envelope(payload,room){return validate(this.schema,'Request',{major:1,revision:this.schema.revision,serviceId:this.info.serviceId,serviceEpoch:this.info.serviceEpoch,requestId:this.uuid(),...(this.credentials?{sessionEpoch:this.credentials.sessionEpoch}:{}),...(room?{roomId:room.roomId,expectedRevision:room.revision,configRevision:room.configRevision,runId:room.runId??'',runEpoch:room.runEpoch??''}:{}),...payload});}
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
  if(result.error)throw new ServiceError(result.error.code,result.error.retryAfterMs??0);return result;
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
 async rpc(payload,room){return this.exclusive(async()=>{if(!this.credentials)throw new ServiceError('UNAUTHENTICATED');if(this.pendingRefresh||Date.parse(this.credentials.expiresAt)-this.now()<60000)await this.recoverRefresh();return this.send(this.envelope(payload,room));});}
 async logout(){return this.exclusive(async()=>{const old=this.credentials;const request=this.envelope({logout:{}});this.dispose();if(old)await this.send(request,old);});}
}
export async function fetchTransport(path,body,token){
 const reply=await fetch(path,{method:body?'POST':'GET',headers:{...(body?{'content-type':'application/json'}:{}),...(token?{authorization:'Bearer '+token}:{})},...(body?{body:JSON.stringify(body)}:{}),signal:AbortSignal.timeout(5000),cache:'no-store',credentials:'omit',redirect:'error'});
 const reader=reply.body.getReader();let size=0,text='';const decoder=new TextDecoder('utf-8',{fatal:true});
 try{for(;;){const {done,value}=await reader.read();if(done)break;size+=value.byteLength;if(size>65536)throw new ServiceError('CAPACITY');text+=decoder.decode(value,{stream:true});}text+=decoder.decode();}finally{await reader.cancel();}
 const value=JSON.parse(text);if(!reply.ok)throw new ServiceError(value.error?.code??'SERVICE_UNAVAILABLE',value.error?.retryAfterMs??1000);return value;
}
