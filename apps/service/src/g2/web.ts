import type {FastifyInstance} from 'fastify';
import {readFile} from 'node:fs/promises';
export async function registerWebsite(app:FastifyInstance){
 const assets=[['/','index.html','text/html; charset=utf-8'],['/app.js','app.js','text/javascript; charset=utf-8'],['/client.js','client.js','text/javascript; charset=utf-8'],['/style.css','style.css','text/css; charset=utf-8']] as const;
 for(const [path,file,type] of assets){
  const contents=await readFile(new URL('../../../../apps/web/public/'+file,import.meta.url));
  app.get(path,async(_req,reply)=>reply.type(type).headers({'content-security-policy':"default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",'x-content-type-options':'nosniff','referrer-policy':'no-referrer','cache-control':'no-store','permissions-policy':'camera=(), microphone=(), geolocation=()'}).send(contents));
 }
 const schema=await readFile(new URL('../../../../packages/contracts/g2/schema.json',import.meta.url));
 app.get('/schema.json',async(_req,reply)=>reply.type('application/json').header('cache-control','no-store').send(schema));
}
