// Development boundary preparation. Canonical shared messages live in
// g3-connection/schema.json; --check verifies the embedded copies consumers need.
import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
const root=new URL('../',import.meta.url),json=async p=>JSON.parse(await readFile(new URL(p,root),'utf8'));
const canonical=await json('g3-connection/schema.json');
for(const name of ['g2-next','g3-rejoin']){
 const path=new URL(name+'/schema.json',root),s=await json(name+'/schema.json');
 for(const [k,v]of Object.entries(canonical.messages))if(k.startsWith('Connection'))s.messages[k]=v;
 s.messages.Response.fields.canonicalAdmission={id:90,type:'ConnectionAdmission'};
 s.messages.Response.fields.credentialValidity={id:91,type:'ConnectionCredential'};
 if(name==='g2-next')s.messages.BrowserLoginResponse.fields.credentialValidity={id:3,type:'ConnectionCredential'};
 const result=JSON.stringify(s,null,2)+'\n';
 if(process.argv.includes('--check')){if(await readFile(path,'utf8')!==result)throw Error('Connection embedding drift: '+name);}else await writeFile(path,result);
}
