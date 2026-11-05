import {mkdir,writeFile} from 'node:fs/promises';
import {generateKeyPairSync,randomBytes} from 'node:crypto';
const folder=new URL('../../../secrets/',import.meta.url);await mkdir(folder,{recursive:true});
const pair=generateKeyPairSync('ed25519');
const keys={privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}).toString(),publicKey:pair.publicKey.export({type:'spki',format:'pem'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:'http://127.0.0.1:4360'};
await writeFile(new URL('g3-dev.json',folder),JSON.stringify(keys,null,2)+'\n',{flag:'wx',mode:0o600});
console.log('Created ignored secrets/g3-dev.json; preserve this separate key file for G3 restart recovery.');
