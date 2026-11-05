-- Actual Client modules; Noita engine/native facts below are injected.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/'
local text=dofile(root..'a2_text.lua');local capture=dofile(root..'a2_capture.lua');local models=os.getenv('NT2_A2_MODEL_FILE')and raw(os.getenv('NT2_A2_MODEL_FILE'))or dofile(root..'a2_ui_model.lua');local locks=dofile(root..'a2_input_lock.lua')
local count=0;local function check(value,message)assert(value,message);count=count+1 end
check(text.length(string.rep('雪',99))==99,'99 Unicode code points')
check(text.length(string.rep('😀',99))==99,'396 UTF8 bytes')
for _,s in ipairs({'\0','\n',string.char(127),string.char(192,128),string.char(237,160,128),string.rep('😀',100)})do check(not text.length(s),'invalid UTF8/control/bound accepted')end
check(text.backspace('A雪😀')=='A雪','Unicode backspace');check(not text.message('   '),'blank message')
local now=0;local hp,max=4,8;local poly=false;local alive=true;local inventory=true;local sampo=true
local e={GameGetRealWorldTimeSinceStarted=function()return now end,EntityGetIsAlive=function(id)return id==99 and alive end,
 EntityHasTag=function(id,tag)return id==99 and poly and tag=='polymorphed_player' or id==102 and sampo and tag=='this_is_sampo'end,
 EntityGetFirstComponentIncludingDisabled=function(id,kind)if id==99 and kind=='DamageModelComponent'then return 2 end end,
 ComponentGetValue2=function(_,key)return key=='hp'and hp or key=='max_hp'and max end,
 EntityGetTransform=function()return 500,1200 end,BiomeMapGetName=function()return '$biome_coalmine'end,
 EntityGetAllChildren=function(id)if not inventory then error('unavailable')end;return id==99 and {101}or id==101 and {102}or {}end}
