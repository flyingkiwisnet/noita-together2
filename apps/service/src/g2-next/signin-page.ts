import type {FastifyInstance} from 'fastify';
import {demand,hash,matches,secret} from '../g1/security.js';
import {MockSignIn} from './signin.js';
const escape=(s:string)=>s.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export async function registerMockSignInPage(app:FastifyInstance,signin:MockSignIn,tls:boolean){
 const auth=signin.auth;
 app.addContentTypeParser('application/x-www-form-urlencoded',{parseAs:'string'},(_req,body,done)=>{try{const parsed=new URLSearchParams(body as string),entries=[...parsed];demand(entries.length===2&&new Set(entries.map(([key])=>key)).size===2&&parsed.has('csrf')&&parsed.has('subject'),'INVALID_MESSAGE');const value=Object.fromEntries(entries);demand(/^[A-Za-z0-9_-]{43}$/.test(value.csrf??'')&&['alex','sam'].includes(value.subject??''),'INVALID_MESSAGE');done(null,value);}catch(e){done(e as Error);}});
 const page=(reply:any,body:string,formAction="'self'")=>reply.type('text/html; charset=utf-8').headers({'content-security-policy':`default-src 'none'; script-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action ${formAction}`,'cache-control':'no-store','referrer-policy':'origin','x-content-type-options':'nosniff'}).send('<!doctype html><html lang="en"><meta charset="utf-8"><title>NT2 mock sign-in</title><body>'+body+'</body></html>');
 app.get('/signin-return.js',async(_req,reply)=>reply.type('text/javascript').header('cache-control','no-store').send("document.getElementById('return').submit();"));
 app.get<{Params:{id:string}}>('/signin/:id',async(req,reply)=>{
  demand(/^[a-f0-9-]{36}$/.test(req.params.id),'NOT_FOUND');
  const row=await auth.db.transaction(async tx=>{const row=(await tx.query('SELECT * FROM account_signin_attempts WHERE id=$1',[req.params.id])).rows[0];demand(row&&row.service_epoch===auth.db.epoch&&row.state==='pending'&&+row.expires_at>auth.now(),'EXPIRED');return row;});
  const csrf=secret(),cookie=auth.security.seal({attemptId:row.id,csrf,until:auth.now()+300000});reply.header('set-cookie',`nt2_mock_return=${cookie}; HttpOnly; SameSite=Strict; Path=/signin/${row.id}; Max-Age=300${tls?'; Secure':''}`);
  return page(reply,`<h1>Mock ${row.provider==='twitch'?'Twitch':'website account'} sign-in</h1><p>Local development accounts only. Choose your account to return automatically to the initiating app.</p><form method="post" action="/signin/${row.id}"><input type="hidden" name="csrf" value="${csrf}"><button name="subject" value="alex">Mock account · Alex</button><button name="subject" value="sam">Mock account · Sam</button></form>`);
 });
 app.post<{Params:{id:string};Body:{csrf:string;subject:string}}>('/signin/:id',async(req,reply)=>{
  demand(req.headers.origin===auth.security.keys.issuer,'FORBIDDEN');const raw=req.headers.cookie?.split(';').map(x=>x.trim()).find(x=>x.startsWith('nt2_mock_return='))?.slice('nt2_mock_return='.length);demand(raw&&raw.length<=1024,'FORBIDDEN');
  const context=auth.security.open<{attemptId:string;csrf:string;until:number}>(raw);demand(context.attemptId===req.params.id&&context.until>auth.now()&&matches(context.csrf,req.body.csrf),'FORBIDDEN');
  const result=await auth.db.transaction(tx=>signin.complete(tx,req.params.id,req.body.subject,req.body.subject==='alex'?'Alex':'Sam',hash(context.csrf)));
  const fields=Object.entries(result.body).map(([key,value])=>`<input type="hidden" name="${escape(key)}" value="${escape(value)}">`).join('');
  return page(reply,`<h1>Mock account signed in</h1><p>Returning to your app. If the browser cannot complete this local return, keep the app open and use Retry or Cancel there.</p><form id="return" method="post" action="${escape(result.url)}">${fields}<button>Return to app</button></form><script src="/signin-return.js"></script>`,result.url);
 });
}
