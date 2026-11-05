local actor=dofile('mods/noita_together2/files/g3/game_api.lua')
local capture=dofile('mods/noita_together2/files/g3/a2_capture.lua')
local catalogue=dofile('mods/noita_together2/files/race/catalogue.lua')
local M={}
local function finite(n,max)return type(n)=='number'and n==n and n>=0 and n<=max end
local function decimal(n)return string.format('%.6f',n):gsub('0+$',''):gsub('%.$','')end
function M.facts(e,player,terminal)
 local out={};local id,state=actor.resolve_player(e,player)
 if id~=player or not state or state.transformed or state.dead and not terminal then return out end
 for _,c in ipairs(e.EntityGetComponentIncludingDisabled(id,'VariableStorageComponent')or{})do if e.ComponentGetValue2(c,'name')=='nt2_g3_recovery'then return out end end
 local maximum=state.damage and e.ComponentGetValue2(state.damage,'max_hp')
 if finite(maximum,2147483647/25)and maximum>0 then out[#out+1]={kind='max_hp',value=decimal(maximum*25)}end
 local wallet=e.EntityGetFirstComponentIncludingDisabled(id,'WalletComponent');local money=wallet and e.ComponentGetValue2(wallet,'money')
 if finite(money,2147483647)and money%1==0 then out[#out+1]={kind='gold',value=string.format('%.0f',money)}end
 return out
end
function M.locations(e,player,map_verified)
 local value=capture.sample(e,player);local out={};local biome=catalogue.biomes[value.biome]
 if biome then out[#out+1]={kind='biome',biomeId=biome}end
 if map_verified and value.x and value.y and type(e.GetParallelWorldPosition)=='function'and type(e.BiomeMapGetSize)=='function'then
  local width=e.BiomeMapGetSize()*512
  if width==35840 then local index=e.GetParallelWorldPosition(value.x,value.y);local expected=math.floor((value.x+width/2)/width);local mx=((value.x+width/2)%width)-width/2
   if type(index)=='number'and index==expected and index~=0 and math.abs(index)<=10 and mx>=-15872 and mx<15872 then out[#out+1]={kind='parallel',direction=index>0 and'east'or'west',index=math.abs(index)}end
  end
 end
 return out
end
return M