local facts=capture.sample(e,99);check(facts.hp==4 and facts.max_hp==8 and facts.sampo==true,'real Noita HP/inventory facts')
check(facts.biome=='$biome_coalmine'and facts.depth/10==120,'biome/depth units')
sampo=false;check(capture.sample(e,99).sampo==false,'known absent Sampo')
inventory=false;check(capture.sample(e,99).sampo==nil,'unavailable inventory stays unknown');inventory=true
poly=true;facts=capture.sample(e,99);check(facts.hp==nil and facts.max_hp==nil and facts.sampo==nil and facts.biome=='$biome_coalmine'and facts.depth==1200,'polymorph HP/inventory unknown but location independently observed');poly=false
alive=false;check(next(capture.sample(e,99))==nil,'absent player unknown');alive=true
hp=0;max=0;facts=capture.sample(e,99);check(facts.hp==0 and facts.max_hp==nil,'HP0 / invalid maximum');hp=4;max=8
local sends={};local publisher=capture.publisher(e,function(value)sends[#sends+1]={at=now,hp=value.hp};return true end)
for frame=0,299 do now=frame/60;publisher:tick(99,true)end
check(#sends>=4 and #sends<=6,'unchanged ~1Hz refresh');for i=2,#sends do check(sends[i].at-sends[i-1].at>=0.5,'bounded publication cadence')end
local before=#sends;publisher:tick(99,false);check(#sends==before,'protected capture does not publish')
local notices={};local model=models.new(text,function(s)notices[#notices+1]=s end)
check(not model.show_players and not model.composer and model.show_chat and model.show_ghosts,'panel defaults')
model:roster('run',{{actor='z',name='alice',ghost='one'},{actor='a',name='Alice',ghost='two'},{actor='b',name='Bob',ghost='three'}})
local rows=model:rows();check(rows[1].actor=='a'and rows[2].actor=='z','case-insensitive stable actor tie')
model:toggle_follow('a');model:roster('run',{{actor='a',name='Alice',ghost='replacement'}});check(model.follow.a,'same-world follow survives attachment replacement')
model:roster('run',{});check(not model.follow.a,'explicit removal clears follow')
model:roster('other',{{actor='a',name='Alice'}});check(not model.follow.a,'new-run follow initially off')
check(not models.radar(250,150,0,0,640,360),'radar exact threshold')
local x,y,d=models.radar(1000,0,0,0,640,360);check(x and y and d==100,'radar legacy distance')
for _,size in ipairs({{320,180},{640,360},{100,80}})do for _,target in ipairs({{1000,0},{-1000,0},{0,1000},{0,-1000},{1000,1000},{-1000,-1000}})do local x,y=models.radar(target[1],target[2],0,0,size[1],size[2]);check(x>=30 and x<=size[1]-30 and y>=15 and y<=size[2]-30,'radar fits viewport in every direction')end end
model.show_chat=false;local n=#notices;check(model:chat('r:1','A','hello')and #notices==n,'mute incoming chat')
model.show_chat=true;check(not model:chat('r:1','A','hello')and #notices==n,'muted chat does not replay');check(model:chat('r:2','A','hello')and #notices==n+1,'attributed chat')
model.composer=true;check(model:edit('雪 message'),'Unicode draft');local requests=0
local function send(message,px,py)requests=requests+1;check(message=='雪 message'and px==50 and py==90,'POI actual position/y-10');return 'original'end
check(model:submit(send,50,100,true),'submit');check(not model:submit(send,50,100,true)and requests==1,'no double submit')
model:outcome('original','uncertain');check(model.pending=='original'and model.draft=='雪 message','lost reply retains original/draft')
model:outcome('original','refused','Too close');check(not model.pending and model.draft=='雪 message','refusal retains draft')
model:submit(send,50,100,true);model:outcome('original','accepted');check(model.draft==''and not model.composer,'confirmed success closes/clears')
model:edit('cancel');model:cancel();check(model.draft=='','cancel clears')
model.composer=true;model:edit('busy draft');local copies=0
local function busy_send()copies=copies+1;return nil,'Optional message lane is busy; keep the draft and try again'end
check(not model:submit(busy_send,50,100,true)and not model.pending and model.draft=='busy draft','no-copy busy outcome keeps editable draft and no original')
model:edit('busy draft');check(model.feedback=='Optional message lane is busy; keep the draft and try again','unchanged GUI text callbacks preserve visible copy-busy feedback')
check(model:submit(function()copies=copies+1;return 'copied-original'end,50,100,true)and model.pending=='copied-original','later explicit user attempt may copy an original')
check(not model:submit(busy_send,50,100,true)and copies==2,'once copied, no further UI submit can replay while outcome is unknown');model:outcome('copied-original','uncertain');model:edit('changed');check(model.pending=='copied-original'and model.draft=='busy draft','uncertain original is never changed or retried as a new draft')
model:outcome('copied-original','refused','Definitive refusal');model:edit('busy draft');check(model.feedback=='Definitive refusal','unchanged GUI text also preserves definitive refusal feedback');model:edit('actually changed');check(model.feedback==''and model.draft=='actually changed','actual edits clear old feedback');model:cancel()
local components={[1]={enabled=true},[2]={hp=4}};local variables={};local serial=10
local engine={EntityGetIsAlive=function()return true end,EntityGetComponentIncludingDisabled=function()local out={};for id in pairs(variables)do out[#out+1]=id end;return out end,
 EntityGetFirstComponentIncludingDisabled=function(_,kind)return kind=='ControlsComponent'and 1 or kind=='DamageModelComponent'and 2 end,
 ComponentGetValue2=function(id,key)return components[id][key]end,ComponentSetValue2=function(id,key,value)components[id][key]=value end,
 EntityAddComponent2=function(_,_,values)serial=serial+1;components[serial]=values;variables[serial]=true;return serial end,
 EntityRemoveComponent=function(_,id)variables[id]=nil;components[id]=nil end}
local lock=locks.new(engine);check(lock:open(99,true)and not components[1].enabled,'composer owns input')
lock:close(true);check(components[1].enabled,'owned composer closes')
components[1].enabled=false;check(not lock:open(99,true)and not components[1].enabled,'foreign-disabled controls untouched');components[1].enabled=true
lock:open(99,true);lock:before_hold(99);check(components[1].enabled,'restore owned UI lease before production protection captures flags')
engine.EntityAddComponent2(99,'VariableStorageComponent',{name='nt2_g3_recovery'});components[1].enabled=false;lock:close(false);check(not components[1].enabled and not lock:open(99,true),'protection never released')
variables={};components[1].enabled=true;components[2].hp=4;lock:open(99,true);components[2].hp=-0.1;lock:close(true);check(not components[1].enabled,'negative HP death lock never released')
variables={};components[1].enabled=true;components[2].hp=0;check(lock:open(99,true),'living zero HP may compose');lock:close(true);check(components[1].enabled,'living zero HP owned composer restores')
local clicked=nil;local notices2={};local drawn={};local globals={};local loaded={};local seq=100
variables={};components[1].enabled=true;components[2].hp=4
local function press(label)if clicked==label then clicked=nil;return true end;return false end
engine.GameGetRealWorldTimeSinceStarted=function()return now end;engine.GuiCreate=function()return 1 end;engine.GuiStartFrame=function()drawn={}end;engine.GuiDestroy=function()end
engine.GuiButton=function(_,_,_,_,label)return press(label)end;engine.GuiText=function(_,_,_,value)drawn[#drawn+1]=value end;engine.GuiImage=function()end
engine.GuiImageButton=function(_,_,_,_,_,path)if path:find('/keyboard.png',1,true)then return press('World message')end;return false end
engine.GuiTextInput=function(_,_,_,_,value)return value end;engine.GamePrint=function(value)notices2[#notices2+1]=value end
engine.GlobalsGetValue=function(k,fallback)return globals[k]or fallback end;engine.GlobalsSetValue=function(k,v)globals[k]=v end
engine.EntityGetTransform=function()return 50,100 end;engine.EntityLoad=function(_,x,y)seq=seq+1;loaded[seq]={x=x,y=y};components[seq+1000]={};return seq end;engine.EntityKill=function(id)loaded[id]=nil;components[id+1000]=nil end
local get_component=engine.EntityGetFirstComponentIncludingDisabled;engine.EntityGetFirstComponentIncludingDisabled=function(id,kind)if loaded[id]and kind=='ItemComponent'then return id+1000 end;return get_component(id,kind)end
engine.EntityGetIsAlive=function(id)return id==99 or loaded[id]~=nil end;engine.a2_positions=function()return {ghost={x=1000,y=100}}end
engine.GameTextGetTranslatedOrNot=function(key)return key=='$biome_coalmine'and 'Mines'or key end
raw(repo..'/apps/desktop/tools/a2-layout-test-api.lua')(engine)
engine.GuiTooltip=function(_,title,body)drawn[#drawn+1]=title..'\n'..body end
local renderer=dofile(root..'a2_renderer.lua').new(engine,{id=function(id)return type(id)=='string'end})
renderer:roster('run','me',{{actor='me',name='Me',ghost='self',status=1},{actor='peer',name='Peer',ghost='ghost',status=1}});renderer.model.show_players=true
now=1;renderer:receive_info({actor='peer',revision=1,hp=0,max_hp=0,biome='_EMPTY_'},now);renderer:tick(true,99)
local output=table.concat(drawn,'|');check(output:find('Unknown location',1,true)and not output:find('nan',1,true),'unknown biome and invalid maximum safe')
check(output:find('Sampo unknown',1,true),'unavailable Sampo is explicit')
now=3600;renderer:tick(true,99);check(table.concat(drawn,'|'):find('HP unknown',1,true)and #renderer.model:rows()==2,'never-known HP remains unknown with retained roster')
local function hud_packet(revision)return {actor='peer',revision=revision,hp=1000,max_hp=2000,biome='$biome_coalmine'}end
check(renderer:receive_info(hud_packet(2)),'new revision refreshes HUD')
check(not renderer:receive_info(hud_packet(1)),'older revision cannot replace floor')
check(not renderer:receive_info(hud_packet(2)),'identical current revision is idempotent')
renderer.api.clear_hud();check(renderer:receive_info(hud_packet(2))and renderer.model.hud.peer,'same canonical revision restores discarded view')
local gap=dofile(root..'hud_poi_local.lua').new(renderer.api,dofile(root..'validation.lua'));check(gap:bind('hud-gap',1,{'peer'}),'current optional owner');check(renderer:receive_info(hud_packet(2)),'current canonical snapshot restores after owner bind')
gap:pause(true,true);check(renderer.model.hud.peer and not gap.playing,'optional copy gap retains state without authority')
for _,field in ipairs({'hp','max_hp','sampo','biome'})do local bad=hud_packet(2);bad[field]=field=='biome'and '$biome_snowcave'or field=='sampo'and true or 999;check(not renderer:receive_info(bad),'same-revision substituted '..field..' cannot replace state')end
renderer:roster('run','me',{{actor='me',name='Me',ghost='self',status=1},{actor='peer',name='Peer',ghost='continued-session',status=1}})
check(not renderer:receive_info(hud_packet(1))and renderer.model.hud.peer.revision==2,'same-world session replacement preserves revision floor and value')
renderer:roster('run','me',{{actor='me',name='Me',ghost='self',status=1}});renderer:roster('run','me',{{actor='me',name='Me',ghost='self',status=1},{actor='peer',name='Peer',status=1}})
check(renderer:receive_info(hud_packet(1)),'true roster removal retires floor')
clicked='World message';renderer:tick(true,99);check(renderer.model.composer and not components[1].enabled,'production composer takes owned controls')
renderer:close();check(components[1].enabled,'optional renderer close restores its owned input lease')
clicked='World message';renderer:tick(true,99);renderer.model.pending='immutable';renderer.model.draft='pending draft';clicked='Close pending';renderer:tick(true,99);check(components[1].enabled and not renderer.model.composer and renderer.model.pending=='immutable','pending original can close UI without abandoning operation')
renderer:roster('run','me',{{actor='me',name='Me',ghost='self',status=1}});renderer.api.poi('1','me','message',50,90,'Me');renderer:tick(true,99);check(next(loaded)~=nil,'canonical marker materialized by actual renderer')
renderer.model.pending='old';renderer.model.draft='old draft';renderer:roster('other','me',{});check(not renderer.model.pending and renderer.model.draft=='','different run clears pending UI')
renderer:close(true);check(next(loaded)==nil and globals.nt2_a2_run=='','terminal clears markers/follow scope')
local canonical=dofile(root..'hud_poi_local.lua').new(renderer.api,dofile(root..'validation.lua'));renderer:roster('canonical-run','me',{{actor='me',name='Me',status=1}});check(canonical:bind('scope',1,{'me'}),'own canonical marker scope')
check(canonical:begin_snapshot('scope',1,'1'),'first own snapshot begins');canonical:pause() -- Local optional interruption before its page is applied.
check(canonical:bind('scope',1,{'me'}),'same-world display resumes')
local marker={sequence='1',actor='me',name='Me',text=string.rep('😀',99),x=50,y=90}
check(canonical:begin_snapshot('scope',1,'1')and canonical:page('scope',1,'1','1',{marker}),'same unchanged high repairs missing canonical page');renderer:tick(true,99)
local own_entity=next(loaded);check(own_entity and components[own_entity+1000].item_name=='My message'and components[own_entity+1000].ui_description==' '..marker.text,'own caption/text comes from authoritative page')
local creations=seq;for _=1,3 do check(canonical:begin_snapshot('scope',1,'1')and canonical:page('scope',1,'1','1',{marker}),'repeat retained canonical snapshot');renderer:tick(true,99)end
check(seq==creations and next(loaded)==own_entity,'unchanged snapshots preserve exactly one existing entity')
renderer.api.clear_pois();check(next(loaded)==nil,'local view discarded independently of retained canonical state')
canonical:begin_snapshot('scope',1,'1');canonical:page('scope',1,'1','1',{marker});renderer:tick(true,99);check(next(loaded)~=nil,'same canonical snapshot repairs a separately discarded renderer view')
check(canonical:dismiss('1'),'local dismiss remembered');canonical:begin_snapshot('scope',1,'1');canonical:page('scope',1,'1','1',{marker});renderer:tick(true,99);check(next(loaded)==nil,'repeated canonical refresh never revives local dismissal');renderer:close(true)
renderer:roster('pickup-retry','me',{{actor='me',name='Me',status=1}});renderer.api.poi('1','me','retry local preference',50,90,'Me');renderer:tick(true,99)
local pickup=renderer.entities['1'];engine.EntityKill(pickup);local pickup_attempts=0;renderer:set_dismiss(function()pickup_attempts=pickup_attempts+1;return pickup_attempts>1 end)
renderer:tick(false,99);check(pickup_attempts==0 and renderer.pois['1'],'protected gameplay does not process local pickup')
renderer:tick(true,99);check(pickup_attempts==1 and renderer.entities['1']==pickup and renderer.pois['1'],'refused local dismissal retains dead-entity identity for retry')
renderer:tick(true,99);check(pickup_attempts==2 and not renderer.entities['1']and not renderer.pois['1'],'accepted local dismissal retires visual bookkeeping once');renderer:close(true)
local retained={};local segmented=dofile(root..'hud_poi_local.lua').new({time=function()return now end,clear_hud=function()end,remove_hud=function()end,clear_pois=function()retained={}end,remove_poi=function(id)retained[id]=nil end,poi=function(id,_,value)retained[id]=value end},dofile(root..'validation.lua'))
check(segmented:bind('same-scope',1,{'me'})and segmented:begin_snapshot('same-scope',1,'256'),'bounded multi-page prefix begins')
for first=1,256,16 do
 local rows={};for i=first,first+15 do rows[#rows+1]={sequence=tostring(i),actor='me',text='literal '..i,x=i*100,y=90}end
 check(segmented:page('same-scope',1,'256',tostring(first+15),rows),'all validated pages advance unchanged prefix')
 if first<241 then segmented:pause(true);check(segmented.pending and not segmented.playing and next(retained)==nil,'optional lease gap preserves pending prefix without displaying or granting play');check(segmented:bind('same-scope',1,{'me'}),'current authenticated frame may rebind same scope')end
end
local retained_count=0;for _ in pairs(retained)do retained_count=retained_count+1 end;check(retained_count==256 and not segmented.pending,'all256 commit only at final page after repeated optional gaps')
segmented:begin_snapshot('same-scope',1,'256');segmented:pause();check(not segmented.pending,'explicit pause/rescan still discards partial prefix')
segmented:bind('same-scope',1,{'me'});segmented:begin_snapshot('same-scope',1,'256');segmented:pause(true);check(not segmented:bind('same-scope',2,{'me'})and not segmented.pending,'generation mismatch discards pending prefix')
segmented:bind('new-scope',2,{'me'});check(next(retained)==nil and not segmented.pending,'different scope clears visible and partial prefix')
local live={};local alpha=nil;local ghost_serial=0;local time=0
local ghosts=dofile(root..'ghosts.lua').new({time=function()return time end,spawn=function()ghost_serial=ghost_serial+1;live[ghost_serial]=true;return ghost_serial end,
 alive=function(id)return id~=nil and live[id]==true end,remove=function(id)live[id]=nil end,position=function()end,opacity=function(_,value)alpha=value end},dofile(root..'validation.lua'))
check(ghosts:bind('scope',{'peer'}),'ghost binding');local function pose(n)return {actor='peer',sequence=n,x=100,y=100,vx=0,vy=0,rotation=0,facing=1}end
check(ghosts:receive('scope',pose(1),1),'fresh ghost');ghosts:set_visibility(true,0);check(next(live)==nil,'hide removes only visuals');time=0.1;check(ghosts:receive('scope',pose(2),2),'hidden still consumes current selected movement');ghosts:tick(2,true);check(next(live)==nil,'hidden peer never respawns')
ghosts:set_visibility(false,0);ghosts:tick(3,true);check(next(live)~=nil and alpha==0,'show restores eligible current peer with global opacity0');ghosts:set_visibility(false,1);ghosts:tick(4,true);check(alpha==1,'opacity1 restored')
ghosts:reconcile('scope',{});ghosts:tick(5,true);check(next(live)==nil,'departed peer cannot revive on show');ghosts:reconcile('scope',{'peer'});time=0.2;ghosts:receive('scope',pose(3),6);time=2.3;ghosts:tick(7,true);check(next(live)==nil,'stale movement removes ghost/radar source')
-- Exercise the production Lua adapter with injected copy/decode facts. Both a
-- DLL-copy and post-authority delay do not change retained state revisions.
local adapter_clock=10;local received=nil
local pointer={};pointer.__add=function(_,offset)return {offset=offset}end
local fake_ffi={new=function()return setmetatable({},pointer)end,copy=function()end,string=function(p,n)return string.rep(p.offset==40 and 'o'or 'c',n)end}
local fake_dll={nt2_g3_social_terminal=function()return 0 end,nt2_g3_social_publish=function()return 1 end,
 nt2_g3_social_lease=function(_,header)for i=0,63 do header[i]=0 end;header[24]=1;header[28]=75;return 64 end,
 nt2_g3_social_next=function()adapter_clock=adapter_clock+0.6;return 1 end}
local fake_renderer={api={},model={},close=function()end,tick=function()end,set_dismiss=function()end,set_submit=function()end,
 receive_info=function(_,packet,at)received={revision=packet.revision}end}
local fake_state={pause=function()end,close=function()end,tick=function()end}
local fake_dispatch={new=function(authority,apply)return {receive=function()local a=authority();adapter_clock=adapter_clock+0.4;return apply({{tag=11,revision=7}},a.now)end,close=function()end}end}
local adapter=dofile(root..'social_native.lua').new(fake_dll,{},fake_ffi,{GameGetRealWorldTimeSinceStarted=function()return adapter_clock end}, {},{new=function()return fake_renderer end},{new=function()return fake_state end},fake_dispatch)
adapter:tick(true,nil);check(received and received.revision==7,'copy and dispatch delays preserve canonical revision')
-- Real native adapter + decoder + canonical consumer; only DLL copy/lease and
-- engine facts are injected. Retransmission is expected before broker ACK.
local current_frame=nil;local lease=true;local visible={};local poi_calls=0;local dismiss_callback=nil;local dismiss_attempts=0;local copied=''
fake_ffi.copy=function(_,body)copied=body end
fake_dll.nt2_g3_social_publish=function()if copied:find('"kind":"dismiss"',1,true)then dismiss_attempts=dismiss_attempts+1;return 0 end;return 1 end
fake_ffi.string=function(p,n)if p.offset then return string.rep(p.offset==40 and 'o'or 'c',n)end;return current_frame end
fake_dll.nt2_g3_social_next=function()return current_frame and #current_frame or 0 end
local real_lease=fake_dll.nt2_g3_social_lease;fake_dll.nt2_g3_social_lease=function(...)if not lease then return 0 end;return real_lease(...)end
fake_renderer.api={time=function()return adapter_clock end,clear_hud=function()end,remove_hud=function()end,clear_pois=function()visible={}end,remove_poi=function(id)visible[id]=nil end,poi=function(id,_,value)visible[id]=value;poi_calls=poi_calls+1 end}
fake_renderer.roster=function()end
fake_renderer.set_dismiss=function(_,callback)dismiss_callback=callback end
local actual=dofile(root..'social_native.lua').new(fake_dll,{},fake_ffi,{GameGetRealWorldTimeSinceStarted=function()return adapter_clock end},dofile(root..'validation.lua'),{new=function()return fake_renderer end},dofile(root..'hud_poi_local.lua'),dofile(root..'social_dispatch.lua'))
local function le(n,count)local bytes={};for _=1,count do bytes[#bytes+1]=string.char(n%256);n=math.floor(n/256)end;return table.concat(bytes)end
local function literal(s)return le(#s,2)..s end
local function wire_frame(sequence,body,count)return 'NSF1'..string.char(1)..string.rep('\0',3)..string.rep('c',16)..le(1,4)..le(sequence,4)..le(#body,4)..le(count,2)..'\0\0'..body end
local function players(run)return string.char(10)..literal(run)..literal('me')..string.char(1)..literal('me')..literal('Me')..string.char(1)..literal('')end
local function page_body(first)local rows={string.char(4)..literal('256')..literal(tostring(first+15))..string.char(16)};for i=first,first+15 do rows[#rows+1]=literal(tostring(i))..literal('me')..literal('Me')..literal('literal '..i)..le(i*100000,4)..le(90000,4)end;return table.concat(rows)end
local begin_frame=wire_frame(1,players('native-run')..string.char(3)..literal('256'),2);current_frame=begin_frame;actual:tick(true,nil)
check(actual.state.pending and actual.state.pending.count==0,'actual adapter begins validated prefix')
local frame_sequence=1
for first=1,256,16 do
 frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,page_body(first),1);actual:tick(true,nil)
 local pending=actual.state.pending;actual:tick(true,nil);check(actual.state.pending==pending,'identical native frame replay cannot discard prefix or apply twice')
 if first<241 then
  check(pending and pending.count==first+15,'actual decoder accumulates each page exactly once')
  local saved=current_frame;current_frame=begin_frame;actual:tick(true,nil);check(actual.state.pending==pending and pending.count==first+15,'older native frame is ignored without restarting snapshot')
  lease=false;actual:tick(true,nil);check(actual.state.pending==pending and not actual.state.playing,'copy lease gap retains prefix but revokes consumer play gate');lease=true;current_frame=saved
 end
end
local visible_count=0;for _ in pairs(visible)do visible_count=visible_count+1 end
check(visible_count==256 and poi_calls==256 and not actual.state.pending,'actual adapter commits all256 once despite duplicate frames and lease gaps')
frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,string.char(3)..literal('256'),1);actual:tick(true,nil);check(actual.state.pending,'new snapshot starts a new prefix')
frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,page_body(1),1);actual:tick(true,nil);check(actual.state.pending and actual.state.pending.count==16,'new snapshot first page accepted')
current_frame=wire_frame(frame_sequence,page_body(17),1);actual:tick(true,nil);check(not actual.state.pending,'same frame sequence with different bytes is invalid and discards prefix')
frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,string.char(3)..literal('256'),1);actual:tick(true,nil)
frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,page_body(1),1);actual:tick(true,nil)
frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,page_body(1),1);actual:tick(true,nil);check(not actual.state.pending,'regressing page in a new frame still discards invalid prefix')
frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,players('native-run'),1);actual:tick(true,nil)
lease=false;actual:tick(true,nil);check(not actual.state.playing,'optional copy lease is absent during legitimate local pickup')
check(dismiss_callback('1')and actual.state.dismissed['1'],'local pickup records scoped preference during optional lease gap');lease=true;actual:tick(true,nil);check(dismiss_attempts>0,'optional native dismissal refusal leaves retry pending')
local attempts_before_new_run=dismiss_attempts
frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,players('different-run'),1);actual:tick(true,nil);check(next(visible)==nil and not actual.state.pending,'same transport envelope cannot retain prefix or markers across a different run')
check(dismiss_attempts==attempts_before_new_run,'old-run refused dismissal is not retried in the new run')
local reused_page=string.char(4)..literal('1')..literal('1')..string.char(1)..literal('1')..literal('me')..literal('Me')..literal('new run marker')..le(100000,4)..le(90000,4)
frame_sequence=frame_sequence+1;current_frame=wire_frame(frame_sequence,string.char(3)..literal('1')..reused_page,2);actual:tick(true,nil)
check(visible['1']=='new run marker'and dismiss_attempts==attempts_before_new_run,'reused sequence in new run stays visible and gets no old dismissal')
-- Full production native adapter/decoder/consumer/renderer. Optional DLL lease
-- copies and Noita facts are injected; no delivery is fabricated by the adapter.
local native_scope,native_owner,native_generation='c','o',1
fake_ffi.string=function(p,n)if p.offset then return string.rep(p.offset==40 and native_owner or native_scope,n)end;return current_frame end
fake_dll.nt2_g3_social_lease=function(_,header)if not lease then return 0 end;for i=0,63 do header[i]=0 end;header[24]=native_generation;header[28]=75;return 64 end
local retained=dofile(root..'social_native.lua').new(fake_dll,{},fake_ffi,engine,dofile(root..'validation.lua'),dofile(root..'a2_renderer.lua'),dofile(root..'hud_poi_local.lua'),dofile(root..'social_dispatch.lua'))
local native_sequence=0
local function info_players(run,ghost,empty)return string.char(10)..literal(run)..literal('me')..string.char(empty and 0 or 1)..(empty and ''or literal('me')..literal('Me')..string.char(1)..literal(ghost or ''))end
local function info_hud(sequence,ttl)return string.char(11)..literal('me')..le(sequence,4)..le(4000,4)..le(8000,4)..string.char(7)..literal('$biome_coalmine')end
local function info_frame(body,commands)
 native_sequence=native_sequence+1;current_frame='NSF1'..string.char(1)..string.rep('\0',3)..string.rep(native_scope,16)..le(native_generation,4)..le(native_sequence,4)..le(#body,4)..le(commands,2)..'\0\0'..body
 retained:tick(true,99)
end
now=40;lease=true;info_frame(info_players('retained-run')..info_hud(1,1500),2);retained.renderer.model.show_players=true;current_frame=nil;retained:tick(true,99)
local first_revision=retained.renderer.model.hud.me.revision
check(table.concat(drawn,'|'):find('HP 100.0 / 200.0',1,true),'actual adapter displays authenticated current facts')
for i=1,100 do
 now=40+i/100;lease=i%4==0;retained:tick(true,99)
 check(table.concat(drawn,'|'):find('HP 100.0 / 200.0',1,true)and retained.renderer.model.hud.me.revision==first_revision,'current HUD continuously visible across repeated optional copy gaps without changing canonical revision')
 if not lease then check(not retained.state.playing,'retained HUD never retains packet application authority')end
end
lease=true;now=41.05;info_frame(string.char(5),1)
check(retained.renderer.model.hud.me and retained.renderer.model.hud.me.revision==first_revision,'optional abort Pause retains already validated current facts until server lifecycle change')
check(not retained.state.playing and not retained.state.pending,'optional abort Pause revokes consumer gate and discards historical prefix')
lease=false;info_frame(info_hud(2,2000),1);check(retained.renderer.model.hud.me.revision==1,'invalid optional lease cannot accept a fresh packet')
current_frame=nil;now=3600;retained:tick(true,99);check(retained.renderer.model.hud.me.revision==1 and table.concat(drawn,'|'):find('HP 100.0 / 200.0',1,true),'long connected gap retains last state')
lease=true;info_frame(info_hud(2,1500),1);check(retained.renderer.model.hud.me.revision==2,'new current revision replaces old state')
retained:tick(false,99);check(retained.renderer.model.hud.me,'local pause retains server-owned facts without granting authority')
current_frame=nil;retained:tick(true,99);check(retained.renderer.model.hud.me,'optional return retains same canonical facts')
info_frame(info_hud(3,1500),1);native_owner='p';current_frame=nil;retained:tick(true,99);check(not retained.renderer.model.hud.me,'changed native owner clears cached facts before any new packet')
info_frame(info_players('retained-run')..info_hud(4,1500),2);native_generation=2;current_frame=nil;retained:tick(true,99);check(not retained.renderer.model.hud.me,'changed generation clears cached facts')
info_frame(info_players('retained-run')..info_hud(5,1500),2);native_scope='d';current_frame=nil;retained:tick(true,99);check(not retained.renderer.model.hud.me,'changed native context clears cached facts')
info_frame(info_players('retained-run')..info_hud(6,1500),2);info_frame(info_players('different-run'),1);check(not retained.renderer.model.hud.me,'changed run clears cached facts')
info_frame(info_players('different-run',string.rep('a',32))..info_hud(1,1500),2);info_frame(info_players('different-run',string.rep('b',32)),1);check(retained.renderer.model.hud.me and retained.renderer.model.hud.me.revision==1,'same-world sender session preserves retained revision')
info_frame(info_hud(1,1500),1);info_frame(info_players('different-run','',true),1);check(not retained.renderer.model.hud.me and not retained.renderer.peers.me,'roster removal clears cached facts')
-- Terminal control has a separate copy path and must work while protected,
-- without a playing lease, even with a committed view and a partial refresh.
lease=true;info_frame(info_players('terminal-run')..string.char(3)..literal('256'),2)
for first=1,256,16 do info_frame(page_body(first),1)end
current_frame=nil;for _=1,32 do retained:tick(true,99)end -- Existing eight-entity callback budget.
local marker_count=0;for _ in pairs(retained.renderer.entities)do marker_count=marker_count+1 end;check(marker_count==256,'actual renderer has256 committed marker entities before End')
info_frame(string.char(3)..literal('256'),1);for first=1,240,16 do info_frame(page_body(first),1)end
check(retained.state.pending and retained.state.pending.pages==15 and retained.state.pending.count==240,'End interrupts actual fifteen-page refresh without discarding committed view early')
native_sequence=native_sequence+1;local body=string.char(6);current_frame='NSF1'..string.char(1)..string.rep('\0',3)..string.rep(native_scope,16)..le(native_generation,4)..le(native_sequence,4)..le(#body,4)..le(1,2)..'\0\0'..body
local terminal_context=nil;local terminal_pending=true;local native_string=fake_ffi.string
fake_ffi.string=function(p,n)if p==terminal_context then return string.rep(native_scope,n)end;return native_string(p,n)end
fake_dll.nt2_g3_social_terminal=function(c)terminal_context=c;if terminal_pending then terminal_pending=false;return #current_frame end;return 0 end
lease=false;retained:tick(false,99)
check(retained.ended and not retained.state.pending and next(retained.state.markers)==nil,'control-only End clears pending and committed consumer state without play authority')
check(next(retained.renderer.entities)==nil and next(retained.renderer.pois)==nil and retained.renderer.model.run==nil,'control-only End removes all actual marker entities and run-scoped UI')
info_frame(string.char(3)..literal('256')..page_body(1),2);check(next(retained.renderer.entities)==nil and not retained.state.pending,'queued old-context page cannot recreate a terminal marker view')
-- Observe only bounded counters through the normal logger, never UI payloads.
local telemetry_logs={};engine.print=function(value)telemetry_logs[#telemetry_logs+1]=value end
fake_dll.nt2_g3_social_terminal=function()return 0 end;lease=true;current_frame=nil;now=100
local observed=dofile(root..'social_native.lua').new(fake_dll,{},fake_ffi,engine,dofile(root..'validation.lua'),dofile(root..'a2_renderer.lua'),dofile(root..'hud_poi_local.lua'),dofile(root..'social_dispatch.lua'))
native_sequence=0
local function observed_frame(sequence,ttl)
 local body=info_players('diagnostic-run')..info_hud(sequence,ttl);native_sequence=native_sequence+1;current_frame='NSF1'..string.char(1)..string.rep('\0',3)..string.rep(native_scope,16)..le(native_generation,4)..le(native_sequence,4)..le(#body,4)..le(2,2)..'\0\0'..body;observed:tick(true,99)
end
observed_frame(1,1000);check(observed.diagnostics.received==1 and observed.diagnostics.known==1 and observed.diagnostics.applied==1,'normal diagnostics count decoded known sample and actual renderer apply')
for frame=1,100 do now=100+frame/60;current_frame=nil;observed:tick(true,99)end
check(#telemetry_logs==1,'normal logger rate limited through100 ordinary ticks')
now=106;observed_frame(2,0);check(observed.diagnostics.received==2 and observed.diagnostics.expired==0 and observed.renderer.model.hud.me.revision==2,'retained sample has no expiry counter or lifetime')
check(#telemetry_logs==2 and #telemetry_logs[2]<1500 and not telemetry_logs[2]:find('diagnostic-run',1,true)and not telemetry_logs[2]:find('Me',1,true)and not telemetry_logs[2]:find('$biome',1,true),'normal diagnostic log exposes counters only')
lease=false;now=112;current_frame=nil;observed:tick(true,99);check(observed.diagnostics.no_copy_lease==1 and #telemetry_logs==3,'missing copy lease distinguished from expired received HUD')
lease=true;now=113;observed.renderer.model.show_chat=true
local function observed_chat(id)
 local body=string.char(12)..literal(id)..literal('private_name')..literal('private_text');native_sequence=native_sequence+1;current_frame='NSF1'..string.char(1)..string.rep('\0',3)..string.rep(native_scope,16)..le(native_generation,4)..le(native_sequence,4)..le(#body,4)..le(1,2)..'\0\0'..body;observed:tick(true,99);observed:post_update()
end
observed_chat('chat-key-1');observed_chat('chat-key-1');observed.renderer.model.show_chat=false;observed_chat('chat-key-2')
check(observed.diagnostics.chats_received==3 and observed.diagnostics.chats_admitted==2 and observed.diagnostics.chats_visible==1,'chat counters distinguish received, deduplicated and hidden future chat')
now=118;current_frame=nil;observed:tick(true,99);local diagnostic_text=table.concat(telemetry_logs,'\n')
check(not diagnostic_text:find('private_name',1,true)and not diagnostic_text:find('private_text',1,true)and not diagnostic_text:find('chat-key',1,true),'chat diagnostics never expose message or actor identity')
engine.print=nil
print('PASS A2 '..count..' Client assertions: UTF8, real capture/unknowns/cadence, list/follow/radar/chat, immutable draft and owned input; engine/native facts injected')
