import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,randomBytes} from 'node:crypto';
import {generateKeyPair,exportPKCS8,exportSPKI} from 'jose';
import pg from 'pg';
import {createCandidateApp} from '../src/g2-next/app.js';
import {hash} from '../src/g1/security.js';
// @ts-expect-error Browser-native module exercised with injected HTTP transport.
import {HostClient} from '../../web/candidate/client.js';
const url=process.env.NT2_G2_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
test('candidate HTTP serves separate UI and negotiates named mock host and default-off guest admission',async()=>{
 const namespace='g2_next_http_'+randomUUID().replaceAll('-',''),pair=await generateKeyPair('EdDSA',{extractable:true});const service=await createCandidateApp({databaseUrl:url,serviceId:'nt2-g2-next-http',namespace,keys:{privateKey:await exportPKCS8(pair.privateKey),publicKey:await exportSPKI(pair.publicKey),recoveryKey:randomBytes(32).toString('base64url'),issuer:'http://127.0.0.1:4354'},mockEnabled:true,website:true,compatibility:{gameVersion:'test',modVersion:'test',bridgeVersion:'test',capabilities:['core.seed'],modes:['core'],settingKeys:[]}});
 try{
  const page=await service.app.inject('/');assert.equal(page.statusCode,200);assert.match(page.body,/Allow guests/);assert.equal(page.headers['referrer-policy'],'no-referrer');assert.match(page.body,/Start inspected players/);assert.match(page.headers['content-security-policy'] as string,/frame-ancestors 'none'/);
  const transport=async(path:string,body?:unknown,token?:string)=>{const response=await service.app.inject({method:body?'POST':'GET',url:path,...(body?{payload:body}:{}),headers:token?{authorization:'Bearer '+token}:{}});assert.equal(response.statusCode,200,response.body);return response.json();};
  const web=new HostClient(transport);await web.initialize();await web.login('website-local-alex');assert.equal(web.credentials.principal.source,'mock:website');
  const created=await web.rpc({roomCreate:{name:'HTTP candidate',config:{seed:1,mode:'core',requiredCapabilities:['core.seed']}}});assert.equal(created.error,undefined);const room=created.room;assert.equal(room.allowGuests,false);
  const base={major:1,revision:'g2-draft-5',serviceId:service.db.serviceId,serviceEpoch:service.db.epoch,requestId:randomUUID()};const issued=await transport('/v1/rpc',{...base,guestIssue:{deviceId:'guest-http',displayName:'Alex',issuanceProof:randomBytes(32).toString('base64url')}});assert.equal(issued.credentials.principal.kind,'guest');
  const joined=await transport('/v1/rpc',{...base,requestId:randomUUID(),sessionEpoch:issued.credentials.sessionEpoch,roomId:room.roomId,configRevision:1,expectedRevision:room.revision,roomJoin:{}},issued.credentials.accessToken);assert.equal(joined.error.code,'ACCOUNT_REQUIRED');
  assert.equal((await service.app.inject({method:'POST',url:'/v1/rpc',payload:{...base,start:{}}})).json().error.code,'INVALID_MESSAGE');
  assert.equal((await service.app.inject({url:'/v1/info',headers:{origin:'https://other.invalid'}})).statusCode,403);
  const initiatorProof=randomBytes(32).toString('base64url'),callbackState=randomBytes(32).toString('base64url'),beginRequest={...base,requestId:randomUUID(),authBegin:{provider:'twitch',deviceId:'desktop',appInstanceId:'app',proofChallenge:hash(initiatorProof),callbackPort:49152,callbackState}};
  const begun=await transport('/v1/rpc',beginRequest),again=await transport('/v1/rpc',beginRequest);assert.equal(begun.authAttempt.attemptId,again.authAttempt.attemptId);assert.equal(begun.authAttempt.launchUrl.includes(initiatorProof),false);
  const path=new URL(begun.authAttempt.launchUrl).pathname,signinPage=await service.app.inject(path);assert.equal(signinPage.statusCode,200);assert.equal(signinPage.headers['referrer-policy'],'origin');const cookie=String(signinPage.headers['set-cookie']).split(';')[0],csrf=signinPage.body.match(/name="csrf" value="([^"]+)"/)![1]!;
  const submit=()=>service.app.inject({method:'POST',url:path,headers:{origin:'http://127.0.0.1:4354',cookie,'content-type':'application/x-www-form-urlencoded'},payload:new URLSearchParams({csrf,subject:'alex'}).toString()});
  assert.equal((await service.app.inject({method:'POST',url:path,headers:{origin:'http://127.0.0.1:4354',cookie,'content-type':'application/x-www-form-urlencoded'},payload:new URLSearchParams({csrf:randomBytes(32).toString('base64url'),subject:'alex'}).toString()})).statusCode,403);
  for(const origin of ['null','https://other.invalid'])assert.equal((await service.app.inject({method:'POST',url:path,headers:{origin,cookie,'content-type':'application/x-www-form-urlencoded'},payload:new URLSearchParams({csrf,subject:'alex'}).toString()})).statusCode,403);
  const returned=await submit(),retry=await submit();assert.equal(returned.headers['referrer-policy'],'origin');assert.equal(returned.statusCode,200);assert.equal(returned.body,retry.body);assert.match(returned.headers['content-security-policy'] as string,/form-action http:\/\/127.0.0.1:49152\/nt2-return/);
  const fields=Object.fromEntries([...returned.body.matchAll(/name="([^"]+)" value="([^"]+)"/g)].map(m=>[m[1],m[2]]));assert.equal(fields.callbackState,callbackState);
  const proof={attemptId:begun.authAttempt.attemptId,deviceId:'desktop',appInstanceId:'app',initiatorProof},status=await transport('/v1/rpc',{...base,requestId:randomUUID(),authStatus:proof});assert.equal(status.authAttempt.state,'completed');assert.equal(JSON.stringify(status).includes(fields.completionGrant),false);
  const exchange={...base,requestId:randomUUID(),authExchange:{...proof,completionGrant:fields.completionGrant}},signedIn=await transport('/v1/rpc',exchange);assert.equal(signedIn.credentials.principal.source,'mock:twitch');assert.equal(signedIn.credentials.kind,'desktop');assert.equal((await transport('/v1/rpc',exchange)).credentials.sessionId,signedIn.credentials.sessionId);

 }finally{await service.app.close();const pool=new pg.Pool({connectionString:url});try{await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);}finally{await pool.end();}}
});
