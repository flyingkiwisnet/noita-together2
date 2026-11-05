import {mkdir,writeFile} from 'node:fs/promises';
import {generateKeyPair,exportPKCS8,exportSPKI} from 'jose';
import {randomBytes} from 'node:crypto';
const folder=new URL('../../../secrets/',import.meta.url);await mkdir(folder,{recursive:true});
const {privateKey,publicKey}=await generateKeyPair('EdDSA',{extractable:true});
const file='g2-dev-4351.json';
const keys={privateKey:await exportPKCS8(privateKey),publicKey:await exportSPKI(publicKey),recoveryKey:randomBytes(32).toString('base64url'),issuer:'http://127.0.0.1:4351'};
// Refuse overwrite: existing refresh recovery requires the same key after restart.
await writeFile(new URL(file,folder),JSON.stringify(keys,null,2)+'\n',{flag:'wx',mode:0o600});console.log(`Created ignored secrets/${file}. Preserve this file for restart fixtures.`);
