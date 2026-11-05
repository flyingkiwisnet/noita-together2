import {randomUUID} from 'node:crypto';
import type {FastifyInstance} from 'fastify';
import type {Request,Response,Credentials,Room} from '../../contracts/g2/g2.js';
import {schema,encode,decode} from '../../contracts/src/g2.js';
// Simulated consumers only. Production Rust owns native context and credential storage.
export const g2Compatibility={gameVersion:'g1-fixture',modVersion:'g1-draft-1',bridgeVersion:'g0-pipe32-1',capabilities:['core.seed','core.ghost','core.gold','core.items','core.emotes','core.cosmetics','integration.nemesis'],modes:['core','nemesis'],settingKeys:['friendlyFire','sharedHealth','respawn','bankEnabled']};
export class G2Consumer {
 constructor(public app:FastifyInstance,public serviceId:string,public serviceEpoch:string,public credentials:Credentials|undefined=undefined){}
 request(payload:Partial<Request>,room?:Room):Request{return {major:schema.major,revision:schema.revision,serviceId:this.serviceId,requestId:randomUUID(),serviceEpoch:this.serviceEpoch,...(this.credentials?{sessionEpoch:this.credentials.sessionEpoch??''}:{}),...(room?{roomId:room.roomId??'',expectedRevision:room.revision??0,configRevision:room.configRevision??0,runId:room.runId??'',runEpoch:room.runEpoch??''}:{}),...payload};}
 async send(request:Request,binary=false):Promise<Response>{
  const response=await this.app.inject({method:'POST',url:'/v1/rpc',headers:{...(this.credentials?{authorization:'Bearer '+this.credentials.accessToken}:{}),'content-type':binary?'application/x-protobuf':'application/json'},payload:binary?Buffer.from(encode('Request',request)):request});
  return (binary?decode('Response',response.rawPayload):response.json()) as Response;
 }
 async rpc(payload:Partial<Request>,room?:Room,binary=false){return this.send(this.request(payload,room),binary);}
 async browser(subject:string,deviceId:string){const response=await this.app.inject({method:'POST',url:'/v1/mock/browser-login',payload:{subject,deviceId,requestId:randomUUID()}});this.credentials=response.json().credentials;if(!this.credentials)throw Error('Fixture login failed');return this;}
 async desktop(browser:G2Consumer,deviceId:string){
  const begin=await this.rpc({pairBegin:{deviceId}});if(!begin.pairing)throw Error('Fixture pairing begin failed');
  const p=begin.pairing;const approved=await browser.rpc({pairApprove:{attemptId:p.attemptId!,code:p.code!}});if(approved.error)throw Error('Fixture approval failed: '+approved.error.code);
  const exchange=await this.rpc({pairExchange:{attemptId:p.attemptId!,proof:p.proof!,deviceId}});if(!exchange.credentials)throw Error('Fixture pairing exchange failed');this.credentials=exchange.credentials;return this;
 }
}
