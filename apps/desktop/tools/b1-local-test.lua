-- Synthetic engine observations; production bank UI/guard/mail code, no save proof.
local repo=assert(os.getenv('NT2_A3_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/'
local checks=0;local function check(v,label)assert(v,label);checks=checks+1 end
local validate=dofile(root..'validation.lua')
local json=dofile(root..'bank_json.lua')
local decoded=json.decode('{"a":[],"b":true,"c":false,"n":123,"text":"Māori \\u0041"}')
check(decoded and decoded.b==true and decoded.c==false and decoded.n==123 and decoded.text=='Māori A','bounded JSON decoded')
for _,s in ipairs({'{} trailing','{"x":1,"x":2}','{"x":1.5}','{"x":"\\uD800"}',string.rep('[',14)..string.rep(']',14),string.rep(' ',8193)})do check(json.decode(s)==nil,'malformed mailbox rejected')end
local sent={};local serial=0;local function identity()serial=serial+1;return string.format('00000000-0000-4000-8000-%012d',serial)end
local ui=dofile(root..'bank_ui.lua').new(function(s)sent[#sent+1]=s;return true end,identity)
ui:receive({fresh=true,enabled=true,balance=100})
for _,amount in ipairs({'','0','-1','1.5','1000001','4294967296'})do ui.amount=amount;check(not ui:submit('deposit',2000000),'invalid amount refused visibly');check(ui.amount==amount and #sent==0,'invalid never clamps/sends')end
ui.amount='5';check(not ui:submit('deposit',4),'insufficient wallet has no request');check(ui:submit('withdraw',nil),'valid TAKE requested')
local id=ui.pending;check(id~=nil and #sent==1,'one click one original');check(not ui:submit('withdraw',100),'pending duplicate blocked');check(ui.amount=='5','pending retains amount')
ui:receive({submission={id=id,status='unresolved'}});check(ui.pending==id and ui.amount=='5','uncertainty retains original/input')
ui:receive({submission={id=id,status='accepted'}});check(ui.pending==nil and ui.amount=='1','only accepted resets input')
ui:receive({fresh=true,enabled=true,balance=100});ui.amount='7';check(ui:submit('deposit',10),'deposit request');id=ui.pending
ui:receive({submission={id=id,status='refused'},status='Not enough gold.'});check(ui.pending==nil and ui.amount=='7','refusal retains input and permits deliberate next action')
local enqueue_busy=true;local attempted={}
local busy_ui=dofile(root..'bank_ui.lua').new(function(body)attempted[#attempted+1]=json.decode(body);return not enqueue_busy end,identity)
busy_ui:receive({fresh=true,enabled=true,balance=100});busy_ui.amount='7'
check(not busy_ui:submit('deposit',100),'false enqueue refuses the click visibly');check(busy_ui.pending==nil and busy_ui.amount=='7'and busy_ui.status:find('busy',1,true),'false enqueue never invents a pending operation or clears the amount')
enqueue_busy=false;check(busy_ui:submit('deposit',100),'later deliberate retry enqueues successfully');check(busy_ui.pending==attempted[2].id and busy_ui.amount=='7','only accepted enqueue owns the UI pending identity')
busy_ui:receive({submission={id=attempted[1].id,status='accepted'}});check(busy_ui.pending==attempted[2].id and busy_ui.amount=='7','rejected enqueue cannot acknowledge a later deliberate retry')
local balance,writes=100,0;local actor,wallet=99,999;local fail=false
local api={snapshot=function()return {player=actor,wallet=wallet,balance=balance}end,commit=function(before,after)if fail or before.player~=actor or before.wallet~=wallet or before.balance~=balance then return false end;balance=after;writes=writes+1;return true end}
local guard=dofile(root..'gold_local.lua').new(api,{},validate);check(guard:bind('world',{}),'guard bind')
local function op(id,direction,amount)return {id=id,digest=string.rep('a',64),scope='world',direction=direction,amount=amount}end
local p=op('deposit-1','deposit',20);local prepared=guard:offer(p,'attempt');check(prepared and balance==100 and writes==0,'proposal does not mutate before flush')
p.attempt='wrong';check(guard:permit(p)==nil and writes==0,'wrong permission inert');p.attempt='attempt';check(guard:permit(p).state=='applied_session'and balance==80 and writes==1,'one permitted debit');check(guard:permit(p)==nil and writes==1,'repeated permission never reapplies')
local q=op('take-1','withdraw',25);check(guard:offer(q,'take'),'queued proposal');actor=199;q.attempt='take';check(guard:permit(q)==nil and writes==1,'actor turnover defers without writing unrelated wallet')
local f=op('fail-1','deposit',10);check(guard:offer(f,'fail'),'write-failure proposal');fail=true;f.attempt='fail';check(guard:permit(f).state=='uncertain'and writes==1,'false engine write is uncertain');fail=false
balance=2147483647;check(guard:offer(op('overflow','withdraw',1),'overflow')==nil,'overflow defers without receipt');balance=0;check(guard:offer(op('short','deposit',1),'short')==nil,'insufficient wallet no effect')
-- Existing synchronous guard must also respect explicit false from an engine write.
local sync=dofile(root..'gold_local.lua').new({balance=function()return 10 end,set_balance=function()return false end},{write=function()return true end},validate);sync:bind('world',{});check(sync:apply(op('sync','deposit',1),true)=='uncertain','false synchronous wallet write never accepted')
-- Execute only placement glue; original spawn and unrelated hooks remain intact.
local original=0;local scene;local spawns={};local unrelated=function()return 'retained'end;spawn_all_shopitems=unrelated;spawn_all_perks=unrelated
spawn_hp=function(x,y)original=original+1 end;RegisterSpawnFunction=function(color,name)spawns[color]=name end
LoadPixelScene=function(path,visual,x,y)scene={path,x,y}end;EntityLoad=function()return 1 end
dofile(root..'bank_mail.lua');spawn_hp(100,200);check(original==1 and scene[2]==51 and scene[3]==200,'legacy left-wing offset and original hp hook');check(spawn_all_shopitems==unrelated and spawn_all_perks==unrelated,'random-loot hooks untouched');check(_G[spawns[0xffc9490a]](0,0)==nil,'inlet inert')
-- Syntax-check every new/changed production Lua source without executing game hooks.
for _,file in ipairs({'gold_wallet.lua','bank_native.lua','bank_ui.lua','bank_glow.lua','bank_mail.lua','bank_json.lua','gold_local.lua','a2_renderer.lua','social_native.lua','social_local.lua'})do check(loadfile(repo..'/mod/noita_together2/files/g3/'..file)~=nil,'Lua51 syntax '..file)end
-- Run the actual copy-only adapter against injected current actor/wallet/outlet
-- APIs. Native/network/durability are separate real-component composition work.
local ffi=require('ffi');local bank_frame,player_x,outlet_x=0,0,100
local native_body='';local copied={};local busy=false;local missing,poly=false,false
local money,mutations,poofs=100,0,0;local false_write=false;local globals={};local print_notices={}
local e={GameGetFrameNum=function()return bank_frame end,EntityGetIsAlive=function(id)return id==99 or id==900 end,
 EntityGetWithTag=function(tag)return tag=='nt2_b1_outlet'and {900}or tag=='player_unit'and {99}or {}end,
 EntityGetRootEntity=function(id)return id end,EntityHasTag=function(_,tag)return poly and tag=='polymorphed'end,
 EntityGetComponentIncludingDisabled=function()return {}end,EntityGetTransform=function(id)return id==900 and outlet_x or player_x,0 end,
 EntityGetFirstComponentIncludingDisabled=function(id,kind)if id~=99 then return nil end;return ({GameStatsComponent=1,ControlsComponent=2,DamageModelComponent=3,WalletComponent=not missing and 4 or nil})[kind]end,
 IsPlayer=function(id)return id==99 end,ComponentGetValue2=function(c,key)if c==1 and key=='is_player'then return true elseif c==3 and key=='hp'then return 1 elseif c==4 and key=='money'then return money end end,
 ComponentSetValue2=function(c,key,value)check(c==4 and key=='money','only current wallet money may mutate');if false_write then return false end;money=value;mutations=mutations+1 end,
 EntityLoad=function(path)check(path=='data/entities/particles/poof_pink.xml','only successful gold pink particle');poofs=poofs+1;error('injected cosmetic failure after feedback attempt')end,
 GlobalsSetValue=function(k,v)globals[k]=v end}
local dll={nt2_g3_bank_view=function(_,buffer,size)check(size==8192,'bank bounded copy view');ffi.copy(buffer,native_body,#native_body);return #native_body end,
 nt2_g3_bank_publish=function(_,buffer,size)if busy then return 0 end;copied[#copied+1]=ffi.string(buffer,size);return 1 end}
local renderer={bank_notice=function(_,s)print_notices[#print_notices+1]=s end}
local bank=dofile(root..'bank_native.lua').new(dll,ffi.new('uint8_t[16]'),ffi,e,validate,renderer,function()serial=serial+1;return string.format('%032x',serial)end)
-- Pause local bank presentation before Globals become readable on turnover.
do
 local setter=e.GlobalsSetValue;local world=e.GameGetWorldStateEntity
 globals.nt2_b1_active='old-bank';globals.nt2_items_active='old-items'
 bank.playing=true;bank.near=true;bank.collecting=true;bank.ui.open=true
 e.GameGetWorldStateEntity=function()return 0 end;e.GlobalsSetValue=function()error('No bank Globals while WorldState is absent')end
 bank:pause();check(not bank.playing and not bank.near and not bank.collecting and not bank.ui.open,'absent-world pause clears local interaction')
 check(globals.nt2_b1_active=='old-bank'and globals.nt2_items_active=='old-items','absent-world pause preserves unreadable persisted values')
 e.GameGetWorldStateEntity=function()error('WorldState unavailable')end;bank:pause()
 e.GlobalsSetValue=setter;e.GameGetWorldStateEntity=world;bank:pause();check(globals.nt2_b1_active=='0'and globals.nt2_items_active=='0','readable-world pause keeps normal persisted cleanup')
end
local function output(stage,id,direction,amount,attempt)
 native_body='{"run":"world-native","balance":100,"version":"1","fresh":true,"enabled":true,"queued":5,"notices":[],"effect":{"id":"'..id..'","digest":"'..string.rep('a',64)..'","direction":"'..direction..'","amount":'..amount..',"stage":"'..stage..'"'..(attempt and ',"attempt":"'..attempt..'"'or '')..'}}'
end
local id='00000000-0000-4000-8000-000000000001'
output('offer',id,'withdraw',5);bank_frame=29;bank:tick(true,99);check(#copied==0 and mutations==0,'outlet proposal only every30frames')
outlet_x=24;bank_frame=30;bank:tick(true,99);check(#copied==0,'Manhattan24 excluded')
outlet_x=23;bank_frame=60;bank:tick(true,99);check(#copied==1 and mutations==0,'Manhattan23 proposes but cannot mutate before native permit')
local prepared=json.decode(copied[1]);check(prepared.kind=='prepare'and prepared.id==id,'original identity/digest carried')
bank.pending=copied[1];bank.ui.amount='7';check(bank.ui:submit('deposit',money)and bank.ui.pending~=nil and bank.ui.amount=='7','occupied Prepare queues one foreground original without clearing its input');check(bank.pending==copied[1]and json.decode(bank.interactive).id==bank.ui.pending,'foreground queue retains exact ID and leaves Prepare untouched');check(not bank.ui:submit('deposit',money),'second click cannot create another queued original');bank.pending=nil;bank.interactive=nil;bank.ui.pending=nil
output('permit',id,'withdraw',5,prepared.attempt);missing=true;bank_frame=90;bank:tick(true,99);check(mutations==0 and bank.guard.receipts[id].state=='prepared','missing wallet after permission still defers queue')
missing=false;poly=true;bank_frame=120;bank:tick(true,99);check(mutations==0,'polymorph after permission still defers')
poly=false;money=2147483647;bank_frame=150;bank:tick(true,99);check(mutations==0,'overflow after permission defers')
money=100;outlet_x=24;bank_frame=180;bank:tick(true,99);check(mutations==0,'leaving outlet before effect defers')
outlet_x=23;busy=true;bank.pending=copied[1];bank_frame=210;bank:tick(true,99);check(mutations==1 and money==105 and poofs==1,'one credit, pink failure does not lose result')
check(bank.pending and bank.pending:find('"kind":"result"',1,true),'already-applied result takes priority over unsent duplicate prepare')
local wallet_result=bank.pending;bank.ui.amount='7';check(bank.ui:submit('deposit',money),'one user original can queue behind a wallet result');check(bank.ui.pending~=nil and bank.ui.amount=='7'and bank.pending==wallet_result,'queued user keeps ID/amount without displacing a wallet result')
busy=false;bank_frame=211;bank:tick(true,99);local result=json.decode(copied[#copied]);check(result.kind=='result'and result.applied==true,'result eventually copied behind native backpressure')
local queued_submission=bank.ui.pending;check(queued_submission~=nil and bank.ui.amount=='7','queued foreground retains amount until actual acceptance');check(json.decode(bank.interactive).id==queued_submission,'queued click still has its initial original ID')
busy=true;bank_frame=212;bank:tick(true,99);check(json.decode(copied[#copied]).kind=='result'and bank.ui.pending==queued_submission,'new submission waits while native wallet-result ACK is outstanding')
busy=false;bank_frame=213;bank:tick(true,99);check(json.decode(copied[#copied]).kind=='submit'and json.decode(copied[#copied]).id==queued_submission,'wallet result reaches native before later deliberate submission');check(mutations==1,'queue admission never mutates a wallet')
bank.ui:receive({submission={id=queued_submission,status='accepted'}});check(bank.ui.pending==nil and bank.ui.amount=='1','later accepted submission clears only its own pending UI')
bank_frame=240;bank:tick(true,99);check(mutations==1 and poofs==1,'repeated proximity/permission never double credits')
bank:tick(false,99);check(globals.nt2_b1_active=='0','protection clears outlet eligibility')
-- Execute the production glow callback with boundary observations.
local glow,hint;local glow_env={GetUpdatedEntityID=function()return 900 end,GlobalsGetValue=function(k,d)return ({nt2_b1_player='99',nt2_b1_queue='5',nt2_b1_active='1'})[k]or d end,
 EntityGetIsAlive=function()return true end,EntityGetTransform=function(id)return id==900 and outlet_x or 0,0 end,
 EntityGetComponentIncludingDisabled=function(_,kind)return kind=='SpriteComponent'and {1}or {2}end,
 EntitySetComponentIsEnabled=function(_,_,value)hint=value end,ComponentSetValue2=function(_,key,value)check(key=='is_emitting','only cosmetic emitter property');glow=value end,ipairs=ipairs,tonumber=tonumber,math=math}
local glow_callback=assert(loadfile(repo..'/mod/noita_together2/files/g3/bank_glow.lua'));setfenv(glow_callback,glow_env)
outlet_x=149;glow_callback();check(hint and glow,'hint under150 and gold keeps glow active');outlet_x=150;glow_callback();check(not hint and glow,'150 excluded but queued gold glow stable')
glow_env.ModSettingGet=function()return false end;outlet_x=0;glow_callback();check(not hint and glow,'hints preference does not extinguish queued glow')
-- The real fixture resolves production dofile calls only inside its copied
-- runtime. A checkout copy of gold_local.lua must never rescue an omitted map.
local isolated=assert(os.getenv('NT2_B1_MISSING_ROOT'));local source=assert(io.open(repo..'/mod/noita_together2/files/g3/gold_local.lua','rb'));source:close()
local normal_dofile=dofile;local requests={}
dofile=function(path)check(path:match('^mods/noita_together2/')~=nil,'isolated runtime accepts only production mod paths');local target=isolated..'/'..path;requests[#requests+1]=target;return raw(target)end
local isolated_bank=dofile(root..'bank_native.lua');local loaded,reason=pcall(isolated_bank.new,dll,ffi.new('uint8_t[16]'),ffi,e,validate,renderer,identity)
check(not loaded and tostring(reason):find('gold_local.lua',1,true)and tostring(reason):find(isolated,1,true),'missing isolated gold_local fails instead of reading the checkout: '..tostring(reason))
check(requests[#requests]==isolated..'/'..root..'gold_local.lua','production wallet guard lookup stays inside the isolated runtime');dofile=normal_dofile
-- The actual composition observer must separate a proven live typing lease
-- from native permission loss, and must never erase a previous genuine hold.
local observer=raw(repo..'/apps/desktop/tools/lifecycle-lc1-permission-observer.lua').new()
local observed_frame=0
local function observe(controls,owned,native,protection,stale)
 observed_frame=observed_frame+1
 return observer:sample({frame=observed_frame,controls=controls,lease_owned=owned,native_status=native,native_status_frame=observed_frame-(stale and 1 or 0),protected=protection},true)
end
check(observe(true,false,2,false).permitted,'initial healthy peer observed')
local typing=observe(false,true,2,false);check(typing.permitted and typing.typing and typing.holds==0 and typing.input_locks==1,'actual owned typing lock with fresh native authority is not a permission hold')
check(observe(false,true,2,false).holds==0,'continued bank typing never churns/reset hold history')
local lost=observe(false,true,0,true);check(lost.lost and not lost.permitted and lost.holds==1,'genuine permission/protection loss during typing is still counted')
check(observe(true,false,2,false).holds==1,'return to play retains genuine hold count')
check(observe(false,true,2,true).lost,'protection marker cannot be masked by a stale UI ownership claim')
observe(true,false,2,false);check(observe(false,false,2,false).lost,'orphan/unowned disabled Controls is never excused as typing')
observe(true,false,2,false);check(observe(false,true,2,false,true).lost,'old native status cannot establish a current owned typing lease')
check(observe(true,false,2,false).holds==4,'all genuine holds survive subsequent healthy samples')
check(loadfile(repo..'/apps/desktop/tools/lifecycle-lc1-native.lua')~=nil,'composed fixture syntax after observer integration')
-- The real copy-only adapter must forget readiness across protected metadata
-- gaps, while ordinary alternating bank-only frames keep an initialized C1 view.
bank.last=nil;bank.c1={ready=true,tick=function()end,receive=function(self,v)self.ready=v.initialized==true;return true end}
native_body='';bank:tick(false,99);check(not bank.c1.ready,'protected metadata gap invalidates cached C1 policy')
native_body='{"c1":{"initialized":true}}';bank:tick(false,99);check(bank.c1.ready,'current protected policy copy restores C1 readiness')
native_body='';bank:tick(true,99);check(bank.c1.ready,'ordinary alternating footer cannot invalidate current policy')
bank:tick(false,99);check(not bank.c1.ready,'later authority gap again requires a current copy')
print('B1_LOCAL_CHECKS '..checks)
