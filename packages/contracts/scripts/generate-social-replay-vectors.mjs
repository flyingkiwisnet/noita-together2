import {createHash} from 'node:crypto';
import {writeFile,readFile} from 'node:fs/promises';
const scope={roomId:'11111111-1111-4111-8111-111111111111',runId:'22222222-2222-4222-8222-222222222222',runEpoch:'33333333-3333-4333-8333-333333333333',configRevision:7,attachmentId:'44444444-4444-4444-8444-444444444444',worldToken:'world-雪'};
const gameSessionId='55555555-5555-4555-8555-555555555555',policy='sequence-bound-transient-v1';
const vectors=['hud','emote','cosmetic'].map((kind,index)=>{
 const sequence=index===2?4294967295:index+1;
 const canonicalJson=JSON.stringify([policy,scope.roomId,scope.runId,scope.runEpoch,scope.configRevision,scope.attachmentId,scope.worldToken,gameSessionId,kind,sequence]);
 const sha256=createHash('sha256').update(canonicalJson).digest('hex'),b=Buffer.from(sha256.slice(0,32),'hex');b[6]=(b[6]&15)|128;b[8]=(b[8]&63)|128;
 const h=b.toString('hex'),eventId=`${h.slice(0,8)}-${h.slice(8,12)}-${h.slice(12,16)}-${h.slice(16,20)}-${h.slice(20)}`;
 return {scope,gameSessionId,kind,sequence,canonicalJson,utf8Hex:Buffer.from(canonicalJson).toString('hex'),sha256,eventId};
});
const path=new URL('../g3-social/replay-policy.json',import.meta.url),body=JSON.stringify({policy,recordsPerSenderKind:64,poiUuidVersion8Reserved:true,vectors},null,2)+'\n';
if(process.argv.includes('--check')){if(await readFile(path,'utf8')!==body)throw Error('Social replay vectors differ');}
else await writeFile(path,body);
