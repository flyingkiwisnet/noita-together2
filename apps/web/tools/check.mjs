import {readFile} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
const publicRoot=new URL('../public/',import.meta.url);
for(const name of ['app.js','client.js']){
 const source=await readFile(new URL(name,publicRoot),'utf8');
 if(/innerHTML|outerHTML|insertAdjacentHTML|document\.write|\beval\s*\(|localStorage|sessionStorage/.test(source))throw Error('Unsafe renderer/storage sink in '+name);
 const result=spawnSync(process.execPath,['--check',new URL(name,publicRoot).pathname.replace(/^\/([A-Z]:)/,'$1')],{stdio:'inherit'});if(result.status!==0)process.exit(result.status??1);
}
const html=await readFile(new URL('index.html',publicRoot),'utf8');if(/\son\w+=|<script(?![^>]*src=)/i.test(html))throw Error('Inline executable content');
console.log('Web module syntax, escaped rendering and in-memory credential boundary checked.');
for(const folder of ['candidate','g3']){
 for(const name of folder==='candidate'?['downloads.js','app.js','client.js','sidebar.js','chat-view.js','mods-view.js','readiness-view.js','statistics-view.js','users.js']:['page.js','client.js','statistics.js']){
  const file=new URL(`../${folder}/${name}`,import.meta.url),source=await readFile(file,'utf8');
  if(/innerHTML|outerHTML|insertAdjacentHTML|document\.write|\beval\s*\(|localStorage|sessionStorage/.test(source))throw Error('Unsafe renderer/storage sink in '+folder+'/'+name);
  const result=spawnSync(process.execPath,['--check',file.pathname.replace(/^\/([A-Z]:)/,'$1')],{stdio:'inherit'});if(result.status!==0)process.exit(result.status??1);
 }
 const html=await readFile(new URL(`../${folder}/index.html`,import.meta.url),'utf8');if(/\son\w+=|<script(?![^>]*src=)/i.test(html))throw Error('Inline executable content in '+folder);
}
