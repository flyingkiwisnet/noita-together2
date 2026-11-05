// ISOLATED POLICY CANDIDATE. Shared web files remain untouched until Root release.
import {HostClient,ServiceError,validate} from '../../candidate/client.js';
export const browserSignInCapacityMessage='Browser sign-in is at capacity. Sign out of an unused tab for this account, then try again. Abandoned sign-ins become reclaimable after their access credentials expire plus 30 minutes.';
export class BrowserSessionCandidate extends HostClient{
 constructor(...args){super(...args);this.pendingLogin=null;this.loginSubject=null;this.pageClosed=false;}
 dispose(){super.dispose();this.pendingLogin=null;this.loginSubject=null;}
 async revokeBrowser(credentials){
  if(credentials?.kind!=='browser'||credentials.deviceId!==this.device)return false;
  const request=this.envelope({logout:{},sessionEpoch:credentials.sessionEpoch});
  try{const value=await this.transport('/v1/rpc',request,credentials.accessToken,{keepalive:true});validate(this.schema,'Response',value);return !value.error;}catch{return false;}
 }
 async closePage(){const old=this.credentials;this.pageClosed=true;this.dispose();return {confirmed:old?await this.revokeBrowser(old):false};}
 restorePage(){this.dispose();this.pageClosed=false;}
 async login(subject){return this.exclusive(async()=>{
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
}
