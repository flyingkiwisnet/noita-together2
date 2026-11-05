-- Canonical base content only; fixed file mapping supplied by the runtime profile.
local root='mods/noita_together2/files/'
local M={}
function M.load(e)
 local catalogue=dofile(root..'g3/c1_catalogue_data.lua')
 assert(catalogue.revision=='nt2-c1-base-1','Canonical C1 catalogue revision')
 local orbs={};for key,value in pairs(catalogue.orbs)do local id=assert(tonumber(key),'Canonical orb ID');assert(id==value.id and id==math.floor(id),'Canonical orb identity');orbs[id]=value end
 local content={perks=catalogue.perks,orbs=orbs,materials=catalogue.materials,fungal=dofile(root..'c1/scripts/fungal_receive.lua')}
 return content
end
return M
