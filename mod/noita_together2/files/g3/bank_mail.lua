-- Only legacy fixed mail placement. Existing shop/perk/random-loot hooks remain.
local nt2_bank_spawn_hp=spawn_hp
RegisterSpawnFunction(0xffc9490a,'nt2_bank_inlet')
RegisterSpawnFunction(0xff63c408,'nt2_bank_outlet')
function nt2_bank_inlet(x,y)
 local path='mods/noita_together2/files/g3/item_inlet.xml'
 if ModDoesFileExist and ModDoesFileExist(path)then EntityLoad(path,x,y)end
end
function nt2_bank_outlet(x,y)EntityLoad('mods/noita_together2/files/g3/bank_outlet.xml',x,y)end
function spawn_hp(x,y)
 nt2_bank_spawn_hp(x,y)
 LoadPixelScene('mods/noita_together2/files/g3/bank_mail.png','',x-49,y,'',true)
end
