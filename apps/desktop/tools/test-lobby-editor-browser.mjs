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
const out=join(repo,'spikes/noita-entry/.build/lobby-editor-client-20261105/ui',randomUUID());await mkdir(out,{recursive:true});const profile=join(out,'browser-profile');await mkdir(profile);
const csp=JSON.parse(await readFile(join(repo,'apps/desktop/src-tauri/tauri.conf.json'),'utf8')).app.security.csp;
const server=createServer(async(req,res)=>{try{const path=resolve(ui,'.'+new URL(req.url,'http://localhost').pathname.replace(/\/$/,'/index.html'));if(!path.startsWith(ui+sep)){res.writeHead(403).end();return;}const body=await readFile(path);res.writeHead(200,{'content-security-policy':csp,'content-type':({'.js':'text/javascript','.json':'application/json','.css':'text/css','.html':'text/html'})[/\.[^.]+$/.exec(path)?.[0]]??'application/octet-stream'}).end(body);}catch{res.writeHead(404).end();}});await new Promise(r=>server.listen(0,'127.0.0.1',r));
let browser,child,page,exited=false;const errors=[],catalogueRequests=[];
try{
 const executablePath=['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(existsSync);assert(executablePath,'Installed headless browser required');
 child=spawn(executablePath,['--headless=new','--remote-debugging-port=0','--user-data-dir='+profile,'--no-first-run','--no-default-browser-check','--disable-extensions','about:blank'],{windowsHide:true,stdio:'ignore'});child.on('exit',()=>exited=true);
 const deadline=Date.now()+20000;let port;while(Date.now()<deadline&&!exited){try{port=Number((await readFile(join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);if(port)break;}catch{}await new Promise(r=>setTimeout(r,50));}assert(port,'Owned headless browser startup failed');
 browser=await chromium.connectOverCDP('http://127.0.0.1:'+port);page=await browser.newPage({viewport:{width:1280,height:900},hasTouch:true});page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(/race-goal-(schema|catalogue)\.json/.test(r.url()))catalogueRequests.push(r.url());});
 await page.addInitScript(view=>{window.testView=view;window.__NT2_CANDIDATE__=true;window.__TAURI__={core:{invoke:async()=>structuredClone(window.testView)}};},joined);
 await page.goto('http://127.0.0.1:'+server.address().port+'/');await page.waitForFunction(()=>window.NT2_RACE_UI&&document.getElementById('lobby-config').elements.mode);

 await page.evaluate(()=>uxNavigate('lobbies'));await page.locator('#create-open').click();
 const create=page.locator('#create-form'),nameHelp=create.locator('.setting-field[data-setting-key=name] .setting-help');
 assert.equal(await page.evaluate(()=>document.activeElement.classList.contains('setting-help')),false,'autofocus must target an editable field');
 assert.equal(await create.locator('.setting-hint:visible').count(),0,'no unsolicited tooltip');
 await nameHelp.focus();assert(await nameHelp.locator('+ .setting-hint').isVisible());await create.locator('[name=capacity]').focus();assert(!(await nameHelp.locator('+ .setting-hint').isVisible()));
 await nameHelp.hover();assert(await nameHelp.locator('+ .setting-hint').isVisible());await page.locator('#create-title').hover();
 await nameHelp.tap();assert.equal(await nameHelp.getAttribute('aria-expanded'),'true');await create.locator('[name=capacity]').focus();
 await create.locator('[name=mode]').selectOption('race');
 let draft=await create.locator('[name=raceDraft]').inputValue();assert.deepEqual(JSON.parse(draft).goals,[{instanceId:'classic',id:'classic_ending'}]);
 await create.locator('.race-goal-type').selectOption('reach_max_hp');await create.locator('.race-goal-field').fill('777');
 await create.locator('[name=mode]').selectOption('core');await create.locator('[name=mode]').selectOption('race');assert.equal(JSON.parse(await create.locator('[name=raceDraft]').inputValue()).goals[0].target,777);
 await page.locator('#create-close').click();await page.locator('#create-open').click();assert.equal(JSON.parse(await create.locator('[name=raceDraft]').inputValue()).goals[0].target,777,'reopen preserves edited draft');await page.locator('#create-close').click();
 const race=structuredClone(joined);race.room.roomId='ended-race';race.room.state='ended';race.room.members=[];race.room.config={...race.room.config,mode:'race',race:{registryRevision:'race-goals-1',durationMs:0,goals:[{instanceId:'old-hp',id:'reach_max_hp',target:500}]}};
 await page.evaluate(v=>{window.testView=v;render(v);},race);await page.locator('[data-lobby-tab="settings"]').click();
 const editor=page.locator('[data-race-editor=lobby-config]');assert.equal(JSON.parse(await editor.locator('[name=raceDraft]').inputValue()).goals[0].instanceId,'old-hp','existing lobby default never overwritten');
 await editor.locator('.race-goal-field').fill('650');await page.evaluate(v=>{window.testView=v;render(v);},race);assert.equal(JSON.parse(await editor.locator('[name=raceDraft]').inputValue()).goals[0].target,650,'refresh preserves dirty draft');
 const row=editor.locator('.race-goal-row').first();const rects=await row.evaluate(n=>[...n.children].slice(0,3).map(c=>({x:c.getBoundingClientRect().x,y:c.getBoundingClientRect().y,h:c.getBoundingClientRect().height})));assert(Math.abs(rects[0].y-rects[1].y)<20&&Math.abs(rects[0].y-rects[2].y)<20,'type/parameters/remove share wide row');
 assert.match(await row.locator('.race-goal-remove').getAttribute('aria-label'),/^Remove .+ goal$/);
 await page.screenshot({path:join(out,'wide.png'),fullPage:true});
 await editor.locator('.race-goal-add').click();assert.equal(await editor.locator('.race-goal-row').count(),2);await editor.locator('.race-goal-remove').last().click();assert.equal(await editor.locator('.race-goal-row').count(),1);
 await row.locator('.race-goal-type').selectOption('defeat_named_bosses');assert(await row.locator('select[multiple]').isVisible());
 await page.setViewportSize({width:480,height:900});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:join(out,'narrow-bosses.png'),fullPage:true});
 await row.locator('.race-goal-type').selectOption('reach_max_hp');await row.locator('.race-goal-field').fill('0');assert(await row.locator('.race-goal-field').evaluate(n=>!n.checkValidity()));
 assert.deepEqual(errors,[]);console.log(JSON.stringify({passed:true,actualDesktopAssets:true,exactCsp:true,tooltipIntentional:true,classicDefault:true,existingAndDirtyDraftsPreserved:true,wideInlineAndNarrowWrap:true,addRemoveValidation:true,syntheticApi:true,actualNoita:false,output:out}));

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
