import {mkdir,copyFile} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url),dest=new URL('dist/apps/web/public/',root);await mkdir(dest,{recursive:true});
for(const file of ['index.html','app.js','client.js','style.css'])await copyFile(new URL('apps/web/public/'+file,root),new URL(file,dest));
console.log('Built same-origin website assets. No deployment performed.');
for(const [folder,files] of [['candidate',['download.html','help.html','credits.html','downloads.js','downloads.css','index.html','app.js','client.js','style.css','sidebar.js','chat-view.js','mods-view.js','readiness-view.js','statistics-view.js','race-view.js','users.html','users.js']],['g3',['index.html','page.js','client.js','statistics.js']]]){
 const target=new URL('dist/apps/web/'+folder+'/',root);await mkdir(target,{recursive:true});
 for(const file of files)await copyFile(new URL('apps/web/'+folder+'/'+file,root),new URL(file,target));
}

const hints=new URL('dist/packages/contracts/src/',root);await mkdir(hints,{recursive:true});await copyFile(new URL('packages/contracts/src/lobby-setting-hints.js',root),new URL('lobby-setting-hints.js',hints));

await copyFile(new URL('packages/contracts/src/race-goal-ui.js',root),new URL('race-goal-ui.js',hints));

await copyFile(new URL('packages/contracts/src/race-goal-editor.js',root),new URL('race-goal-editor.js',hints));

await copyFile(new URL('packages/contracts/src/release-metadata.js',root),new URL('release-metadata.js',hints));
const releases=new URL('dist/apps/web/releases/',root);await mkdir(releases,{recursive:true});await copyFile(new URL('apps/web/releases/latest.json',root),new URL('latest.json',releases));
