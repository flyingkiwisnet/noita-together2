// Isolated transport fixture only. Temporary private keys never enter the distribution.
import https from 'node:https';
import fs from 'node:fs';
const [pfx,ready,ip]=process.argv.slice(2);
if(!pfx||!ready||ip!=='192.168.0.104')throw Error('Use the documented local private-IP transport harness');
const server=https.createServer({pfx:fs.readFileSync(pfx),passphrase:'nt2-generated-fixture-only',minVersion:'TLSv1.2'},(req,res)=>{
    if(req.method==='GET'&&req.url==='/probe'){res.writeHead(200,{'content-type':'text/plain'});res.end('nt2-private-ip-tls-ok');}else{res.writeHead(404);res.end();}
});
server.listen(0,ip,()=>fs.writeFileSync(ready,JSON.stringify({port:server.address().port,pid:process.pid})));
