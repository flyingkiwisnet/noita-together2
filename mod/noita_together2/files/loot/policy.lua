-- Local loot inputs only. No terrain seed, authority, paths or scripts from the service.
local M={}
local bootstrap_path="mods/noita_together2/files/loot/run-policy.txt"
local key="nt2_seed_loot_policy_v1"
local function valid(recorded)
    if recorded=="" or recorded=="individual:0" then return true end
    local bit,digits
    if type(recorded)=="string" then bit,digits=recorded:match("^shared:([01]):(%d+)$") end
    local seed=tonumber(digits)
    return seed~=nil and seed>=0 and (bit=="0" or seed>0) and seed<=4294967295 and seed==math.floor(seed)
        and digits==string.format("%.0f",seed)
end
-- ModTextFileSetContent is valid during mod init, before WorldState exists.
-- This fixed inert file is shared by separately loaded Lua contexts. It is
-- local loot configuration, never enrollment authority or executable content.
function M.stage(recorded,current_entry)
    assert(valid(recorded),"Invalid local loot policy")
    local prefix=current_entry and "entry-new:" or "new:"
    ModTextFileSetContent(bootstrap_path,prefix..recorded)
    assert(ModTextFileGetContent(bootstrap_path)==prefix..recorded,"Local loot policy staging failed")
end
function M.saved()
    ModTextFileSetContent(bootstrap_path,"saved")
    assert(ModTextFileGetContent(bootstrap_path)=="saved","Local loot bootstrap reset failed")
end
-- Entry Continue reads loaded save data without mod-init-only bootstrap writes.
function M.read_saved()
    local ok,world=pcall(GameGetWorldStateEntity)
    if not ok or type(world)~="number" or world<=0 then return "" end
    local read,value=pcall(GlobalsGetValue,key,"")
    return read and valid(value) and value or ""
end
function M.initialize()
    local bootstrap=ModTextFileGetContent(bootstrap_path)
    if type(bootstrap)=="string" and bootstrap:sub(1,10)=="entry-new:" then
        -- Candidate policy is usable only while its one consumed entry registry
        -- is still current. A retained/script context cannot overwrite Continue.
        local ok,list=pcall(ModLuaFileGetAppends,"mods/noita_together2/files/startup/scope_registry.lua")
        if not ok or type(list)~="table" or #list~=2 or list[1]~="mods/noita_together2/files/startup/scope_new.lua" or list[2]~="mods/noita_together2/files/startup/scope_spent.lua" then return M.read_saved() end
        bootstrap="new:"..bootstrap:sub(11)
    end
    local recorded=type(bootstrap)=="string" and bootstrap:match("^new:(.*)$")
    if recorded~=nil and not valid(recorded) then return "" end
    if recorded==nil and bootstrap~="saved" and bootstrap~="saved\n" and bootstrap~="saved\r\n" then return "" end
    local ok,world=pcall(GameGetWorldStateEntity)
    -- Early generation can read the staged policy without touching Globals.
    if not ok or type(world)~="number" or world<=0 then return recorded or "" end
    local read,value=pcall(GlobalsGetValue,key,"")
    if not read then return "" end
    if recorded~=nil then
        if value~=recorded then
            local wrote=pcall(GlobalsSetValue,key,recorded)
            read,value=pcall(GlobalsGetValue,key,"")
            if not wrote or not read or value~=recorded then return "" end
        end
        return recorded
    end
    -- Continue/offline uses only this loaded world's saved configuration.
    return valid(value) and value or ""
end
function M.enabled()
    local recorded=M.initialize()
    local seed=type(recorded)=="string" and recorded:match("^shared:1:(%d+)$")
    seed=tonumber(seed)
    return seed~=nil and seed>0 and seed<=4294967295 and seed==math.floor(seed)
        and tostring(StatsGetValue("world_seed"))==string.format("%.0f",seed)
end
function M.deaths() return tonumber(StatsGlobalGetValue("death_count")) or 0 end
local function pack(...) return {n=select("#",...),...} end
function M.with_rng(fn,transform,...)
    local original=SetRandomSeed
    SetRandomSeed=function(x,y) local a,b=transform(x,y);return original(a,b) end
    local result=pack(pcall(fn,...))
    SetRandomSeed=original
    if not result[1] then error(result[2],0) end
    return unpack(result,2,result.n)
end
function M.frame_rng(fn,...)
    return M.with_rng(fn,function(x,y) return x+GameGetFrameNum(),y end,...)
end
function M.offset(max_x,max_y)
    SetRandomSeed(GameGetFrameNum(),69420+M.deaths())
    return Random(0,max_x),Random(0,max_y)
end
return M
