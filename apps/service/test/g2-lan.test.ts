import test from 'node:test';
import assert from 'node:assert/strict';
import {lanConfig,validateLanConfig,privateIPv4,localInterface} from '../src/g2/lan.js';
import {spawnSync} from 'node:child_process';
test('LAN config binds canonical HTTPS origin, issuer and independent identity to literal RFC1918 IPv4',()=>{
 for(const ip of ['10.1.2.3','172.16.0.1','172.31.255.254','192.168.0.104']){
  const config=lanConfig(ip,4352);assert.equal(config.origin,`https://${ip}:4352`);assert.equal(config.issuer,config.origin);assert.deepEqual(validateLanConfig(config),config);
 }
 for(const ip of ['127.0.0.1','0.0.0.0','::1','::','192.168.0.104.example','192.168.000.104','172.15.0.1','172.32.0.1','8.8.8.8','169.254.1.2','localhost','192.168.0.104/24','192.168.0.104:4352']){assert.equal(privateIPv4(ip),false);assert.throws(()=>lanConfig(ip,4352));}
 for(const port of [0,80,1023,65536,4352.5,NaN,'4352'])assert.throws(()=>lanConfig('192.168.0.104',port));
 const config=lanConfig('192.168.0.104',4352);
 for(const change of [{origin:'http://192.168.0.104:4352'},{issuer:'https://192.168.0.105:4352'},{origin:'https://192.168.0.104:4352/'},{serviceId:'nt2-g2-local'},{namespace:'g2_dev'},{extra:true}])assert.throws(()=>validateLanConfig({...config,...change}));
 assert.throws(()=>localInterface(config.ip,{}));
});
test('G2 LAN launcher refuses production and missing explicit mock activation before initialization',()=>{
 for(const env of [{NODE_ENV:'production',NT2_G2_MOCK:'1'},{NODE_ENV:'development',NT2_G2_MOCK:''}]){
  const result=spawnSync(process.execPath,['--import','tsx','apps/service/src/g2/main.ts'],{env:{...process.env,...env,NT2_G2_LAN_CONFIG:'not-used'},encoding:'utf8',timeout:10000});
  assert.equal(result.status,1);assert.match(result.stderr,/requires NT2_G2_MOCK=1 and refuses production/);
 }
});
