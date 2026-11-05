import {mkdir,writeFile} from 'node:fs/promises';
import {generateKeyPair,exportPKCS8,exportSPKI} from 'jose';
import {randomBytes} from 'node:crypto';
const folder=new URL('../../../secrets/',import.meta.url);await mkdir(folder,{recursive:true});
const {privateKey,publicKey}=await generateKeyPair('EdDSA',{extractable:true});
const corrective=process.argv.includes('--corrective'),file=corrective?'g1-corrective-4341.json':'g1-dev.json';
const keys={privateKey:await exportPKCS8(privateKey),publicKey:await exportSPKI(publicKey),recoveryKey:randomBytes(32).toString('base64url'),issuer:corrective?'http://127.0.0.1:4341':'http://127.0.0.1:4321'};
// Refuse overwrite: existing refresh recovery requires the same key after restart.
await writeFile(new URL(file,folder),JSON.stringify(keys,null,2)+'\n',{flag:'wx',mode:0o600});console.log(`Created ignored secrets/${file}. Preserve this file for restart fixtures.`);
