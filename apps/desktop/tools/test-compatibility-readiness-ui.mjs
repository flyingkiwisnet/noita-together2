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
const out=join(repo,'spikes/noita-entry/.build/compatibility-readiness-client-20261105/ui',randomUUID());await mkdir(out,{recursive:true});const profile=join(out,'browser-profile');await mkdir(profile);
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

 const v=structuredClone(joined),m=v.room.members[0];m.attached=true;m.loaded=true;m.readiness='ready';
 v.ux1.compatibility.modVersion='required-mod';v.ux1.compatibility.bridgeVersion='required-native';
 const report={...v.inspections[0],attachmentId:'attachment',worldToken:'world',gameInstanceId:'instance',capturedAt:new Date().toISOString(),actualSeed:String(v.room.config.seed),seedStatus:'known',compatibility:'compatible',modVersion:'required-mod',bridgeVersion:'required-native',compatibilityProvenance:'game-thread-client-observation',presenceUntil:new Date(Date.now()+60000).toISOString(),validUntil:new Date(Date.now()+60000).toISOString()};v.inspections=[report];
 const render=async value=>{await page.evaluate(v=>{window.testView=v;render(v);},value);return page.locator('#players tr').first();};
 const states=async row=>row.locator('.readiness-symbol').evaluateAll(b=>b.map(n=>({state:n.dataset.state,label:n.getAttribute('aria-label')})));
 let row=await render(v);assert.equal(await row.locator('td').count(),6);assert.deepEqual((await states(row)).map(c=>c.state),['ok','ok','ok','ok','warn']);
 assert.equal(await row.locator('.player-name').textContent(),m.principal.displayName);assert.equal(await row.locator('input[type=checkbox]').count(),1);
 const cells=row.locator('.readiness-symbol');await cells.nth(2).focus();assert(await cells.nth(2).evaluate(n=>n.getAttribute('aria-describedby')));assert(await row.locator('.readiness-tip').nth(2).isVisible());await cells.nth(2).press('Enter');assert.equal(await cells.nth(2).getAttribute('aria-expanded'),'true');
 await page.locator('#lobby-title').click();await cells.nth(0).hover();assert(await row.locator('.readiness-tip').first().isVisible());
 const noGame=structuredClone(v);noGame.inspections=[];noGame.room.members[0].attached=false;noGame.room.members[0].readiness='waiting_noita';row=await render(noGame);assert.equal((await states(row))[2].state,'unknown');assert.equal((await states(row))[1].state,'unknown');
 const loading=structuredClone(v);loading.inspections[0].loaded=false;loading.room.members[0].readiness='loading_world';loading.room.members[0].loaded=false;row=await render(loading);assert.match((await states(row))[1].label,/loading/i);
 const longRunning=structuredClone(v);longRunning.room.state='running';longRunning.room.members[0].readiness='playing';longRunning.inspections[0].validUntil='2000-01-01T00:00:00Z';row=await render(longRunning);assert.equal((await states(row))[2].state,'ok','playing world uses current presence, not expired original review deadline');
 const wrongSeed=structuredClone(v);wrongSeed.inspections[0].actualSeed='99';wrongSeed.inspections[0].compatibility='seed_mismatch';row=await render(wrongSeed);assert.equal((await states(row))[2].state,'ok');assert.equal((await states(row))[3].state,'error');
 const individual=structuredClone(noGame);individual.room.config.seedPolicy='individual-random';row=await render(individual);assert.match((await states(row))[3].label,/N\/A/);
 const paused=structuredClone(noGame);paused.room.members[0].attached=true;paused.room.members[0].readiness='paused';row=await render(paused);assert.equal((await states(row))[0].state,'ok');assert.match((await states(row))[1].label,/Paused/);
 const disconnected=structuredClone(paused);disconnected.room.members[0].present=false;disconnected.room.members[0].readiness='reconnecting';row=await render(disconnected);assert.equal((await states(row))[0].state,'warn');assert.equal((await states(row))[4].label,'Reconnecting');
 const excluded=structuredClone(v);excluded.room.members[0].included=false;row=await render(excluded);assert.equal((await states(row))[4].label,'Excluded');
 const ended=structuredClone(v);ended.room.members[0].readiness='completed';row=await render(ended);assert.equal((await states(row))[4].label,'Completed');
 const mismatch=structuredClone(wrongSeed);mismatch.inspections[0].modVersion='other-mod';row=await render(mismatch);assert.equal((await states(row))[2].state,'error');assert.match(await row.locator('.readiness-tip').nth(2).textContent(),/other-mod.*required-mod/);
 row=await render(noGame);assert.equal((await states(row))[2].state,'error','last confirmed mismatch survives generic refresh');row=await render(v);assert.equal((await states(row))[2].state,'ok','matching current validation clears mismatch');
 await render(mismatch);const different=structuredClone(noGame);different.room.roomId='new-room';row=await render(different);assert.equal((await states(row))[2].state,'unknown');
 const issue={origin:v.server_origin,component:'app_service',code:'Service/contract mismatch',message:'App and service protocols do not match.',action:'Use the matching release and reconnect.',details:{requiredProtocol:'expected',advertisedProtocol:'other'}};v.compatibility_issue=issue;await render(v);assert(await page.locator('#compatibility-alert').isVisible());v.handoff='Checking ordinary refresh';await render(v);assert(await page.locator('#compatibility-alert').isVisible());
 const other=structuredClone(v);other.server_origin='https://other.invalid';await render(other);assert(!(await page.locator('#compatibility-alert').isVisible()));v.compatibility_issue=null;row=await render(v);assert(!(await page.locator('#compatibility-alert').isVisible()));
 await page.evaluate(()=>uxNavigate('lobby'));await row.locator('.readiness-symbol').nth(2).evaluate(n=>{n.dataset.open='false';n.setAttribute('aria-expanded','false');});
 await page.screenshot({path:join(out,'desktop.png'),fullPage:true});await page.setViewportSize({width:480,height:900});await row.locator('.readiness-symbol').nth(2).tap();assert.equal(await row.locator('.readiness-symbol').nth(2).getAttribute('aria-expanded'),'true');assert(await row.locator('.readiness-tip').nth(2).isVisible());assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:join(out,'narrow.png'),fullPage:true});assert.deepEqual(errors,[]);
 console.log(JSON.stringify({passed:true,actualDesktopAssets:true,exactCsp:true,readinessCases:10,unknownDistinct:true,pausedDistinct:true,canonicalSeed:true,stickyMismatch:true,matchingAndContextClear:true,keyboardHoverTouch:true,includePreserved:true,idsInDetailsOnly:true,narrowLayout:true,syntheticApi:true,actualNoita:false,output:out}));
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
