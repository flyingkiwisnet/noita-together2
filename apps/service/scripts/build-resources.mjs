import {mkdir,copyFile,readdir} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url);
const outputRoot=process.env.NT2_BUILD_OUTPUT??'dist';
if(!/^[a-zA-Z0-9_-]+(?:\/[a-zA-Z0-9_-]+)*$/.test(outputRoot))throw Error('Build output must be a workspace-relative directory');
for(const folder of ['apps/web/releases','packages/contracts/releases','packages/contracts/g1','packages/contracts/g2','packages/contracts/g2-next','packages/contracts/g3-auth1','packages/contracts/g3','packages/contracts/g3-c1','packages/contracts/g3-c2','packages/contracts/g3-items','packages/contracts/g3-live','packages/contracts/g3-rejoin','packages/contracts/g3-connection','packages/contracts/g3-waiting','packages/contracts/g3-lifecycle','packages/contracts/src','packages/contracts/g3-social','packages/contracts/g3-history','packages/contracts/g3-stats','packages/contracts/g3-race','packages/contracts/g3-race-goals','packages/contracts/g3-hourglass','packages/contracts/g3-hourglass-v2','packages/contracts/g3-nemesis','db/migrations','apps/web/public','apps/web/candidate','apps/web/g3','deploy/profiles']){
 await mkdir(new URL(outputRoot+'/'+folder+'/',root),{recursive:true});
 for(const name of await readdir(new URL(folder+'/',root)))if(/\.(json|sql|proto|rs|html|js|css|xml)$/.test(name))await copyFile(new URL(folder+'/'+name,root),new URL(outputRoot+'/'+folder+'/'+name,root));
}
console.log('Copied versioned schema, codec descriptors and migration resources to '+outputRoot+'.');

