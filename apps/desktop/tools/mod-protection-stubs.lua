-- Engine stubs only: original LuaJIT runs this, but it is not an actual Noita test.
NT2_FIXTURE={frame=0,poly=false,effect=false,button=false,components={[1]={enabled=true},[2]={wait_for_kill_flag_on_death=false},[4]={frames=1}},enabled={[5]=true,[6]=true,[7]=false}}
local s=NT2_FIXTURE
function dofile_once() end
function ModMagicNumbersFileAdd() end
function GameGetFrameNum() return s.frame end
function SetWorldSeed(v) assert(v==424242) end
function EntityGetIsAlive(id) return id==99 or id==999 and s.effect end
function EntityGetFirstComponentIncludingDisabled(id,kind)
    if id==999 then return kind=='GameEffectComponent' and 4 end
    return kind=='ControlsComponent' and 1 or kind=='DamageModelComponent' and 2 or kind=='InventoryGuiComponent' and 5 or kind=='Inventory2Component' and 6 or kind=='ItemPickUpperComponent' and 7
end
function EntityGetComponentIncludingDisabled(_,kind) if kind=='VariableStorageComponent' and s.components[3] then return {3} end return {} end
function ComponentGetIsEnabled(id) return s.enabled[id] end
function EntitySetComponentIsEnabled(_,id,value) s.enabled[id]=value end
function ComponentGetValue2(id,key) return s.components[id][key] end
function ComponentSetValue2(id,key,value) s.components[id][key]=value end
function EntityAddComponent2(_,_,values) s.components[3]=values;return 3 end
function EntityRemoveComponent(_,id) s.components[id]=nil end
function EntityGetTransform() return 227,-85 end
function EntitySetTransform() end
function EntityHasTag(id) return id==99 and s.poly or id==999 and s.effect end
function EntityAddTag(id) if id==99 then s.poly=true end end
function EntityRemoveTag(id) if id==99 then s.poly=false end end
function LoadGameEffectEntityTo() s.effect=true;return 999 end
function EntityKill() s.effect=false end
function EntityGetAllChildren() return s.effect and {999} or {} end
function GuiCreate() return 1 end
function GuiStartFrame() end
function GuiText() end
function GuiButton() return s.button end
