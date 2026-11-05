-- Production renderer, injected Noita APIs. No actual game/display evidence.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local now=1;local images,texts={},{};local z=0
local e={GameGetRealWorldTimeSinceStarted=function()return now end,GuiCreate=function()return 1 end,GuiDestroy=function()end,
 GuiStartFrame=function()images={};texts={};z=0 end,GuiZSetForNextWidget=function(_,value)z=value end,
 GuiImage=function(_,_,_,_,path,_,scale)images[#images+1]={path=path,scale=scale,z=z};z=0 end,
 GuiText=function(_,_,_,value)texts[#texts+1]=value end,GuiButton=function()return false end,GuiImageButton=function()return false end,
 EntityGetIsAlive=function(id)return id==99 end,EntityGetTransform=function()return 10,100 end,
 GameTextGetTranslatedOrNot=function(key)return key=='$biome_coalmine'and 'Mines'or key end}
raw(repo..'/apps/desktop/tools/a2-layout-test-api.lua')(e)
e.GuiTooltip=function(_,title,body)texts[#texts+1]=title..'\n'..body end
local r=dofile(root..'a2_renderer.lua').new(e,dofile(root..'validation.lua'))
r:roster('test-run','peer',{{actor='peer',name='Peer',status=1,ghost=''}});r.model.show_players=true
if os.getenv('NT2_A2_BOUNDARY_CASE')=='chat' then
 assert(type(r.post_update)=='function','notice delivery must have an actual post-update boundary')
 assert(r.model:chat('1','A','hello'));r:post_update();assert(r.notice_diagnostics.unavailable==1 and r.notice_diagnostics.pending==1,'missing printer must retain one bounded pending delivery')
 e.GamePrint=function()error('injected printer failure')end;r:post_update();assert(r.notice_diagnostics.pending==0 and r.notice_diagnostics.dropped==1,'printer failure cannot retry indefinitely')
 local printed={};local phase='pre';e.GamePrint=function(value)assert(phase=='post','GamePrint called before post-update');printed[#printed+1]=value end
 assert(r.model:chat('2','A','visible'));r:tick(true,99);assert(#printed==0,'chat must not print during pre-update tick');phase='post';r:post_update();assert(#printed==1 and printed[1]==' A: visible');assert(not r.model:chat('2','A','visible'));r:post_update();assert(#printed==1,'duplicate must not print again')
 r.model.show_chat=false;assert(r.model:chat('3','A','hidden'));r.model.show_chat=true;assert(not r.model:chat('3','A','hidden'));r:post_update();assert(#printed==1,'hide must not queue history for later replay')
 assert(r.model:chat('4','A','old scope'));r:close();r:post_update();assert(#printed==1 and r.notice_diagnostics.pending==0,'scope close clears pending notices')
 for i=1,33 do r.model:chat('bound-'..i,'A','bounded')end;assert(r.notice_diagnostics.pending==32,'notice queue bound');r:close(true);r:post_update();assert(r.notice_diagnostics.pending==0,'End clears notices')
 print('PASS actual GamePrint boundary, finite missing/throwing handling, dedup/hide/scope/End; engine callback facts injected')
else
 local output=assert(io.open(assert(os.getenv('NT2_A2_DRAW_PLAN')),'wb'))
 for sequence,hp in ipairs({0,1000,2000})do
  r:receive_info({actor='peer',revision=sequence,hp=hp,max_hp=2000,biome='$biome_coalmine'},now);r:tick(true,99)
  local full,empty;for _,image in ipairs(images)do if image.path:find('hpbar_full',1,true)then full=image elseif image.path:find('hpbar_empty',1,true)then empty=image end end
  assert(empty and empty.z==10 and empty.scale==1,'opaque HP background must use legacy depth10')
  if hp==0 then assert(not full,'zero HP must not draw a full fill')else assert(full and full.z==9 and full.scale==hp/2000,'partial/full HP fill must be in front at depth9')end
  local tooltip=table.concat(texts,'|');assert(tooltip:find('Location: Mines\nDepth: 10m\nStatus: playing',1,true),'legacy name-hover tooltip retains known location, depth and current roster status')
  output:write(hp,'|',full and full.scale or 0,'|',full and full.z or 9,'|',empty.scale,'|',empty.z,'\n')
 end
 output:close()
 now=3600;r:tick(true,99);local retained_bar=false
 for _,image in ipairs(images)do if image.path:find('hpbar_full',1,true)then retained_bar=image.scale==1 end end
 assert(retained_bar,'connected last-known full HP must remain visible through long gaps')
 r:receive_info({actor='peer',revision=4,biome=''},now);r:tick(true,99)
 assert(r.model.hud.peer.hp==2000,'missing HP retains known value')
 r:roster('test-run','peer',{{actor='peer',name='Peer',status=4,ghost=''}});r:tick(true,99);assert(table.concat(texts,'|'):find('Status: respawning',1,true),'C1 temporary respawn status must remain distinct from terminal/disconnected')
 r:roster('new-run','peer',{{actor='peer',name='Peer',status=1,ghost=''}});r:tick(true,99)
 for _,image in ipairs(images)do assert(not image.path:find('hpbar_',1,true),'never-known HP must not draw a health bar')end
 print('PASS production zero/partial/full HP draw plan; connected retention, never-known no bar; ASCII location line; engine draws injected')
end
