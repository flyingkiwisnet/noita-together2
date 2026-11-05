// Test-only HTTP transport around the server's unchanged G1 application.
// Fault controls exist only here, never in the desktop or service distribution.
import http from 'node:http';
import {generateKeyPairSync,randomBytes,randomUUID,createHash} from 'node:crypto';
import pg from 'pg';
import {createG1App} from '../../../dist/apps/service/src/g1/app.js';
const g2=process.argv.includes('--g2'),port=g2?4326:4323,serviceId=g2?'nt2-client-g2-test':'nt2-client-g1-test',namespace=(g2?'client_g2_':'client_g1_')+randomUUID().replaceAll('-','');
const databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
const pair=generateKeyPairSync('ed25519');
const keys={privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}),publicKey:pair.publicKey.export({type:'spki',format:'pem'}),recoveryKey:randomBytes(32).toString('base64url'),issuer:`http://127.0.0.1:${port}`};
let offset=0,provider=true,drop='',delay='',attachmentAudit=[],service;
async function start(){service=await createG1App({databaseUrl,namespace,serviceId,keys,mockEnabled:true,protocolG2:g2,website:g2,profile:g2?{id:'g2-client-simulated',evidenceStatus:'simulated',gameExeSha256:'0'.repeat(64),originalLuaSha256:'0'.repeat(64),bridgeSha256:'0'.repeat(64),modSha256:'0'.repeat(64)}:undefined,now:()=>Date.now()+offset,providerAvailable:()=>provider,compatibility:{gameVersion:'g1-fixture',modVersion:'g1-draft-1',bridgeVersion:'g0-pipe32-1',capabilities:['core.seed'],modes:['core'],settingKeys:[]}});}
await start();
let serial=Promise.resolve();
const server=http.createServer((req,res)=>{if(req.url==='/fixture/attachment-audit'&&req.method==='GET'){res.writeHead(200,{'content-type':'application/json'}).end(JSON.stringify(attachmentAudit));return;}serial=serial.then(async()=>{
    const chunks=[];let size=0;for await(const chunk of req){size+=chunk.length;if(size>65536){res.writeHead(413).end();return;}chunks.push(chunk);}const bytes=Buffer.concat(chunks);
    if(req.url==='/fixture/control'&&req.method==='POST'){const cmd=JSON.parse(bytes);if(cmd.reset){await service.app.close();const pool=new pg.Pool({connectionString:databaseUrl});if(!/^client_g[12]_[a-f0-9]{32}$/.test(namespace))throw Error('Unsafe fixture namespace');await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);await pool.end();offset=0;provider=true;drop='';delay='';attachmentAudit=[];await start();}else if(cmd.restart){await service.app.close();await start();}if(cmd.drop)drop=cmd.drop;if(cmd.delay)delay=cmd.delay;if(cmd.advance)offset+=cmd.advance;if(typeof cmd.provider==='boolean')provider=cmd.provider;res.writeHead(200,{'content-type':'application/json'}).end('{}');return;}
    if(req.url==='/fixture/attachment-audit'&&req.method==='GET'){res.writeHead(200,{'content-type':'application/json'}).end(JSON.stringify(attachmentAudit));return;}
    const result=await service.app.inject({method:req.method,url:req.url,headers:req.headers,payload:bytes.length?bytes:undefined});
    if(req.url==='/v1/rpc'&&bytes.length){const request=JSON.parse(bytes);if(request.gameAttach){const response=JSON.parse(result.rawPayload);attachmentAudit.push({requestId:request.requestId,bodySha256:createHash('sha256').update(bytes).digest('hex'),sessionId:response.credentials?.sessionId??null,attachmentId:response.credentials?.attachmentId??null,error:response.error?.code??null});}if(delay&&request[delay]){delay='';await new Promise(r=>setTimeout(r,300));}}
    if(drop&&req.url==='/v1/rpc'){const body=JSON.parse(bytes);if(body[drop]){drop='';req.socket.destroy();return;}}
    res.writeHead(result.statusCode,result.headers);res.end(result.rawPayload);
}).catch(()=>{if(!res.headersSent)res.writeHead(500);res.end();});});
server.listen(port,'127.0.0.1',()=>console.log('Temporary client G'+(g2?2:1)+' fault fixture listening on '+port+'; isolated database namespace; simulated game facts.'));
let closing=false;async function close(){if(closing)return;closing=true;await new Promise(r=>server.close(r));await serial;await service.app.close();const pool=new pg.Pool({connectionString:databaseUrl});if(!/^client_g[12]_[a-f0-9]{32}$/.test(namespace))throw Error('Unsafe fixture namespace');await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);await pool.end();}
for(const signal of ['SIGINT','SIGTERM'])process.once(signal,()=>{void close().then(()=>process.exit(0));});
