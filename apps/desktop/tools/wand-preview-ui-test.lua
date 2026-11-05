local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(p)if p:find('item_catalogue_data.lua',1,true)then return raw(repo..'/packages/contracts/g3-items/catalogue.lua')end;return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local json=dofile(root..'bank_json.lua');local frame=1;local sent,tips={},{};local checks=0
local function check(v,s)assert(v,s);checks=checks+1 end
local e={GameGetFrameNum=function()return frame end,GlobalsGetValue=function(_,d)return d end,GuiTooltip=function(_,title,body)tips[#tips+1]={title=title,body=body}end,GuiZSet=function()end,GuiZSetForNextWidget=function()end,GuiButton=function()return false end}
local bridge=dofile(root..'item_bridge.lua').new(e,function()return '11111111-1111-4111-8111-111111111111'end,function(body)sent[#sent+1]=assert(json.decode(body));return true end,function()return {}end,function()return false end,function()end)
local actor='22222222-2222-4222-8222-222222222222';local session='33333333-3333-4333-8333-333333333333';local row={actor=actor,name='Remote'}
bridge.players={{actor=actor,session=session,revision=1}}
bridge:player_row(e,1,function()return 50 end,row,true,false)
check(#sent==1 and sent[1].kind=='inventory_inspect'and sent[1].actor==session and sent[1].slot==0,'valid right-click requests real game session even if separate hover flag is false')
check(tips[#tips]and tips[#tips].body:find('Loading',1,true),'right-click produces immediate loading feedback')
local chunks=dofile(root..'item_chunks.lua');local body=assert(json.encode({gameSessionId=session,state='empty',wands=json.array({})}));local sender=assert(chunks.sender('preview-one','preview',body));bridge:receive({chunk=sender:chunk()})
bridge:player_row(e,1,function()return 50 end,row,false,false)
check(tips[#tips].body=='No wands','selected row displays copied preview without depending on hover')
bridge:player_row(e,1,function()return 50 end,row,true,false);local before=#sent;bridge:player_row(e,1,function()return 50 end,row,false,true)
check(#sent==before and not bridge.preview_mode,'same row right-click toggles preview off')
bridge:player_row(e,1,function()return 50 end,{actor='44444444-4444-4444-8444-444444444444',name='Missing'},true,true)
check(#sent==before and tips[#tips].body:find('unavailable',1,true),'missing authenticated inventory mapping does not fabricate target or request')
print('PASS wand preview '..checks..' assertions; production item bridge/read request, injected clicks/roster/preview, no game/service')
