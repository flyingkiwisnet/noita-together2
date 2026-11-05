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
const out=join(repo,'spikes/noita-entry/.build/race-first-release-client-20261105/ui',randomUUID());await mkdir(out,{recursive:true});const profile=join(out,'browser-profile');await mkdir(profile);
const csp=JSON.parse(await readFile(join(repo,'apps/desktop/src-tauri/tauri.conf.json'),'utf8')).app.security.csp;
const server=createServer(async(req,res)=>{try{const path=resolve(ui,'.'+new URL(req.url,'http://localhost').pathname.replace(/\/$/,'/index.html'));if(!path.startsWith(ui+sep)){res.writeHead(403).end();return;}const body=await readFile(path);res.writeHead(200,{'content-security-policy':csp,'content-type':({'.js':'text/javascript','.json':'application/json','.css':'text/css','.html':'text/html'})[/\.[^.]+$/.exec(path)?.[0]]??'application/octet-stream'}).end(body);}catch{res.writeHead(404).end();}});await new Promise(r=>server.listen(0,'127.0.0.1',r));
let browser,child,page,exited=false;const errors=[],catalogueRequests=[];
try{
 const executablePath=['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(existsSync);assert(executablePath,'Installed headless browser required');
 child=spawn(executablePath,['--headless=new','--remote-debugging-port=0','--user-data-dir='+profile,'--no-first-run','--no-default-browser-check','--disable-extensions','about:blank'],{windowsHide:true,stdio:'ignore'});child.on('exit',()=>exited=true);
 const deadline=Date.now()+20000;let port;while(Date.now()<deadline&&!exited){try{port=Number((await readFile(join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);if(port)break;}catch{}await new Promise(r=>setTimeout(r,50));}assert(port,'Owned headless browser startup failed');
 browser=await chromium.connectOverCDP('http://127.0.0.1:'+port);page=await browser.newPage({viewport:{width:1280,height:900}});page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(/race-goal-(schema|catalogue)\.json/.test(r.url()))catalogueRequests.push(r.url());});
 await page.addInitScript(view=>{window.testView=view;window.__NT2_CANDIDATE__=true;window.__TAURI__={core:{invoke:async()=>structuredClone(window.testView)}};},joined);
 await page.goto('http://127.0.0.1:'+server.address().port+'/');await page.waitForFunction(()=>window.NT2_RACE_UI&&document.getElementById('lobby-config').elements.mode);
 await page.locator('[data-lobby-tab="settings"]').click();
 assert(await page.locator('[name=mode][form=lobby-config]').evaluate(n=>n.tagName==='INPUT'&&n.readOnly));
 assert.equal(await page.evaluate(()=>{const form=document.getElementById('lobby-config');form.elements.mode.value='race';return uxConfig(form,current,current.room.config).mode;}),'core');
 await page.locator('#reload-settings').click();await page.waitForFunction(()=>document.getElementById('lobby-config').elements.mode.value==='core');
 await page.evaluate(()=>uxNavigate('lobbies'));await page.locator('#create-open').click();await page.locator('[name=mode][form=create-form]').selectOption('race');
 const created=await page.evaluate(()=>uxConfig(document.getElementById('create-form'),current));assert.equal(created.mode,'race');assert(created.race.goals.length);
 await page.locator('#create-close').click();
 const raceJoined=structuredClone(joined);raceJoined.room.roomId='race-room-b';raceJoined.room.config={...raceJoined.room.config,...created};
 await page.evaluate(v=>{window.testView=v;render(v);},raceJoined);await page.locator('[data-lobby-tab="settings"]').click();
 assert(await page.locator('[name=mode][form=lobby-config]').evaluate(n=>n.value==='race'&&n.readOnly));
 const form=page.locator('#lobby-config'),editor=page.locator('[data-race-editor=lobby-config]');assert(await editor.isVisible());
 await editor.locator('input[type=number]').last().fill('1.5');await editor.getByRole('button',{name:'Add goal',exact:true}).click();
 const rows=editor.locator(':scope > div > div');await rows.first().locator('select').first().selectOption('reach_max_hp');await rows.nth(1).locator('select').first().selectOption('defeat_named_bosses');await rows.nth(1).locator('select[multiple]').selectOption(['dragon','high_alchemist']);
 let config=await page.evaluate(()=>uxConfig(document.getElementById('lobby-config'),current,current.room.config));
 assert.equal(config.mode,'race');assert.equal(config.race.durationMs,90000);assert.deepEqual(config.race.goals[1].bossIds,['high_alchemist','dragon']);assert(config.requiredCapabilities.includes('race.goals.v1'));
 assert.equal(await form.getAttribute('data-dirty'),'true');
 await rows.nth(1).locator('select').first().selectOption('classic_ending');await editor.getByRole('button',{name:'Add goal',exact:true}).click();await rows.nth(2).locator('select').first().selectOption('collect_orbs');await rows.nth(2).locator('input').fill('33');
 const invalid=await page.evaluate(()=>{try{uxConfig(document.getElementById('lobby-config'),current,current.room.config);return false;}catch{return true;}});assert(invalid,'Classic+33 must fail the shared canonical helper');
 await rows.nth(2).locator('input').fill('3');config=await page.evaluate(()=>uxConfig(document.getElementById('lobby-config'),current,current.room.config));
 const view=structuredClone(raceJoined);view.room.config=config;view.g3={race:{error:'',snapshot:{runKey:'race-one',revision:'7',selfRow:'row-a',config:config.race,status:'open',collectionState:'available',elapsedMs:'12345',entrants:[{rowKey:'row-a',displayName:'Same Name',rank:1,status:'racing',completedGoals:1,finalCountAtMs:'12000',progress:config.race.goals.map((g,i)=>({instanceId:g.instanceId,value:i===0?'500':'0',target:String(g.target??1),complete:i===0,completedAtMs:i===0?'12000':'',credits:[]}))},{rowKey:'row-b',displayName:'Same Name',rank:2,status:'dead',completedGoals:0,finalCountAtMs:'',progress:[]}]}}};
 view.g3.race.snapshot.entrants.reverse();
 await page.evaluate(v=>{window.testView=v;render(v);},view);const standings=page.locator('#race-standings');assert.equal(await standings.locator('details').count(),2);assert.deepEqual(await standings.locator('details').evaluateAll(rows=>rows.map(r=>r.dataset.row)),['row-a','row-b'],'Canonical rank order overrides row query order');await standings.locator('details').first().evaluate(n=>n.open=true);
 view.g3.race.snapshot.elapsedMs='13345';await page.evaluate(v=>{window.testView=v;render(v);},view);assert(await standings.locator('details').first().evaluate(n=>n.open),'Polling preserves expanded progress');
 view.g3.race.snapshot.revision='8';for(const row of view.g3.race.snapshot.entrants)row.rank=1;await page.evaluate(v=>{window.testView=v;render(v);},view);assert.deepEqual(await standings.locator('details').evaluateAll(rows=>rows.map(r=>r.dataset.row)),['row-b','row-a'],'Canonical ties retain stable query order');assert.equal(await standings.locator('summary').filter({hasText:'#1 '}).count(),2);
 await page.screenshot({path:join(out,'settings.png'),fullPage:true});await page.locator('[data-lobby-tab=race]').click();await page.screenshot({path:join(out,'desktop.png'),fullPage:true});await page.setViewportSize({width:640,height:900});await page.screenshot({path:join(out,'compact.png'),fullPage:true});assert.deepEqual(errors,[]);
 assert.deepEqual(catalogueRequests,[],'Desktop boot must not fetch catalogue JSON under IPC-only connect CSP');
 await page.route('**/ux1-race.js',route=>route.fulfill({status:503,contentType:'text/javascript',body:''}));await page.reload();await page.waitForFunction(()=>window.NT2_RACE_LOAD?.status==='error'&&document.getElementById('lobby-config').elements.mode);
 const failure=await page.evaluate(()=>{const form=document.getElementById('lobby-config');try{uxConfig(form,{...current,room:{...current.room,config:{...current.room.config,mode:'race'}}},current.room.config);return '';}catch(e){return e.message;}});assert.match(failure,/could not load.*Restart.*Capture Logs/);assert.match(await page.locator('#error').textContent(),/could not load/);
 console.log(JSON.stringify({passed:true,canonicalRankOrder:true,stableCanonicalTies:true,exactPackagedCsp:true,noCatalogueFetch:true,actionableModuleFailure:true,creationOnlyType:true,existingTypeReadOnly:true,canonicalTypePreserved:true,canonicalManualEditor:true,namedBossSelection:true,invalidCombination:true,commonDraftDirty:true,duplicateNamesIndependent:true,pollingPreservesDetails:true,actualDesktopAssets:true,syntheticApi:true,actualNoita:false,output:out}));
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
