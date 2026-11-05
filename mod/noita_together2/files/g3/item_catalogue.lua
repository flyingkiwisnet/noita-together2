-- The data file is mapped byte-for-byte from the server-owned generated
-- packages/contracts/g3-items/catalogue.lua in the matching runtime closure.
local fixed=dofile('mods/noita_together2/files/g3/item_catalogue_data.lua')
local result={revision=fixed.revision,sha256=fixed.sha256,spells=fixed.actions,materials=fixed.materials,sprites=fixed.sprites,objects=fixed.objects,sprite_ids={},object_ids={}}
for id,entry in pairs(result.sprites)do result.sprite_ids[entry.path]=id end
for id,entry in pairs(result.objects)do result.object_ids[entry.path]=id end
return result
