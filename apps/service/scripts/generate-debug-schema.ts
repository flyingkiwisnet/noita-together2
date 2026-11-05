import {readFile,writeFile} from 'node:fs/promises';
import {serverDebugSchema} from '../src/g3/debug-diagnostics.js';
const path=new URL('../../../docs/g3/server-debug.schema.json',import.meta.url),bytes=JSON.stringify(serverDebugSchema(),null,2)+'\n';
if(process.argv.includes('--check')){if(await readFile(path,'utf8')!==bytes)throw Error('Server debug schema drift');}else await writeFile(path,bytes);
console.log('Server debug privacy schema '+(process.argv.includes('--check')?'verified':'generated'));
