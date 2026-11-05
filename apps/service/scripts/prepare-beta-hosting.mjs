import {mkdir,readFile,lstat,realpath,copyFile,writeFile} from 'node:fs/promises';
import {createReadStream} from 'node:fs';
import {createHash} from 'node:crypto';
import path,{resolve,basename,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {validateReleaseMetadata,releaseLinks} from '../../../packages/contracts/src/release-metadata.js';
const root=fileURLToPath(new URL('../../../',import.meta.url));
const digest=async file=>{const hash=createHash('sha256');for await(const part of createReadStream(file))hash.update(part);return hash.digest('hex');};
export const inside=(parent,child,paths=path)=>{const r=paths.relative(parent,child);return r!==''&&r!=='..'&&!r.startsWith('..'+paths.sep)&&!paths.isAbsolute(r)&&!r.includes(':');};
export async function prepareBetaHosting({manifest,artifact,out}){
 const m=validateReleaseMetadata(JSON.parse(await readFile(manifest,'utf8')));if(m.status!=='available')throw Error('An available staged release is required');
 const source=resolve(artifact),target=resolve(out),scratch=resolve(root,'spikes/noita-entry/.build');
 if(!inside(scratch,target))throw Error('Output must be a new directory inside spikes/noita-entry/.build');
 const sourceStat=await lstat(source);if(!sourceStat.isFile()||sourceStat.isSymbolicLink()||basename(source)!==m.artifact.file||sourceStat.size!==m.artifact.bytes||await digest(source)!==m.artifact.sha256)throw Error('Artifact name, type, size or checksum mismatch');
 // Resolve the parent before creating anything: reject junction/symlink escapes.
 const parent=await realpath(dirname(target));if(!inside(await realpath(scratch),parent)&&parent!==await realpath(scratch))throw Error('Output parent escapes staging');
 await mkdir(target); // No overwrite, including a previously failed staging tree.
 const links=releaseLinks(m),zip=resolve(target,'.'+links.artifact),versionDir=resolve(target,'releases',m.version);
 await mkdir(dirname(zip),{recursive:true});await mkdir(versionDir,{recursive:true});await copyFile(source,zip);
 if((await lstat(zip)).size!==m.artifact.bytes||await digest(zip)!==m.artifact.sha256)throw Error('Copied artifact changed; do not publish this staging directory');
 const json=JSON.stringify(m,null,2)+'\n';await writeFile(resolve(versionDir,'manifest.json'),json);await writeFile(resolve(versionDir,'SHA256SUMS.txt'),`${m.artifact.sha256}  ${m.artifact.file}\n`);await writeFile(resolve(target,'releases/latest.json'),json);
 return {out:target,version:m.version,bytes:m.artifact.bytes};
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 const args=process.argv.slice(2),options={};if(args.length!==6)throw Error('Usage: --manifest FILE --artifact ZIP --out NEW_STAGING_DIRECTORY');
 for(let i=0;i<args.length;i+=2){const key=args[i].slice(2);if(!args[i].startsWith('--')||!['manifest','artifact','out'].includes(key)||options[key])throw Error('Invalid arguments');options[key]=args[i+1];}
 console.log(JSON.stringify(await prepareBetaHosting(options)));
}
