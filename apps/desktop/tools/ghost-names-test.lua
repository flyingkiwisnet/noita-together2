local root=assert(os.getenv('NT2_G3_MODULES'));local raw=dofile
dofile=function(path)local relative=path:match('^mods/noita_together2/(.+)$');return raw(relative and root:gsub('/files/g3$','')..'/'..relative or path)end
local function load(name)return dofile(root..'/'..name..'.lua')end
local checks=0;local function check(value,message)assert(value,message);checks=checks+1 end
local entities,components,next_id={}, {},0;local seconds=0
local function entity(name)next_id=next_id+1;entities[next_id]={alive=true,name=name,children={},components={}};return next_id end
local function add(id,kind,values)next_id=next_id+1;components[next_id]={kind=kind,values=values};entities[id].components[#entities[id].components+1]=next_id;return next_id end
local e={}
function e.EntityCreateNew(name)return entity(name)end
function e.EntityGetIsAlive(id)return entities[id]and entities[id].alive or false end
function e.EntitySetName(id,name)entities[id].name=name end
function e.EntityAddComponent(id,kind,values)return add(id,kind,values)end
function e.EntityAddChild(parent,child)entities[parent].children[#entities[parent].children+1]=child end
function e.EntityGetAllChildren(parent)return entities[parent]and entities[parent].children or{}end
function e.EntityGetComponentIncludingDisabled(id,kind)local found={};for _,c in ipairs(entities[id].components)do if components[c].kind==kind then found[#found+1]=c end end;return found end
function e.ComponentSetValue2(id,key,value)components[id].values[key]=value end
function e.EntityKill(id)if not entities[id]then return end;entities[id].alive=false;for _,child in ipairs(entities[id].children)do e.EntityKill(child)end end
function e.EntitySetTransform(id,x,y,rotation,sx,sy)entities[id].transform={x=x,y=y,rotation=rotation,sx=sx,sy=sy}end
function e.EntityLoad(path,x,y)local id=entity('ghost');add(id,'SpriteComponent',{image_file='body.xml'});local cape=entity('cape');add(cape,'SpriteComponent',{image_file='cape.xml'});e.EntityAddChild(id,cape);return id end
e.GameGetRealWorldTimeSinceStarted=function()return seconds end
for name,fn in pairs(e)do _G[name]=fn end
local labels=load('ghost_labels');local api=load('game_api').ghosts();local validate=load('validation');local ghosts=load('ghosts').new(api,validate)
local scope,actor='owned-context',string.rep('a',32);check(ghosts:bind(scope,{actor}),'bind authenticated movement roster')
local function pose(sequence,facing)return{actor=actor,sequence=sequence,x=10,y=20,rotation=0,facing=facing or 1,vx=0,vy=0}end
local function tick(sequence,facing)seconds=seconds+.05;check(ghosts:receive(scope,pose(sequence,facing),sequence),'current actor pose');ghosts:tick(sequence,true);return ghosts.ghosts[actor].entity end
local function child(parent)for _,id in ipairs(entities[parent].children)do if entities[id].alive and entities[id].name=='player_name'then return id end end end
local function sprite(id)return components[e.EntityGetComponentIncludingDisabled(id,'SpriteComponent')[1]].values end
local first=tick(1);local label=child(first);check(label~=nil,'missing name has a safe nameplate');check(sprite(label).text=='Player','missing-name fallback does not expose actor UUID')
check(ghosts:set_names(scope,{[actor]='Älex 玩家.[x]'}),'authenticated name update');tick(2,-1);check(child(first)==label,'late name reuses existing child');check(sprite(label).text=='Älex 玩家.[x]','Unicode/punctuation literal name')
seconds=seconds+.2;ghosts:tick(3,true);check(entities[first].transform.sx==-1,'body facing preserved');local inherit=components[e.EntityGetComponentIncludingDisabled(label,'InheritTransformComponent')[1]].values;check(inherit.only_position=='1'and inherit.use_root_parent=='1','name inherits position only, not facing or body rotation')
check(sprite(label).image_file=='data/fonts/font_pixel_white.xml'and sprite(label).special_scale_x=='0.7'and sprite(label).offset_y=='-6','legacy fixed font, scale and offset');check(sprite(entities[first].children[1]).image_file=='cape.xml','cape child is untouched')
local long=string.rep('界',60);local shown,n=labels.display(long);check(load('a2_text').length(shown)==35 and n==35,'long Unicode truncated at codepoint boundary');check(shown:sub(-3)=='...','long name signals truncation');check(labels.display('\255broken')=='Player','malformed UTF8 fallback');check(labels.display('bad\nname')=='Player','control characters refused');check(labels.display('   ')=='Player','blank name fallback')
check(ghosts:set_names(scope,{[actor]=long}),'long roster name accepted within bound');tick(3);check(sprite(label).text==shown,'display truncation used in actual adapter');check(tonumber(sprite(label).offset_x)==35*1.9,'centering uses characters, not UTF8 byte length')
ghosts:set_visibility(true,.4);check(not e.EntityGetIsAlive(first)and not e.EntityGetIsAlive(label),'hiding removes body and child together');ghosts:set_visibility(false,.4);local second=tick(4);check(second~=first,'unhide respawns ghost');check(sprite(child(second)).text==shown,'name reapplied after respawn');check(sprite(child(second)).alpha==.4,'opacity follows ghost');check(sprite(second).alpha==.4,'body opacity retained')
check(not ghosts:set_names('previous-context',{[actor]='Wrong'}),'old context cannot rename current ghost');check(not ghosts:receive(scope,{actor='intruder',sequence=1,x=0,y=0,rotation=0,facing=1,vx=0,vy=0},5),'name mapping cannot authorize movement actor')
check(ghosts:reconcile(scope,{}),'roster removal accepted');check(not e.EntityGetIsAlive(second),'departed ghost and label removed');check(ghosts:reconcile(scope,{actor}),'current actor returns');local third=tick(5);e.EntityKill(third);ghosts:tick(6,true);local fourth=tick(6);check(fourth~=third and child(fourth)~=nil,'externally removed body/label recovers on a new pose')
ghosts:clear();check(ghosts:bind('new-context',{actor}),'new lobby binds');scope='new-context';local fifth=tick(1);check(sprite(child(fifth)).text=='Player','previous lobby name does not leak')
local captured;e.a2_ghost_names=function(names)captured=names;ghosts:set_names(scope,names)end;e.GlobalsSetValue=function()end;e.GameGetRealWorldTimeSinceStarted=function()return seconds end
local renderer=load('a2_renderer').new(e,validate);local local_actor=string.rep('b',32);renderer:roster('run',local_actor,{{actor=local_actor,ghost=local_actor,name='Self',status='playing'},{actor='remote-social-actor',ghost=actor,name='Canonical name',status='playing'}});check(captured[actor]=='Canonical name'and captured[local_actor]==nil,'canonical social actor maps to movement ghost; local name excluded');tick(2);check(sprite(child(fifth)).text=='Canonical name','current authenticated canonical name reaches ghost')
renderer:roster('run',local_actor,{{actor='remote-social-actor',ghost=actor,name='Renamed',status='playing'}});tick(3);check(sprite(child(fifth)).text=='Renamed','canonical late rename updates in place');renderer:close(true);check(next(captured)==nil,'terminal renderer clears labels map');ghosts:clear();check(not e.EntityGetIsAlive(fifth),'terminal movement cleanup owns label lifetime')
print('PASS ghost names: '..checks..' checks; synthetic engine components, actual Noita positioning remains owner smoke')
