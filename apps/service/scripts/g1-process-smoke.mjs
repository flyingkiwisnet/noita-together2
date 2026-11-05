import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
import {once} from 'node:events';
const root=fileURLToPath(new URL('../../../',import.meta.url));
const corrective=process.argv.includes('--corrective');
const origin=corrective?'http://127.0.0.1:4341':'http://127.0.0.1:4321';
let child;
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function start(){
 child=spawn(process.execPath,['dist/apps/service/src/g1/main.js'],{cwd:root,env:{...process.env,NT2_G1_MOCK:'1',...(corrective?{NT2_G1_PORT:'4341',NT2_G1_SERVICE_ID:'nt2-g1-process-corrective',NT2_G1_NAMESPACE:'g1_process_corrective',NT2_G1_KEYS_PATH:'secrets/g1-corrective-4341.json'}:{})},stdio:'ignore',windowsHide:true});
 for(let i=0;i<100;i++){
  if(child.exitCode!==null)throw Error('Compiled G1 process failed startup');
  try{const result=await fetch(origin+'/v1/info');if(result.ok)return await result.json();}catch{}
  await delay(50);
 }
 throw Error('Compiled G1 process did not become available');
}
async function stop(){if(child&&child.exitCode===null){const exit=once(child,'exit');child.kill('SIGTERM');await exit;}child=undefined;}
async function send(path,body,token){const response=await fetch(origin+path,{method:'POST',headers:{'content-type':'application/json',...(token?{authorization:'Bearer '+token}:{})},body:JSON.stringify(body)});return await response.json();}
try{
 let occupied=false;try{await fetch(origin+'/v1/info');occupied=true;}catch{}
 if(occupied)throw Error(`Process smoke requires ${origin} free; refuses to replace another service`);
 let info=await start();
 const envelope=(payload,session)=>({major:info.major,revision:info.revision,serviceId:info.serviceId,serviceEpoch:info.serviceEpoch,requestId:randomUUID(),...(session?{sessionEpoch:session.sessionEpoch}:{}),...payload});
 const login=await send('/v1/mock/browser-login',{subject:'process_'+randomUUID().replaceAll('-',''),deviceId:'process-tab',requestId:randomUUID()});assert.ok(login.credentials);
 const old=login.credentials;
 const created=await send('/v1/rpc',envelope({roomCreate:{name:'Process restart fixture',config:{seed:424242,mode:'core',settings:[],requiredCapabilities:['core.seed']}}},old),old.accessToken);assert.ok(created.room);
 const exactRefresh=envelope({refresh:{secret:old.refreshSecret,deviceId:old.deviceId}},old);const rotated=await send('/v1/rpc',exactRefresh);assert.ok(rotated.credentials);
 const firstEpoch=info.serviceEpoch;await stop();info=await start();assert.notEqual(info.serviceEpoch,firstEpoch);
 const retry=await send('/v1/rpc',exactRefresh);assert.deepEqual(retry.credentials,rotated.credentials);
 const current=retry.credentials;const restored=await send('/v1/rpc',envelope({roomId:created.room.roomId,roomGet:{}},current),current.accessToken);assert.ok(restored.room);assert.equal(restored.room.state,'waiting');assert.equal(restored.room.config.seed,424242);
 const exactLogout=envelope({logout:{}},current);assert.equal((await send('/v1/rpc',exactLogout,current.accessToken)).error,undefined);
 await stop();info=await start();assert.equal((await send('/v1/rpc',exactLogout,current.accessToken)).error,undefined);
 assert.equal((await send('/v1/rpc',envelope({roomList:{}},current),current.accessToken)).error.code,'UNAUTHENTICATED');
 console.log('PASS compiled Node process restart: room/seed persistence, exact refresh recovery and durable logout retry/revocation.');
}finally{await stop();}
