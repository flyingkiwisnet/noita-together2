// Actual shipped desktop assets in an isolated headless browser. Synthetic API
// view only: no normal app, game, credentials, database or personal browser data.
import assert from 'node:assert/strict';
import {readFile,mkdir,rm,writeFile} from 'node:fs/promises';
import {existsSync,readFileSync} from 'node:fs';
import {createServer} from 'node:http';
import {resolve,join,sep} from 'node:path';
import {pathToFileURL} from 'node:url';
import vm from 'node:vm';
import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
const repo=resolve(import.meta.dirname,'../../..'),ui=join(repo,'apps/desktop/ui');
const {chromium}=await import(pathToFileURL(process.env.NT2_PLAYWRIGHT_PATH??(()=>{throw Error('Set NT2_PLAYWRIGHT_PATH to an installed Playwright module');})()));
const source=readFileSync(join(repo,'apps/desktop/tools/test-ux1-ui.mjs'),'utf8').replaceAll('\r\n','\n');
for(const [source,destination]of [['src/race-goal-ui.js','race-goal-ui.js'],['src/race-goal-editor.js','race-goal-editor.js'],['g3-race-goals/schema.json','race-goal-schema.json'],['g3-race-goals/catalogue.json','race-goal-catalogue.json']])assert.deepEqual(await readFile(join(repo,'packages/contracts',source)),await readFile(join(ui,destination)),'Canonical Race UI asset drift: '+destination);
const dataModule=await readFile(join(ui,'race-goal-data.js'),'utf8');assert.deepEqual(JSON.parse(dataModule.slice('export const catalogue = '.length).trim().replace(/;$/,'')),{schema:JSON.parse(await readFile(join(ui,'race-goal-schema.json'),'utf8')),content:JSON.parse(await readFile(join(ui,'race-goal-catalogue.json'),'utf8'))});
const {joined}=vm.runInNewContext(source.slice(source.indexOf('const base='),source.indexOf('\n{\n'))+';({joined})');
joined.ux1.compatibility.modes=['core','race'];joined.ux1.compatibility.capabilities.push('race.goals.v1');
const out=join(repo,'spikes/noita-entry/.build/beta-client-20261105/ui',randomUUID());await mkdir(out,{recursive:true});const profile=join(out,'browser-profile');await mkdir(profile);
const csp=JSON.parse(await readFile(join(repo,'apps/desktop/src-tauri/tauri.conf.json'),'utf8')).app.security.csp;
const server=createServer(async(req,res)=>{try{const path=resolve(ui,'.'+new URL(req.url,'http://localhost').pathname.replace(/\/$/,'/index.html'));if(!path.startsWith(ui+sep)){res.writeHead(403).end();return;}const body=await readFile(path);res.writeHead(200,{'content-security-policy':csp,'content-type':({'.js':'text/javascript','.json':'application/json','.css':'text/css','.html':'text/html'})[/\.[^.]+$/.exec(path)?.[0]]??'application/octet-stream'}).end(body);}catch{res.writeHead(404).end();}});await new Promise(r=>server.listen(0,'127.0.0.1',r));
let browser,child,page,exited=false;const errors=[],catalogueRequests=[];
try{
 const executablePath=['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(existsSync);assert(executablePath,'Installed headless browser required');
 child=spawn(executablePath,['--headless=new','--remote-debugging-port=0','--user-data-dir='+profile,'--no-first-run','--no-default-browser-check','--disable-extensions','about:blank'],{windowsHide:true,stdio:'ignore'});child.on('exit',()=>exited=true);
 const deadline=Date.now()+20000;let port;while(Date.now()<deadline&&!exited){try{port=Number((await readFile(join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);if(port)break;}catch{}await new Promise(r=>setTimeout(r,50));}assert(port,'Owned headless browser startup failed');
 browser=await chromium.connectOverCDP('http://127.0.0.1:'+port);page=await browser.newPage({viewport:{width:1280,height:900},hasTouch:true});page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(/race-goal-(schema|catalogue)\.json/.test(r.url()))catalogueRequests.push(r.url());});
 await page.addInitScript(view=>{window.testView=view;window.__NT2_CANDIDATE__=true;window.__TAURI__={core:{invoke:async(...args)=>window.betaInvoke?window.betaInvoke(...args):structuredClone(window.testView)}};},joined);
 await page.goto('http://127.0.0.1:'+server.address().port+'/');await page.waitForFunction(()=>window.NT2_RACE_UI&&document.getElementById('lobby-config').elements.mode);


 const first=structuredClone(joined);first.signed_in=false;first.room=null;first.beta={enabled:true,version:'0.1.0-beta.1',name:'Noita Together 2 - Beta'};
 await page.evaluate(v=>{window.testView=v;window.betaCalls=[];window.betaTest={};window.betaInvoke=async(command,args)=>{betaCalls.push({command,args});if(command==='g1_beta_discover')return {candidates:[]};if(command==='g1_browse_game')return 'C:\\Games\\Noita';if(command==='g1_beta_export')return 'C:\\Reports\\NT2-diagnostics-owned.json';if(command==='g1_beta_setup'){if(betaTest.error)throw betaTest.error;if(!args.token)return {target:args.path+'\\mods\\noita_together2',backupRoot:args.path+'\\nt2-mod-backups',version:'0.1.0-beta.1',token:'synthetic-reviewed-preview',replacesExisting:true,instructions:'Enable NT2 in Noita Mods and allow native API access.'};const view={...window.testView,game_path:args.path};window.testView=view;return {result:{installed:true,instructions:'Enable NT2 in Noita Mods and restart.'},view};}return structuredClone(window.testView);};render(v);},first);
 await page.locator('#beta-open').click();assert(await page.locator('#settings-dialog').isVisible());assert(await page.locator('#beta-setup').isVisible());assert.match(await page.locator('#beta-version').textContent(),/0\.1\.0-beta\.1/);
 await page.locator('#beta-discover').click();assert.match(await page.locator('#beta-discovery').textContent(),/Use Browse/);
 await page.locator('#browse-game').click();assert.match(await page.locator('#game-path').inputValue(),/Games/);assert(await page.locator('#beta-apply').isDisabled());
 await page.locator('#beta-preview').click();assert.match(await page.locator('#beta-preview-text').textContent(),/Update\/repair.*mods.*backups/);assert(!(await page.locator('#beta-apply').isDisabled()));assert.equal(await page.evaluate(()=>betaCalls.filter(c=>c.command==='g1_beta_setup'&&c.args.token).length),0,'preview never installs');
 await page.evaluate(()=>betaTest.error='Close Noita normally before installing/updating the mod, then retry.');await page.locator('#beta-apply').click();await page.locator('#confirm-yes').click();assert.match(await page.locator('#beta-install-status').textContent(),/Close Noita normally/);
 await page.evaluate(()=>betaTest.error='The extracted beta payload is incomplete or changed. Extract the matching complete ZIP again.');await page.locator('#beta-preview').click();assert.match(await page.locator('#beta-install-status').textContent(),/complete ZIP/);assert(await page.locator('#beta-apply').isDisabled());
 await page.evaluate(()=>betaTest.error=null);await page.locator('#beta-preview').click();await page.locator('#game-path').fill('C:\\Games\\Other');assert(await page.locator('#beta-apply').isDisabled(),'path change invalidates preview');await page.locator('#beta-preview').click();await page.locator('#beta-apply').click();await page.locator('#confirm-yes').click();assert.match(await page.locator('#beta-install-status').textContent(),/Installed and verified.*Enable NT2/);assert(await page.locator('#beta-first-run').isHidden());
 await page.locator('#beta-export').click();assert.match(await page.locator('#beta-export-status').textContent(),/Saved.*excludes credentials/);assert.equal(await page.evaluate(()=>betaCalls.some(c=>c.command==='g1_login')),false,'local setup before sign-in');
 await page.screenshot({path:join(out,'desktop.png'),fullPage:true});await page.setViewportSize({width:480,height:900});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:join(out,'narrow.png'),fullPage:true});assert.deepEqual(errors,[]);console.log(JSON.stringify({passed:true,actualDesktopAssets:true,exactCsp:true,syntheticNativeFacts:true,firstRunBeforeSignIn:true,browseFallback:true,explicitPreviewInstall:true,closedAndPayloadErrors:true,pathFence:true,alreadyConfigured:true,versionAndDiagnosticExport:true,narrow:true,actualNoita:false,output:out}));

}catch(error){
 if(page){await writeFile(join(out,'failure.html'),await page.content());await page.screenshot({path:join(out,'failure.png'),fullPage:true});await writeFile(join(out,'failure.json'),JSON.stringify({message:error.message,errors,state:await page.evaluate(()=>({raceModule:!!window.NT2_RACE_UI,mode:!!document.querySelector('[name=mode][form=lobby-config]'),error:document.getElementById('error')?.textContent}))},null,2));}
 console.error('Race UI evidence: '+out);throw error;
}finally{
 if(browser){try{const cdp=await browser.newBrowserCDPSession();await cdp.send('Browser.close');}catch{}await browser.close();}
 for(let i=0;i<200&&child&&!exited;i++)await new Promise(r=>setTimeout(r,50));
 server.closeAllConnections();await new Promise(r=>server.close(r));
 if(child&&!exited)throw Error('Owned browser did not close cooperatively; no termination attempted.');
 if(resolve(profile).startsWith(resolve(out)+sep))await rm(profile,{recursive:true,force:true});
}
